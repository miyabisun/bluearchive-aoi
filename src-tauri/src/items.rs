//! The item list: SchaleDB's items and the few it lacks (`data/extra`, icons
//! cut from screenshots), consumables first, the time-limited event items left
//! out.

use crate::{
    Item, Read,
    catalog::{Catalog, ListKey, Rules},
    icon::Mask,
    json, picture,
};

/// Whether the user keeps track of an item. On 2026-10-11 they dropped, from the
/// event point (P) on, the time-limited event items (gone with their event and
/// looked up on the event page) and the Valentine chocolates (they pile up for
/// every student within a few years). Consumables in that id range stay.
fn tracked(item: &Item) -> bool {
    item.category == "Consumable" || !(80_000..190_000).contains(&item.id)
}

/// Categories in the order the default list shows them; within a category
/// items ascend by id. Categories not listed come last.
const CATEGORY_ORDER: &[&str] = &["Consumable"];

fn key(item: &Item) -> ListKey {
    let rank = CATEGORY_ORDER
        .iter()
        .position(|c| *c == item.category)
        .unwrap_or(CATEGORY_ORDER.len());
    (rank, item.id)
}

pub const RULES: Rules = Rules {
    tracked,
    key,
    // The "xN" count covers the bottom right of the icon.
    mask: Mask {
        left: 0.25,
        top: 0.8,
    },
};

pub fn catalog(read: &Read) -> Result<Catalog, String> {
    let mut items: Vec<Item> = json(read, "schaledb/items.json")?;
    let extras: Vec<Item> = json(read, "extra/items.json")?;
    if let Some(both) = extras.iter().find(|e| items.iter().any(|i| i.id == e.id)) {
        return Err(format!(
            "SchaleDB now has id {} ({}); remove it from data/extra",
            both.id, both.name
        ));
    }
    let extra_icons: Vec<String> = extras.iter().map(|e| e.icon.clone()).collect();
    items.extend(extras);
    Catalog::new(items, RULES, |name| {
        if extra_icons.iter().any(|e| e == name) {
            picture(read, &format!("extra/icons/{name}.png"))
        } else {
            picture(read, &format!("schaledb/icons/{name}.webp"))
        }
    })
}
