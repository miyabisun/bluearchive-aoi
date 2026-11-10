//! A list's entries and how its cells are told apart. The item list and the
//! equipment list share this; they differ only in their entries and `Rules`
//! (`items.rs`, `equipment.rs`).

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
};

use image::{RgbImage, RgbaImage};

use crate::{
    Cell, Item, Row, grid,
    icon::{IconMatcher, Mask, Rarity},
    order,
    quantity::{Count, Digits},
};

/// Where an entry sits in its list's default order.
pub type ListKey = (usize, i64);

/// What a list decides for itself.
pub struct Rules {
    /// Whether the user keeps track of an entry (others are read for the list
    /// order but never shown).
    pub tracked: fn(&Item) -> bool,
    /// The entry's place in the default order.
    pub key: fn(&Item) -> ListKey,
    /// The entry's place in the window.
    pub shown: fn(&Item) -> ListKey,
    /// The icon part that other drawings of the list's cards cover.
    pub mask: Mask,
}

pub struct Catalog {
    entries: Vec<Item>,
    rules: Rules,
    /// List positions of the entries drawn with each icon, ascending; the
    /// matcher's icon indices point here.
    icon_keys: Vec<Vec<ListKey>>,
    /// Whether every entry drawn with each icon is untracked.
    icon_ignored: Vec<bool>,
    matcher: IconMatcher,
    /// The cards told apart lately, by their look (and the icon left out), as they were told.
    told: Mutex<Told>,
    /// How many cards were told apart, not taken as told before (see `cards_told_apart`).
    told_apart: AtomicUsize,
    /// How many cards were read, not taken as read before (see `cards_read`).
    read_anew: AtomicUsize,
    /// The cards read lately, as they were read.
    reads: Mutex<Reads>,
}

/// A card as it was read: its icon as told, the first row under the badge it wears, its count.
#[derive(Clone)]
struct ReadCard {
    told: Result<usize, Vec<usize>>,
    badge: Option<u32>,
    quantity: Option<Count>,
}

/// The cards read lately by what they were read from (see `drawn`), the oldest first out.
#[derive(Default)]
struct Reads {
    by_drawn: HashMap<u64, ReadCard>,
    order: VecDeque<u64>,
}

impl Reads {
    fn keep(&mut self, drawn: u64, read: ReadCard) {
        if self.by_drawn.insert(drawn, read).is_none() {
            self.order.push_back(drawn);
            if self.order.len() > TOLD_KEPT
                && let Some(oldest) = self.order.pop_front()
            {
                self.by_drawn.remove(&oldest);
            }
        }
    }
}

/// How far around a card its reading looks, as shares of its height: across, where its
/// slanted foot and the count on it reach (0.1 of its height at the foot, and a pixel of the
/// count's blend); up and down, where a badge hangs over its top (0.1).
const AROUND: (f32, f32) = (0.25, 0.15);

/// All a card's reading goes by, as one number: where the card is, the icon left out, and
/// every pixel around it the reading looks at (`AROUND`). Two cards of one number are read
/// alike (but for a chance of one in 2^64).
fn drawn(image: &RgbImage, card: &grid::Card, missing: Option<usize>) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u32(card.left.to_bits());
    hasher.write_u32(card.width.to_bits());
    hasher.write_u32(card.top);
    hasher.write_u32(card.height);
    hasher.write_u64(missing.map_or(u64::MAX, |m| m as u64));
    let height = card.height as f32;
    let (across, down) = (AROUND.0 * height, AROUND.1 * height);
    let x0 = (card.left - across).max(0.0) as usize;
    let x1 = ((card.left + card.width + across).ceil() as usize).min(image.width() as usize);
    let y0 = (card.top as f32 - down).max(0.0) as usize;
    let y1 = ((card.top as f32 + height + down).ceil() as usize).min(image.height() as usize);
    let (raw, stride) = (image.as_raw(), image.width() as usize * 3);
    for y in y0..y1 {
        hasher.write(&raw[y * stride + x0 * 3..y * stride + x1 * 3]);
    }
    hasher.finish()
}

/// How many cards' tellings are kept: a list shows 20 to 25 cards, so the last few frames'.
const TOLD_KEPT: usize = 128;

/// The cards told apart lately, the oldest first out.
#[derive(Default)]
struct Told {
    by_look: HashMap<Vec<u8>, Result<usize, Vec<usize>>>,
    order: VecDeque<Vec<u8>>,
}

impl Told {
    fn keep(&mut self, look: Vec<u8>, told: Result<usize, Vec<usize>>) {
        if self.by_look.insert(look.clone(), told).is_none() {
            self.order.push_back(look);
            if self.order.len() > TOLD_KEPT
                && let Some(oldest) = self.order.pop_front()
            {
                self.by_look.remove(&oldest);
            }
        }
    }
}

impl Catalog {
    /// The catalog of `entries`, whose icons `picture` gives by name.
    pub fn new(
        entries: Vec<Item>,
        rules: Rules,
        picture: impl Fn(&str) -> Result<RgbaImage, String>,
    ) -> Result<Self, String> {
        let mut icons: Vec<&str> = entries.iter().map(|e| e.icon.as_str()).collect();
        icons.sort_unstable();
        icons.dedup();
        let images = icons
            .iter()
            .map(|name| picture(name))
            .collect::<Result<Vec<_>, _>>()?;
        let icon_ignored = icons
            .iter()
            .map(|name| {
                entries
                    .iter()
                    .filter(|e| e.icon == *name)
                    .all(|e| !(rules.tracked)(e))
            })
            .collect();
        let icon_keys = icons
            .iter()
            .map(|name| {
                let mut keys: Vec<ListKey> = entries
                    .iter()
                    .filter(|e| e.icon == *name)
                    .map(rules.key)
                    .collect();
                keys.sort_unstable();
                keys
            })
            .collect();
        Ok(Self {
            matcher: IconMatcher::new(&images, rarities(&entries, &icons), rules.mask),
            icon_keys,
            icon_ignored,
            entries,
            rules,
            told: Mutex::new(Told::default()),
            told_apart: AtomicUsize::new(0),
            read_anew: AtomicUsize::new(0),
            reads: Mutex::new(Reads::default()),
        })
    }

    /// How many cards this catalog has told apart by comparing their icons since it was made. A
    /// card that looks exactly as one told apart lately (its icon's samples and the rows a badge
    /// leaves, the same icon left out) is taken as that one was told, and not counted: the frames
    /// of a list held still cost little, and read the same (the user's of 2026-11-07
    /// 「一覧の読み取りの改善」).
    pub fn cards_told_apart(&self) -> usize {
        self.told_apart.load(Ordering::Relaxed)
    }

    /// How many cards this catalog has read since it was made: a card drawn exactly as one
    /// read lately, where that one was, is taken as that one was read, and not counted.
    pub fn cards_read(&self) -> usize {
        self.read_anew.load(Ordering::Relaxed)
    }

    /// The icon `card` shows (see `IconMatcher::identify`), or as a card of the same look was told.
    fn tell(
        &self,
        image: &RgbImage,
        card: &grid::Card,
        badge: Option<u32>,
        candidates: &[usize],
        missing: Option<usize>,
    ) -> Result<usize, Vec<usize>> {
        let look = IconMatcher::look(image, card, badge);
        let mut key = look.bytes();
        key.extend_from_slice(&missing.map_or(u64::MAX, |m| m as u64).to_le_bytes());
        let earlier = self
            .told
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .by_look
            .get(&key)
            .cloned();
        if let Some(found) = earlier {
            return found;
        }
        let found = self.matcher.identify_look(&look, candidates);
        self.told_apart.fetch_add(1, Ordering::Relaxed);
        self.told
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(key, found.clone());
        found
    }

    pub fn entries(&self) -> &[Item] {
        &self.entries
    }

    /// The tracked `counts` as rows, in the default list order.
    pub fn rows(&self, counts: &HashMap<i64, Count>) -> Vec<Row> {
        rows(&self.entries, &self.rules, counts)
    }

    /// The entries the list shown in `cells` lacks (see `gaps`).
    pub fn gaps(&self, cells: &[Cell], start: bool, held: impl Fn(i64) -> bool) -> Vec<i64> {
        gaps(&self.entries, &self.rules, cells, start, held)
    }

    /// The index of the icon `id` is drawn with.
    #[cfg(test)]
    pub(crate) fn icon_of(&self, id: i64) -> Option<usize> {
        self.icon_keys
            .iter()
            .position(|keys| keys.iter().any(|k| k.1 == id))
    }

    /// Reads `cards` in reading order, as if the catalog lacked the entries
    /// drawn with icon `missing`.
    pub(crate) fn read_cards(
        &self,
        image: &RgbImage,
        cards: &[grid::Card],
        digits: &Digits,
        missing: Option<usize>,
    ) -> Vec<Cell> {
        // A card drawn as one read lately, where that one was, is taken as that one was read.
        let drawn: Vec<u64> = cards
            .iter()
            .map(|card| drawn(image, card, missing))
            .collect();
        let mut read: Vec<Option<ReadCard>> = {
            let reads = self.reads.lock().unwrap_or_else(PoisonError::into_inner);
            drawn
                .iter()
                .map(|drawn| reads.by_drawn.get(drawn).cloned())
                .collect()
        };
        let anew: Vec<usize> = (0..cards.len()).filter(|&i| read[i].is_none()).collect();
        self.read_anew.fetch_add(anew.len(), Ordering::Relaxed);
        let candidates: Vec<usize> = (0..self.icon_keys.len())
            .filter(|&i| Some(i) != missing)
            .collect();
        let read_card = |i: usize| {
            let card = &cards[i];
            let badge = grid::badge_bottom(image, card);
            ReadCard {
                told: self.tell(image, card, badge, &candidates, missing),
                badge,
                quantity: digits.read(image, card),
            }
        };
        // Cards are read independently, so more than a few share out over the CPU's cores.
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let per_thread = anew.len().div_ceil(threads).max(1);
        let fresh: Vec<ReadCard> = if anew.len() <= 2 {
            anew.iter().map(|&i| read_card(i)).collect()
        } else {
            std::thread::scope(|scope| {
                let workers: Vec<_> = anew
                    .chunks(per_thread)
                    .map(|chunk| {
                        let read_card = &read_card;
                        scope.spawn(move || chunk.iter().map(|&i| read_card(i)).collect::<Vec<_>>())
                    })
                    .collect();
                workers
                    .into_iter()
                    .flat_map(|worker| worker.join().expect("a card worker panicked"))
                    .collect()
            })
        };
        {
            let mut reads = self.reads.lock().unwrap_or_else(PoisonError::into_inner);
            for (&i, fresh) in anew.iter().zip(fresh) {
                reads.keep(drawn[i], fresh.clone());
                read[i] = Some(fresh);
            }
        }
        let read: Vec<ReadCard> = read.into_iter().flatten().collect();
        let badges: Vec<Option<u32>> = read.iter().map(|card| card.badge).collect();
        let found: Vec<&Result<usize, Vec<usize>>> = read.iter().map(|card| &card.told).collect();
        let icons: Vec<Option<usize>> = found.iter().map(|f| f.as_ref().ok().copied()).collect();
        // A cell is ignored when its icon, or every icon it could be, is untracked.
        let ignored: Vec<bool> = found
            .iter()
            .map(|f| match f {
                Ok(icon) => self.icon_ignored[*icon],
                Err(tied) => !tied.is_empty() && tied.iter().all(|&i| self.icon_ignored[i]),
            })
            .collect();
        // Time-limited entries (with a badge) are not in id order, so they take no
        // part in it and are named only when their icon belongs to a single entry.
        let ordered: Vec<Option<Vec<ListKey>>> = icons
            .iter()
            .zip(&badges)
            .map(|(icon, badge)| {
                icon.filter(|_| badge.is_none())
                    .map(|i| self.icon_keys[i].clone())
            })
            .collect();
        order::resolve(&ordered)
            .into_iter()
            .zip(icons.iter().zip(&badges))
            .zip(read.iter().zip(ignored))
            .map(|((key, (&icon, badge)), (card, ignored))| {
                let key = if badge.is_some() {
                    icon.and_then(|i| match self.icon_keys[i].as_slice() {
                        [only] => Some(*only),
                        _ => None,
                    })
                } else {
                    key
                };
                Cell {
                    id: key.map(|(_, id)| id),
                    quantity: card.quantity,
                    ignored,
                }
            })
            .collect()
    }
}

fn rows(entries: &[Item], rules: &Rules, counts: &HashMap<i64, Count>) -> Vec<Row> {
    let mut found: Vec<(&Item, Count)> = entries
        .iter()
        .filter(|e| (rules.tracked)(e))
        .filter_map(|e| counts.get(&e.id).map(|&count| (e, count)))
        .collect();
    found.sort_unstable_by_key(|(e, _)| (rules.shown)(e));
    found
        .into_iter()
        .map(|(e, count)| Row {
            id: e.id,
            name: e.name.clone(),
            icon: e.icon.clone(),
            tier: (e.tier > 0).then_some(e.tier),
            count,
            rarity: e.rarity,
            eleph: e.category == "SecretStone",
        })
        .collect()
}

/// The most entries `held` that can leave the list between two cells side by
/// side: an oopart's four tiers used up at once. More is a cell told as an
/// entry far off, or the start seen where it is not, which would drop much of
/// the list (as on 2026-10-16).
const MOST_GONE: usize = 4;

/// The entries a list in its default order lacks: those whose place lies
/// between two cells shown side by side, both told apart, and, where the
/// list's `start` shows, before its first cell; but none between two with more
/// than `MOST_GONE` entries `held` between them. An item used up leaves the
/// list, and this is how its going is seen.
pub fn gaps(
    entries: &[Item],
    rules: &Rules,
    cells: &[Cell],
    start: bool,
    held: impl Fn(i64) -> bool,
) -> Vec<i64> {
    let key = |id: i64| entries.iter().find(|e| e.id == id).map(rules.key);
    let before_all = start.then_some(Some((0, i64::MIN)));
    let keys: Vec<Option<ListKey>> = before_all
        .into_iter()
        .chain(cells.iter().map(|c| c.id.and_then(key)))
        .collect();
    let mut gone: Vec<(ListKey, i64)> = keys
        .windows(2)
        .filter_map(|pair| Some((pair[0]?, pair[1]?)))
        .flat_map(|(after, before)| {
            let between: Vec<(ListKey, i64)> = entries
                .iter()
                .map(|e| ((rules.key)(e), e.id))
                .filter(|(k, _)| after < *k && *k < before)
                .collect();
            let held_between = between.iter().filter(|(_, id)| held(*id)).count();
            if held_between > MOST_GONE {
                Vec::new()
            } else {
                between
            }
        })
        .collect();
    gone.sort_unstable();
    gone.into_iter().map(|(_, id)| id).collect()
}

/// The rarity of each of `icons`, where all the `entries` drawn with it have
/// the same.
pub fn rarities(entries: &[Item], icons: &[&str]) -> Vec<Option<Rarity>> {
    icons
        .iter()
        .map(|name| {
            let mut of = entries.iter().filter(|e| e.icon == *name).map(|e| e.rarity);
            let first = of.next().flatten();
            of.all(|r| r == first).then_some(first).flatten()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{equipment, items};

    fn entry(id: i64, name: &str, category: &str, tier: u8) -> Item {
        Item {
            id,
            name: name.into(),
            icon: String::new(),
            category: category.into(),
            tier,
            rarity: None,
        }
    }

    fn shown(entries: &[Item], rules: &Rules, counts: &[(i64, u32)]) -> Vec<String> {
        let counts = counts
            .iter()
            .map(|&(id, n)| (id, Count::Exact(n)))
            .collect();
        rows(entries, rules, &counts)
            .into_iter()
            .map(|r| {
                format!(
                    "{}{} {}",
                    r.tier.map_or(String::new(), |t| format!("T{t} ")),
                    r.name,
                    r.count
                )
            })
            .collect()
    }

    #[test]
    fn item_rows_follow_the_list_order_and_leave_out_untracked_items() {
        let entries = [
            entry(10, "material", "Material", 0),
            entry(80_660, "event point", "Material", 0),
            entry(3, "secret stone", "SecretStone", 0),
            entry(90_000, "event ticket", "Consumable", 0),
            entry(20, "unseen", "Material", 0),
        ];
        assert_eq!(
            shown(
                &entries,
                &items::RULES,
                &[(10, 5), (80_660, 1480), (3, 7), (90_000, 2)]
            ),
            ["event ticket 2", "secret stone 7", "material 5"]
        );
    }

    #[test]
    fn a_row_carries_its_entrys_rarity_and_whether_it_is_an_eleph() {
        let entries = [
            Item {
                rarity: Some(Rarity::SR),
                ..entry(10, "material", "Material", 0)
            },
            entry(3, "secret stone", "SecretStone", 0),
        ];
        let counts = HashMap::from([(10, Count::Exact(5)), (3, Count::Exact(7))]);
        let rows: Vec<_> = rows(&entries, &items::RULES, &counts)
            .into_iter()
            .map(|r| (r.id, r.rarity, r.eleph))
            .collect();
        assert_eq!(rows, [(3, None, true), (10, Some(Rarity::SR), false)]);
    }

    fn cell(id: Option<i64>) -> Cell {
        Cell {
            id,
            quantity: Some(Count::Exact(1)),
            ignored: false,
        }
    }

    #[test]
    fn what_lies_between_two_cells_side_by_side_is_gone() {
        let entries = [
            entry(10, "a", "Material", 0),
            entry(11, "b", "Material", 0),
            entry(12, "c", "Material", 0),
            entry(13, "d", "Material", 0),
            entry(14, "e", "Material", 0),
        ];
        let cells = [cell(Some(10)), cell(Some(13)), cell(Some(14))];
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false, |_| false),
            [11, 12]
        );
    }

    #[test]
    fn what_lies_before_the_first_cell_is_gone_only_where_the_start_shows() {
        let entries = [
            entry(10, "a", "Material", 0),
            entry(11, "b", "Material", 0),
            entry(12, "c", "Material", 0),
        ];
        let cells = [cell(Some(12))];
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, true, |_| false),
            [10, 11]
        );
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false, |_| false),
            Vec::<i64>::new()
        );
        let untold = [cell(None), cell(Some(12))];
        assert_eq!(
            gaps(&entries, &items::RULES, &untold, true, |_| false),
            Vec::<i64>::new()
        );
    }

    /// Cells side by side with more entries held between them than can be used
    /// up at once tell nothing: one of them is told as an entry far off, or the
    /// start is seen where it is not (2026-10-16: most of the items were
    /// dropped so).
    #[test]
    fn cells_with_many_entries_held_between_tell_nothing() {
        let entries: Vec<Item> = (10..30).map(|id| entry(id, "a", "Material", 0)).collect();
        let held = |id: i64| id != 12;
        let cells = [cell(Some(10)), cell(Some(29))];
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false, held),
            Vec::<i64>::new()
        );
        let first = [cell(Some(29))];
        assert_eq!(
            gaps(&entries, &items::RULES, &first, true, held),
            Vec::<i64>::new()
        );
        // As many as an oopart's tiers used up at once are gone.
        let cells = [cell(Some(10)), cell(Some(10 + MOST_GONE as i64 + 2))];
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false, held).len(),
            MOST_GONE + 1
        );
    }

    #[test]
    fn a_cell_not_told_apart_between_tells_nothing() {
        let entries = [
            entry(10, "a", "Material", 0),
            entry(11, "b", "Material", 0),
            entry(12, "c", "Material", 0),
        ];
        let cells = [cell(Some(10)), cell(None), cell(Some(12))];
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false, |_| false),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn equipment_rows_keep_every_entry_in_id_order_with_its_tier() {
        let entries = [
            entry(101_009, "gaming helmet blueprint", "Hat", 10),
            entry(1, "orb", "Exp", 0),
            entry(1000, "cap", "Hat", 1),
        ];
        assert_eq!(
            shown(
                &entries,
                &equipment::RULES,
                &[(101_009, 17), (1, 1383), (1000, 249)]
            ),
            ["orb 1383", "T1 cap 249", "T10 gaming helmet blueprint 17"]
        );
    }
}
