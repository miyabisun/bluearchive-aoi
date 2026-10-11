//! Reads item kinds and counts from screenshots of the Blue Archive item list.

pub mod grid;
mod icon;
mod order;
pub mod quantity;

use std::path::Path;

use image::RgbImage;
use serde::Deserialize;

use icon::IconMatcher;
use quantity::Digits;

/// One fully visible cell of the item list, in reading order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// SchaleDB item id, or `None` when the icon could not be told apart.
    pub id: Option<i64>,
    /// Count shown as "xN", or `None` when it could not be read.
    pub quantity: Option<u32>,
}

/// An entry of the SchaleDB snapshot (`data/schaledb/items.json`).
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
    matcher: IconMatcher,
    digits: Digits,
}

impl Recognizer {
    /// Loads the SchaleDB snapshot from `data` (the crate's `data` directory).
    pub fn load(data: &Path) -> Result<Self, String> {
        let dir = data.join("schaledb");
        let json = std::fs::read_to_string(dir.join("items.json")).map_err(|e| e.to_string())?;
        let items: Vec<Item> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let mut icons: Vec<&str> = items.iter().map(|i| i.icon.as_str()).collect();
        icons.sort_unstable();
        icons.dedup();
        let images = icons
            .iter()
            .map(|name| {
                let path = dir.join("icons").join(format!("{name}.webp"));
                image::open(&path)
                    .map(|i| i.to_rgba8())
                    .map_err(|e| format!("{}: {e}", path.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
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
        let digits =
            std::fs::read_to_string(data.join("digits.json")).map_err(|e| e.to_string())?;
        Ok(Self {
            icon_keys,
            matcher: IconMatcher::new(&images),
            digits: serde_json::from_str(&digits).map_err(|e| e.to_string())?,
            items,
        })
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Reads the fully visible cells in reading order.
    pub fn read(&self, image: &RgbImage) -> Vec<Cell> {
        self.read_without(image, None)
    }

    /// Reads as if the snapshot lacked the items drawn with icon `missing`.
    fn read_without(&self, image: &RgbImage, missing: Option<usize>) -> Vec<Cell> {
        let cards = grid::find_cards(image);
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
        let options: Vec<Option<Vec<ListKey>>> = std::thread::scope(|scope| {
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
                                let icon = self.matcher.identify(image, card, badge, candidates)?;
                                Some(self.icon_keys[icon].clone())
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
        // Time-limited items (with a badge) are not in id order, so they take no part
        // in it and are named only when their icon belongs to a single item.
        let ordered: Vec<Option<Vec<ListKey>>> = options
            .iter()
            .zip(&badges)
            .map(|(keys, badge)| keys.clone().filter(|_| badge.is_none()))
            .collect();
        order::resolve(&ordered)
            .into_iter()
            .zip(options.iter().zip(&badges))
            .zip(&cards)
            .map(|((key, (keys, badge)), card)| {
                let key = if badge.is_some() {
                    keys.as_deref().and_then(|keys| match keys {
                        [only] => Some(*only),
                        _ => None,
                    })
                } else {
                    key
                };
                Cell {
                    id: key.map(|(_, id)| id),
                    quantity: self.digits.read(image, card),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
