//! The item list: SchaleDB's items and the few it lacks (`data/extra`, icons
//! cut from screenshots), consumables first, the time-limited event items left
//! out.

use image::RgbaImage;
use serde::Deserialize;

use crate::{
    Item, Read,
    catalog::{Catalog, ListKey, Rules},
    icon::Mask,
    json, picture,
    shop::{self, Goods},
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
    shown: key,
    // The "xN" count covers the bottom right of the icon.
    mask: Mask {
        left: 0.25,
        top: 0.8,
    },
};

pub fn catalog(read: &Read) -> Result<Catalog, String> {
    let (items, icon) = with_icons(read)?;
    Catalog::new(items, RULES, icon)
}

/// A currency of the SchaleDB snapshot (`data/schaledb/currency.json`).
#[derive(Deserialize)]
struct Currency {
    id: i64,
    name: String,
    icon: String,
}

/// The shops' goods and what tells them apart on a card: the items, and the
/// equipment list's entries and the currencies whose names `sold` takes
/// (チタン撃針, クレジットポイント), which the item list lacks; these take ids
/// past the items' (`shop::EQUIPMENT_GOODS`, `shop::CURRENCY_GOODS`).
pub fn goods(read: &Read, sold: impl Fn(&str) -> bool) -> Result<(Vec<Item>, Goods), String> {
    let (mut goods, icon) = with_icons(read)?;
    let equipment: Vec<Item> = json::<Vec<Item>>(read, "schaledb/equipment.json")?
        .into_iter()
        .filter(|e| sold(&e.name))
        .map(|e| Item {
            id: shop::EQUIPMENT_GOODS + e.id,
            ..e
        })
        .collect();
    let equipment_icons: Vec<String> = equipment.iter().map(|e| e.icon.clone()).collect();
    goods.extend(equipment);
    goods.extend(
        json::<Vec<Currency>>(read, "schaledb/currency.json")?
            .into_iter()
            .filter(|c| sold(&c.name))
            .map(|c| Item {
                id: shop::CURRENCY_GOODS + c.id,
                name: c.name,
                icon: c.icon,
                category: "Currency".to_owned(),
                tier: 0,
                rarity: None,
            }),
    );
    let matcher = Goods::new(&goods, |name: &str| {
        if equipment_icons.iter().any(|e| e == name) {
            picture(read, &format!("schaledb/equipment-icons/{name}.webp"))
        } else {
            icon(name)
        }
    })?;
    Ok((goods, matcher))
}

/// An icon's picture, or why it could not be read.
type Picture = Result<RgbaImage, String>;

/// SchaleDB's items and those it lacks, and their icons by name.
fn with_icons<'a>(read: &'a Read) -> Result<(Vec<Item>, impl Fn(&str) -> Picture + 'a), String> {
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
    Ok((items, move |name: &str| {
        if extra_icons.iter().any(|e| e == name) {
            picture(read, &format!("extra/icons/{name}.png"))
        } else {
            picture(read, &format!("schaledb/icons/{name}.webp"))
        }
    }))
}
