//! The equipment list: SchaleDB's equipment, all of it kept track of, in id
//! order (enhancement orbs, weapon parts, T1 equipment, blueprints by category
//! and tier, then the universal blueprints).

use crate::{
    Item, Read,
    catalog::{Catalog, ListKey, Rules},
    icon::Mask,
    json, picture,
};

fn key(item: &Item) -> ListKey {
    (0, item.id)
}

pub const RULES: Rules = Rules {
    tracked: |_| true,
    key,
    // The tier badge ("T10") at the bottom left and the "xN" count at the
    // bottom right together cover the bottom of the icon.
    mask: Mask {
        left: 0.0,
        top: 0.72,
    },
};

/// The game draws a few icons unlike SchaleDB (bigger, so a thin cord no longer
/// lines up); those are cut from screenshots into `data/extra/equipment-icons`
/// and used instead.
pub fn catalog(read: &Read) -> Result<Catalog, String> {
    let equipment: Vec<Item> = json(read, "schaledb/equipment.json")?;
    Catalog::new(equipment, RULES, |name| {
        picture(read, &format!("extra/equipment-icons/{name}.png"))
            .or_else(|_| picture(read, &format!("schaledb/equipment-icons/{name}.webp")))
    })
}
