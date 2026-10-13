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

/// Slot by slot, the pieces T1 to T10, then the blueprints T2 to T10 and the
/// universal one; the orbs and the weapon parts first, in id order (the game
/// lists every slot's pieces before any blueprint; the user's choice,
/// 2026-10-13: "帽子T1 -> T10設計図をつなげる"). The ids tell the slot: hats
/// are 1000, 101001 and 501000 on.
fn shown(item: &Item) -> ListKey {
    let id = item.id;
    let rank = |base: i64| base + i64::from(item.tier);
    match id {
        ..1000 => (0, id),
        1000..100_000 => ((id / 1000) as usize, rank(0)),
        100_000..500_000 => ((id / 1000 % 100) as usize, rank(100)),
        _ => ((id / 1000 % 100) as usize, rank(200)),
    }
}

pub const RULES: Rules = Rules {
    tracked: |_| true,
    key,
    shown,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: i64, tier: u8) -> Item {
        Item {
            id,
            name: String::new(),
            icon: String::new(),
            category: String::new(),
            tier,
            rarity: None,
        }
    }

    #[test]
    fn a_slots_pieces_lead_to_its_blueprints_and_its_universal_one() {
        let mut shown_order = vec![
            piece(2000, 1),
            piece(101_009, 10),
            piece(501_000, 0),
            piece(1009, 10),
            piece(102_002, 2),
            piece(10, 0),
            piece(1000, 1),
            piece(101_002, 2),
            piece(1, 0),
        ];
        shown_order.sort_by_key(shown);
        let ids: Vec<i64> = shown_order.iter().map(|p| p.id).collect();
        assert_eq!(
            ids,
            [1, 10, 1000, 1009, 101_002, 101_009, 501_000, 2000, 102_002]
        );
    }
}
