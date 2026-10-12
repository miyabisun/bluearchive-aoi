//! A list's entries and how its cells are told apart. The item list and the
//! equipment list share this; they differ only in their entries and `Rules`
//! (`items.rs`, `equipment.rs`).

use std::collections::HashMap;

use image::{RgbImage, RgbaImage};

use crate::{
    Cell, Item, Row, grid,
    icon::{IconMatcher, Mask},
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
        // An icon's rarity, where all its items have the same.
        let rarity = icons
            .iter()
            .map(|name| {
                let mut of = entries.iter().filter(|e| e.icon == *name).map(|e| e.rarity);
                let first = of.next().flatten();
                of.all(|r| r == first).then_some(first).flatten()
            })
            .collect();
        Ok(Self {
            matcher: IconMatcher::new(&images, rarity, rules.mask),
            icon_keys,
            icon_ignored,
            entries,
            rules,
        })
    }

    pub fn entries(&self) -> &[Item] {
        &self.entries
    }

    /// The tracked `counts` as rows, in the default list order.
    pub fn rows(&self, counts: &HashMap<i64, Count>) -> Vec<Row> {
        rows(&self.entries, &self.rules, counts)
    }

    /// The entries the list shown in `cells` lacks (see `gaps`).
    pub fn gaps(&self, cells: &[Cell], start: bool) -> Vec<i64> {
        gaps(&self.entries, &self.rules, cells, start)
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
        let badges: Vec<Option<u32>> = cards
            .iter()
            .map(|card| grid::badge_bottom(image, card))
            .collect();
        let candidates: Vec<usize> = (0..self.icon_keys.len())
            .filter(|&i| Some(i) != missing)
            .collect();
        // Cards are told apart independently, so they share out over the CPU's cores.
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let per_thread = cards.len().div_ceil(threads).max(1);
        let found: Vec<Result<usize, Vec<usize>>> = std::thread::scope(|scope| {
            let workers: Vec<_> = cards
                .chunks(per_thread)
                .zip(badges.chunks(per_thread))
                .map(|(chunk, badges)| {
                    let candidates = &candidates;
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .zip(badges)
                            .map(|(card, &badge)| {
                                self.matcher.identify(image, card, badge, candidates)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().expect("a card worker panicked"))
                .collect()
        });
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
            .zip(cards.iter().zip(ignored))
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
                    quantity: digits.read(image, card),
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
    found.sort_unstable_by_key(|(e, _)| (rules.key)(e));
    found
        .into_iter()
        .map(|(e, count)| Row {
            id: e.id,
            name: e.name.clone(),
            icon: e.icon.clone(),
            tier: (e.tier > 0).then_some(e.tier),
            count,
        })
        .collect()
}

/// The entries a list in its default order lacks: those whose place lies
/// between two cells shown side by side, both told apart, and, where the
/// list's `start` shows, before its first cell. An item used up leaves the
/// list, and this is how its going is seen.
pub fn gaps(entries: &[Item], rules: &Rules, cells: &[Cell], start: bool) -> Vec<i64> {
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
            entries
                .iter()
                .map(|e| ((rules.key)(e), e.id))
                .filter(move |(k, _)| after < *k && *k < before)
        })
        .collect();
    gone.sort_unstable();
    gone.into_iter().map(|(_, id)| id).collect()
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
        assert_eq!(gaps(&entries, &items::RULES, &cells, false), [11, 12]);
    }

    #[test]
    fn what_lies_before_the_first_cell_is_gone_only_where_the_start_shows() {
        let entries = [
            entry(10, "a", "Material", 0),
            entry(11, "b", "Material", 0),
            entry(12, "c", "Material", 0),
        ];
        let cells = [cell(Some(12))];
        assert_eq!(gaps(&entries, &items::RULES, &cells, true), [10, 11]);
        assert_eq!(
            gaps(&entries, &items::RULES, &cells, false),
            Vec::<i64>::new()
        );
        let untold = [cell(None), cell(Some(12))];
        assert_eq!(
            gaps(&entries, &items::RULES, &untold, true),
            Vec::<i64>::new()
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
            gaps(&entries, &items::RULES, &cells, false),
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
