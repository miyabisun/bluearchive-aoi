//! The item and equipment tabs' tables by kind (the user's of 2026-10-21): the
//! skill materials by school, the ooparts by tier and the blueprints by slot
//! and tier, each with how many are held.

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

use serde::Serialize;

use crate::{
    Item,
    plan::{BLUEPRINTS, Piece},
    quantity::Count,
    work::{HeldCell, Raise, by_tier, raise, slot_data},
};

/// What a tiered skill material or an oopart is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A tactical education BD (an EX skill's).
    Bd,
    /// A tech note (the other skills').
    Note,
    Oopart,
}

/// The secret notes, by name, as the skill materials list them first.
pub const SECRET: [&str; 2] = ["秘伝ノート", "秘伝ノートの断片"];

/// The schools by the key their materials' icons carry
/// (`item_icon_skillbook_trinity_0`): as the shops list them, but the three
/// great schools first, whose materials run short most (the user's of
/// 2026-10-21). Schools not listed follow, in the order their materials come.
const SCHOOLS: [&str; 11] = [
    "trinity",
    "gehenna",
    "millennium",
    "hyakkiyako",
    "abydos",
    "shanhaijing",
    "redwinter",
    "valkyrie",
    "arius",
    "highlander",
    "wildhunt",
];

/// The tables by kind.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Kinds {
    /// The secret notes held (see `SECRET`).
    pub secret: Vec<HeldCell>,
    pub schools: Vec<SchoolMaterials>,
    /// Each oopart's tiers.
    pub ooparts: Vec<[Option<HeldCell>; 4]>,
    pub blueprints: Vec<SlotPrints>,
    /// The enhancement materials, kind by kind (see `ENHANCEMENTS`).
    pub enhancements: Vec<Enhancement>,
}

/// A school's skill materials by tier: its name, the icon of its emblem
/// (`school_icon_<key>`), its BDs and its tech notes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SchoolMaterials {
    pub name: String,
    pub icon: String,
    pub bd: [Option<HeldCell>; 4],
    pub notes: [Option<HeldCell>; 4],
}

/// A slot's name, the icon of its T10 piece, how many are held of its T1
/// piece and of each tier's blueprints from T2 to T10 (`tiers`, T1 first),
/// and of its universal blueprints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotPrints {
    pub name: String,
    pub icon: String,
    pub tiers: Vec<Option<HeldCell>>,
    pub universal: Option<HeldCell>,
}

/// The secret notes of `entries` (see `SECRET`) with how many are held and
/// what one student `needs`.
pub fn secret<S: BuildHasher, T: BuildHasher>(
    entries: &[Item],
    held: &HashMap<i64, Count, S>,
    needs: &HashMap<i64, u64, T>,
) -> Vec<HeldCell> {
    SECRET
        .iter()
        .filter_map(|name| entries.iter().find(|e| e.name == *name))
        .map(|e| HeldCell::of(e, held, needs))
        .collect()
}

/// An entry's kind (`item_icon_skillbook_trinity`) and, last in it, its
/// school's key ("trinity"): its icon without the tier's number.
fn stem(entry: &Item) -> Option<(&str, &str)> {
    let (stem, _) = entry.icon.rsplit_once('_')?;
    Some((stem, stem.rsplit('_').next()?))
}

/// A school's name as its materials' names tell it ("…（トリニティ）").
fn school_name(entry: &Item) -> Option<&str> {
    entry.name.split_once('（')?.1.strip_suffix('）')
}

/// The schools' BDs and tech notes of `entries`, told by `kind`, in the order
/// of `SCHOOLS`.
pub fn schools<S: BuildHasher, T: BuildHasher>(
    entries: &[Item],
    kind: impl Fn(&Item) -> Option<Kind>,
    held: &HashMap<i64, Count, S>,
    needs: &HashMap<i64, u64, T>,
) -> Vec<SchoolMaterials> {
    let materials: Vec<&Item> = entries
        .iter()
        .filter(|e| matches!(kind(e), Some(Kind::Bd | Kind::Note)))
        .collect();
    let mut keys: Vec<&str> = Vec::new();
    for key in materials.iter().filter_map(|e| stem(e).map(|(_, key)| key)) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort_by_key(|key| {
        SCHOOLS
            .iter()
            .position(|s| s == key)
            .unwrap_or(SCHOOLS.len())
    });
    keys.into_iter()
        .map(|key| {
            let of = |wanted: Kind| {
                let found = materials
                    .iter()
                    .copied()
                    .filter(|e| kind(e) == Some(wanted) && stem(e).map(|(_, k)| k) == Some(key));
                by_tier(found, |_| Some(String::new()), held, needs)
                    .pop()
                    .map(|(_, tiers)| tiers)
                    .unwrap_or_default()
            };
            let name = materials
                .iter()
                .filter(|e| stem(e).map(|(_, k)| k) == Some(key))
                .find_map(|e| school_name(e));
            SchoolMaterials {
                name: name.unwrap_or(key).to_owned(),
                icon: format!("school_icon_{key}"),
                bd: of(Kind::Bd),
                notes: of(Kind::Note),
            }
        })
        .collect()
}

/// The ooparts of `entries`, told by `kind`, each with its tiers, in the order
/// of their first tiers.
pub fn ooparts<S: BuildHasher, T: BuildHasher>(
    entries: &[Item],
    kind: impl Fn(&Item) -> Option<Kind>,
    held: &HashMap<i64, Count, S>,
    needs: &HashMap<i64, u64, T>,
) -> Vec<[Option<HeldCell>; 4]> {
    let found = entries.iter().filter(|e| kind(e) == Some(Kind::Oopart));
    by_tier(
        found,
        |e| stem(e).map(|(stem, _)| stem.to_owned()),
        held,
        needs,
    )
    .into_iter()
    .map(|(_, tiers)| tiers)
    .collect()
}

/// The enhancement materials of the equipment list by their category, in the
/// user's order of 2026-10-22: the orbs, then the unique weapons' springs,
/// hammers and barrels, and the pins every weapon takes.
const ENHANCEMENTS: [&str; 5] = [
    "Exp",
    "WeaponExpGrowthA",
    "WeaponExpGrowthB",
    "WeaponExpGrowthC",
    "WeaponExpGrowthZ",
];

/// A kind of enhancement material: its category, its tiers held, and how many
/// students what is held raises, as the work tab counts the scrimmages' parts
/// (the user's of 2026-10-22: 「生徒何人分？」). The work tab's normal items shop
/// shows the orbs' row itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Enhancement {
    pub category: String,
    pub tiers: [Option<HeldCell>; 4],
    pub raise: Raise,
}

/// What a kind of enhancement material feeds: the exp of one of each tier
/// (`(id, exp)`), the rate it is taken at and what one student takes.
pub struct Feed {
    pub exp: Vec<(i64, u64)>,
    pub rate: f64,
    pub each: u64,
}

/// The enhancement materials of `equipment`, a kind a row, in the order of
/// `ENHANCEMENTS`, each raising as its category's `feed` tells.
pub fn enhancements<S: BuildHasher>(
    equipment: &[Item],
    held: &HashMap<i64, Count, S>,
    feed: impl Fn(&str) -> Feed,
) -> Vec<Enhancement> {
    let none = HashMap::<i64, u64>::new();
    let mut rows = by_tier(
        equipment,
        |e| {
            ENHANCEMENTS
                .contains(&e.category.as_str())
                .then(|| e.category.clone())
        },
        held,
        &none,
    );
    rows.sort_by_key(|(kind, _)| ENHANCEMENTS.iter().position(|k| k == kind));
    rows.into_iter()
        .map(|(category, tiers)| {
            let feed = feed(&category);
            Enhancement {
                raise: raise(&feed.exp, feed.rate, feed.each, held),
                category,
                tiers,
            }
        })
        .collect()
}

/// Each slot's T1 piece and blueprints (see `SlotPrints`), in the order of
/// their T10 pieces.
pub fn blueprints<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    held: &HashMap<i64, Count, S>,
) -> Vec<SlotPrints> {
    let none = HashMap::<i64, u64>::new();
    let cell = |id: i64| {
        equipment
            .iter()
            .find(|e| e.id == id)
            .map(|e| HeldCell::of(e, held, &none))
    };
    slot_data(pieces, equipment, &BTreeMap::new())
        .into_iter()
        .map(|slot| {
            let first = pieces
                .iter()
                .find(|p| p.category == slot.category && p.tier == 1 && p.id < BLUEPRINTS);
            let print = |tier: u8| slot.prints.iter().find(|p| p.1 == tier).map(|p| p.0);
            let tiers = std::iter::once(first.map(|p| p.id))
                .chain((2..=10).map(print))
                .map(|id| id.and_then(cell))
                .collect();
            SlotPrints {
                tiers,
                universal: cell(slot.universal),
                name: slot.name,
                icon: slot.icon,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::Rarity;

    fn item(id: i64, name: &str, icon: &str, rarity: Rarity) -> Item {
        Item {
            id,
            name: name.into(),
            icon: icon.into(),
            category: "Material".into(),
            tier: 0,
            rarity: Some(rarity),
        }
    }

    /// A school's BD and tech note of the first two tiers.
    fn school(first: i64, key: &str, name: &str) -> [Item; 4] {
        let bd = |n: i64, tier: &str, rarity| {
            item(
                first + n,
                &format!("{tier}戦術教育BD（{name}）"),
                &format!("item_icon_material_exskill_{key}_{n}"),
                rarity,
            )
        };
        let note = |n: i64, tier: &str, rarity| {
            item(
                first + 1000 + n,
                &format!("{tier}技術ノート（{name}）"),
                &format!("item_icon_skillbook_{key}_{n}"),
                rarity,
            )
        };
        [
            bd(0, "初級", Rarity::N),
            bd(1, "中級", Rarity::R),
            note(0, "初級", Rarity::N),
            note(1, "中級", Rarity::R),
        ]
    }

    /// Tells the kinds apart as the shops' terms do, by the icon here.
    fn kind(e: &Item) -> Option<Kind> {
        if e.icon.starts_with("item_icon_material_exskill_") {
            Some(Kind::Bd)
        } else if e.icon.starts_with("item_icon_skillbook_") && !e.name.contains("秘伝") {
            Some(Kind::Note)
        } else if e.icon.starts_with("item_icon_material_") {
            Some(Kind::Oopart)
        } else {
            None
        }
    }

    /// Each cell as the id and the count held.
    fn cells(tiers: &[Option<HeldCell>]) -> Vec<Option<(i64, Option<Count>)>> {
        tiers
            .iter()
            .map(|c| c.as_ref().map(|c| (c.id, c.held)))
            .collect()
    }

    #[test]
    fn the_secret_notes_come_the_note_first_then_its_fragments() {
        let entries = [
            item(
                9998,
                "秘伝ノートの断片",
                "item_icon_skillbook_ultimate_piece",
                Rarity::SSR,
            ),
            item(
                9999,
                "秘伝ノート",
                "item_icon_skillbook_ultimate",
                Rarity::SSR,
            ),
        ];
        let held = HashMap::from([(9998, Count::Exact(41))]);
        let needs = HashMap::from([(9999, 3)]);
        let secret = secret(&entries, &held, &needs);
        let cells: Vec<_> = secret.iter().map(|c| (c.id, c.held, c.need)).collect();
        assert_eq!(
            cells,
            [(9999, None, Some(3)), (9998, Some(Count::Exact(41)), None)]
        );
    }

    #[test]
    fn the_schools_come_the_three_great_first_each_its_bds_over_its_notes() {
        let entries: Vec<Item> = [
            school(3000, "hyakkiyako", "百鬼夜行"),
            school(3020, "trinity", "トリニティ"),
            school(3030, "gehenna", "ゲヘナ"),
            school(3120, "odyssey", "オデュッセイア"),
            school(3050, "millennium", "ミレニアム"),
        ]
        .into_iter()
        .flatten()
        .collect();
        let held = HashMap::from([(3021, Count::Exact(7)), (4020, Count::Thousands(2))]);
        let needs = HashMap::from([(3020, 12)]);
        let schools = schools(&entries, kind, &held, &needs);
        let names: Vec<_> = schools
            .iter()
            .map(|s| (s.name.as_str(), s.icon.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("トリニティ", "school_icon_trinity"),
                ("ゲヘナ", "school_icon_gehenna"),
                ("ミレニアム", "school_icon_millennium"),
                ("百鬼夜行", "school_icon_hyakkiyako"),
                ("オデュッセイア", "school_icon_odyssey"),
            ]
        );
        let trinity = &schools[0];
        assert_eq!(
            cells(&trinity.bd),
            [
                Some((3020, None)),
                Some((3021, Some(Count::Exact(7)))),
                None,
                None
            ]
        );
        assert_eq!(
            cells(&trinity.notes),
            [
                Some((4020, Some(Count::Thousands(2)))),
                Some((4021, None)),
                None,
                None
            ]
        );
        assert_eq!(trinity.bd[0].as_ref().unwrap().need, Some(12));
    }

    #[test]
    fn each_oopart_is_a_row_of_its_tiers() {
        let entries = [
            item(
                100,
                "ネブラディスクの欠片",
                "item_icon_material_nebra_0",
                Rarity::N,
            ),
            item(
                101,
                "破損したネブラディスク",
                "item_icon_material_nebra_1",
                Rarity::R,
            ),
            item(
                103,
                "完全なネブラディスク",
                "item_icon_material_nebra_3",
                Rarity::SSR,
            ),
            item(
                110,
                "ファイストス円盤の欠片",
                "item_icon_material_phaistos_0",
                Rarity::N,
            ),
            item(
                3000,
                "初級戦術教育BD（百鬼夜行）",
                "item_icon_material_exskill_hyakkiyako_0",
                Rarity::N,
            ),
        ];
        let held = HashMap::from([(103, Count::Exact(2))]);
        let rows = ooparts(&entries, kind, &held, &HashMap::<i64, u64>::new());
        let rows: Vec<_> = rows.iter().map(|r| cells(r)).collect();
        assert_eq!(
            rows,
            [
                vec![
                    Some((100, None)),
                    Some((101, None)),
                    None,
                    Some((103, Some(Count::Exact(2))))
                ],
                vec![Some((110, None)), None, None, None],
            ]
        );
    }

    #[test]
    fn the_enhancement_materials_come_the_orbs_first_then_springs_hammers_barrels_and_pins() {
        let entry = |id: i64, category: &str, rarity| Item {
            id,
            name: format!("{category}{id}"),
            icon: String::new(),
            category: category.into(),
            tier: 0,
            rarity: Some(rarity),
        };
        // As the equipment list holds them: the parts by id, the orbs apart, a piece.
        let equipment = [
            entry(10, "WeaponExpGrowthA", Rarity::N),
            entry(13, "WeaponExpGrowthA", Rarity::SSR),
            entry(20, "WeaponExpGrowthB", Rarity::N),
            entry(30, "WeaponExpGrowthC", Rarity::N),
            entry(40, "WeaponExpGrowthZ", Rarity::N),
            entry(1000, "Hat", Rarity::N),
            entry(1, "Exp", Rarity::N),
            entry(4, "Exp", Rarity::SSR),
        ];
        let held = HashMap::from([(4, Count::Thousands(2)), (13, Count::Exact(7))]);
        // The orbs fed at 1,000 exp a top tier to students taking 500; the springs at
        // 1,000 exp a titanium one, 1.5 times, to weapons taking 1,000.
        let feed = |category: &str| match category {
            "Exp" => Feed {
                exp: vec![(1, 10), (4, 1000)],
                rate: 1.0,
                each: 500,
            },
            _ => Feed {
                exp: vec![(10, 10), (13, 1000)],
                rate: 1.5,
                each: 1000,
            },
        };
        let enhancements = enhancements(&equipment, &held, feed);
        let raised: Vec<_> = enhancements
            .iter()
            .map(|e| (e.category.as_str(), e.raise.students))
            .take(2)
            .collect();
        assert_eq!(raised, [("Exp", 4000.0), ("WeaponExpGrowthA", 10.5)]);
        assert!(enhancements[0].raise.about);
        let rows: Vec<_> = enhancements.iter().map(|e| cells(&e.tiers)).collect();
        assert_eq!(
            rows,
            [
                vec![
                    Some((1, None)),
                    None,
                    None,
                    Some((4, Some(Count::Thousands(2))))
                ],
                vec![
                    Some((10, None)),
                    None,
                    None,
                    Some((13, Some(Count::Exact(7))))
                ],
                vec![Some((20, None)), None, None, None],
                vec![Some((30, None)), None, None, None],
                vec![Some((40, None)), None, None, None],
            ]
        );
    }

    #[test]
    fn a_slot_shows_its_t1_piece_then_its_blueprints_by_tier_and_its_universal_ones() {
        let piece = |id, tier| Piece {
            id,
            category: "Hat".into(),
            tier,
            max_level: 1,
            recipe: Vec::new(),
            recipe_cost: 0,
        };
        let pieces = [
            piece(1000, 1),
            piece(1009, 10),
            piece(101_001, 2),
            piece(101_009, 10),
            piece(501_000, 0),
        ];
        let entry = |id, name: &str, icon: &str| Item {
            id,
            name: name.into(),
            icon: icon.into(),
            category: "Hat".into(),
            tier: 0,
            rarity: Some(Rarity::N),
        };
        let equipment = [
            entry(1000, "無地のキャップ", "equipment_icon_hat_tier1"),
            entry(1009, "ゲーミングヘルメット", "equipment_icon_hat_tier10"),
            entry(
                101_001,
                "ニット帽の設計図",
                "equipment_icon_hat_tier2_piece",
            ),
            entry(
                101_009,
                "ゲーミングヘルメットの設計図",
                "equipment_icon_hat_tier10_piece",
            ),
            entry(
                501_000,
                "帽子の万能設計図",
                "equipment_icon_hat_useall_piece",
            ),
        ];
        let held = HashMap::from([
            (1000, Count::Exact(249)),
            (101_009, Count::Exact(17)),
            (501_000, Count::Thousands(51)),
        ]);
        let slots = blueprints(&pieces, &equipment, &held);
        assert_eq!(slots.len(), 1);
        let hat = &slots[0];
        assert_eq!(
            (hat.name.as_str(), hat.icon.as_str()),
            ("帽子", "equipment_icon_hat_tier10")
        );
        let none = None;
        assert_eq!(
            cells(&hat.tiers),
            [
                Some((1000, Some(Count::Exact(249)))),
                Some((101_001, None)),
                none,
                none,
                none,
                none,
                none,
                none,
                none,
                Some((101_009, Some(Count::Exact(17)))),
            ]
        );
        let universal = hat.universal.as_ref().unwrap();
        assert_eq!(
            (universal.id, universal.held),
            (501_000, Some(Count::Thousands(51)))
        );
    }
}
