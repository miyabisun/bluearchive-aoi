//! Reads item kinds and counts from screenshots of the Blue Archive item list.

pub mod app;
pub mod grid;
pub mod icon;
mod order;
pub mod quantity;
pub mod screen;
pub mod tally;
pub mod watch;
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

use std::{collections::HashMap, path::Path};

use image::RgbImage;
use serde::{Deserialize, Serialize};

use icon::IconMatcher;
use quantity::Digits;

/// One fully visible cell of the item list, in reading order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// SchaleDB item id, or `None` when the icon could not be told apart.
    pub id: Option<i64>,
    /// Count shown as "xN", or `None` when it could not be read.
    pub quantity: Option<u32>,
    /// An item the user does not keep track of (see `tracked`): the cell is read
    /// for the list order but is not to be shown or exported.
    pub ignored: bool,
}

/// Whether the user keeps track of an item. On 2026-10-11 they dropped, from the
/// event point (P) on, the time-limited event items (gone with their event and
/// looked up on the event page) and the Valentine chocolates (they pile up for
/// every student within a few years). Consumables in that id range stay.
pub fn tracked(item: &Item) -> bool {
    item.category == "Consumable" || !(80_000..190_000).contains(&item.id)
}

/// An entry of the SchaleDB snapshot (`data/schaledb/items.json`), or of the
/// items it lacks (`data/extra/items.json`).
#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub category: String,
    pub rarity: String,
}

/// Categories in the order the default list shows them; within a category
/// items ascend by id. Categories not listed come last.
const CATEGORY_ORDER: &[&str] = &["Consumable"];

/// Where an item sits in the default list order.
type ListKey = (usize, i64);

fn list_key(item: &Item) -> ListKey {
    let rank = CATEGORY_ORDER
        .iter()
        .position(|c| *c == item.category)
        .unwrap_or(CATEGORY_ORDER.len());
    (rank, item.id)
}

pub struct Recognizer {
    items: Vec<Item>,
    /// List positions of the items drawn with each icon, ascending; the
    /// matcher's icon indices point here.
    icon_keys: Vec<Vec<ListKey>>,
    /// Whether every item drawn with each icon is untracked.
    icon_ignored: Vec<bool>,
    matcher: IconMatcher,
    digits: Digits,
    /// The sort buttons in the default order (`data/sort-default.png`).
    sort_template: RgbImage,
}

impl Recognizer {
    /// Loads the files under `data` (the crate's `data` directory): the SchaleDB
    /// snapshot (`schaledb`), the items it lacks (`extra`, icons cut from
    /// screenshots), the digit references and the default sort buttons.
    pub fn load(data: &Path) -> Result<Self, String> {
        Self::build(&|path| {
            std::fs::read(data.join(path))
                .map_err(|e| format!("{}: {e}", data.join(path).display()))
        })
    }

    /// The same data, built into the program.
    pub fn embedded() -> Result<Self, String> {
        Self::build(&|path| {
            embedded::FILES
                .iter()
                .find(|(name, _)| *name == path)
                .map(|(_, bytes)| bytes.to_vec())
                .ok_or_else(|| format!("{path} is not built in"))
        })
    }

    fn build(read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Self, String> {
        let items_in = |path: &str| -> Result<Vec<Item>, String> {
            serde_json::from_slice(&read(path)?).map_err(|e| format!("{path}: {e}"))
        };
        let mut items = items_in("schaledb/items.json")?;
        let extras = items_in("extra/items.json")?;
        if let Some(both) = extras.iter().find(|e| items.iter().any(|i| i.id == e.id)) {
            return Err(format!(
                "SchaleDB now has id {} ({}); remove it from data/extra",
                both.id, both.name
            ));
        }
        let extra_icons: Vec<String> = extras.iter().map(|e| e.icon.clone()).collect();
        items.extend(extras);
        let mut icons: Vec<&str> = items.iter().map(|i| i.icon.as_str()).collect();
        icons.sort_unstable();
        icons.dedup();
        let picture = |path: &str| -> Result<image::DynamicImage, String> {
            image::load_from_memory(&read(path)?).map_err(|e| format!("{path}: {e}"))
        };
        let images = icons
            .iter()
            .map(|name| {
                let path = if extra_icons.iter().any(|e| e == name) {
                    format!("extra/icons/{name}.png")
                } else {
                    format!("schaledb/icons/{name}.webp")
                };
                picture(&path).map(|i| i.to_rgba8())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let icon_ignored = icons
            .iter()
            .map(|name| {
                items
                    .iter()
                    .filter(|i| i.icon == *name)
                    .all(|i| !tracked(i))
            })
            .collect();
        let icon_keys = icons
            .iter()
            .map(|name| {
                let mut keys: Vec<ListKey> = items
                    .iter()
                    .filter(|i| i.icon == *name)
                    .map(list_key)
                    .collect();
                keys.sort_unstable();
                keys
            })
            .collect();
        let digits = serde_json::from_slice(&read("digits.json")?)
            .map_err(|e| format!("digits.json: {e}"))?;
        Ok(Self {
            icon_keys,
            icon_ignored,
            matcher: IconMatcher::new(&images),
            digits,
            sort_template: picture("sort-default.png")?.to_rgb8(),
            items,
        })
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The tally's tracked counts, in the default list order.
    pub fn rows(&self, tally: &tally::Tally) -> Vec<Row> {
        rows(&self.items, tally.counts())
    }

    /// Reads the fully visible cells in reading order.
    pub fn read(&self, image: &RgbImage) -> Vec<Cell> {
        self.read_cards(image, &grid::find_cards(image), None)
    }

    /// Reads the cells when the frame shows the item list sorted the default way,
    /// ascending; any other screen or order gives `None`.
    pub fn read_list(&self, image: &RgbImage) -> Option<Vec<Cell>> {
        let (panel, cards) = grid::find(image)?;
        screen::sorted_by_default(image, panel.card_height, &self.sort_template)
            .then(|| self.read_cards(image, &cards, None))
    }

    /// Reads as if the snapshot lacked the items drawn with icon `missing`.
    #[cfg(test)]
    fn read_without(&self, image: &RgbImage, missing: Option<usize>) -> Vec<Cell> {
        self.read_cards(image, &grid::find_cards(image), missing)
    }

    fn read_cards(
        &self,
        image: &RgbImage,
        cards: &[grid::Card],
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
        // Time-limited items (with a badge) are not in id order, so they take no part
        // in it and are named only when their icon belongs to a single item.
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
                    quantity: self.digits.read(image, card),
                    ignored,
                }
            })
            .collect()
    }
}

/// One line of the tally, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub id: i64,
    pub name: String,
    pub count: u32,
}

/// The tracked counts as rows, in the default list order.
fn rows(items: &[Item], counts: &HashMap<i64, u32>) -> Vec<Row> {
    let mut found: Vec<(&Item, u32)> = items
        .iter()
        .filter(|item| tracked(item))
        .filter_map(|item| counts.get(&item.id).map(|&count| (item, count)))
        .collect();
    found.sort_unstable_by_key(|(item, _)| list_key(item));
    found
        .into_iter()
        .map(|(item, count)| Row {
            id: item.id,
            name: item.name.clone(),
            count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i64, name: &str, category: &str) -> Item {
        Item {
            id,
            name: name.into(),
            icon: String::new(),
            category: category.into(),
            rarity: "N".into(),
        }
    }

    #[test]
    fn rows_follow_the_list_order_and_leave_out_untracked_items() {
        let items = [
            item(10, "material", "Material"),
            item(80_660, "event point", "Material"),
            item(3, "secret stone", "SecretStone"),
            item(90_000, "event ticket", "Consumable"),
            item(20, "unseen", "Material"),
        ];
        let counts = HashMap::from([(10, 5), (80_660, 1480), (3, 7), (90_000, 2)]);
        let names: Vec<(String, u32)> = rows(&items, &counts)
            .into_iter()
            .map(|r| (r.name, r.count))
            .collect();
        assert_eq!(
            names,
            [
                ("event ticket".to_owned(), 2),
                ("secret stone".to_owned(), 7),
                ("material".to_owned(), 5),
            ]
        );
    }

    #[derive(Deserialize)]
    struct Fixture {
        image: String,
        cells: Vec<Label>,
    }

    #[derive(Deserialize)]
    struct Label {
        id: Option<i64>,
    }

    /// An item missing from the snapshot (a new release) must never be read
    /// as a look-alike, nor make other cells wrong. Each labeled item is
    /// removed in turn and the whole screenshot is read again.
    #[test]
    fn an_item_missing_from_the_snapshot_is_never_misread() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let recognizer = Recognizer::load(&root.join("data")).unwrap();
        let fixtures: Vec<Fixture> = serde_json::from_str(
            &std::fs::read_to_string(root.join("tests/fixtures/labels.json")).unwrap(),
        )
        .unwrap();
        let mut wrong = Vec::new();
        for fixture in fixtures {
            let image = image::open(root.join("tests/fixtures").join(&fixture.image))
                .unwrap()
                .to_rgb8();
            for id in fixture.cells.iter().filter_map(|l| l.id) {
                let missing = recognizer
                    .icon_keys
                    .iter()
                    .position(|keys| keys.iter().any(|k| k.1 == id));
                let read = recognizer.read_without(&image, missing);
                for (cell, label) in read.iter().zip(&fixture.cells) {
                    let expected = label.id.filter(|&l| l != id);
                    if cell.id.is_some() && cell.id != expected {
                        wrong.push(format!(
                            "{} without {id}: read {:?} where {:?} is",
                            fixture.image, cell.id, label.id
                        ));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }
}
