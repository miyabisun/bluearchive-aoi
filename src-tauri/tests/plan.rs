//! The training plan over the built-in data, against totals known outside it:
//! the skill credits of the archived SchaleDB site and the game's level table.

use std::{collections::BTreeMap, path::Path};

use aoi::{
    Recognizer,
    plan::{Kind, Target},
    student::{Gear, Training},
    watch::StudentRow,
};

fn aru(level: u8) -> StudentRow {
    StudentRow {
        id: 10000,
        name: "アル".to_owned(),
        training: Training {
            level: Some(level),
            rarity: Some(3),
            bond: Some(1),
            skills: [Some(1); 4],
            weapon: Some(None),
            equipment: [Some(Gear { tier: 1 }); 3],
            gear: Some(0),
            ability: [Some(0); 3],
        },
    }
}

fn need(rows: &[aoi::plan::NeedRow], kind: Kind, id: i64) -> u64 {
    rows.iter()
        .find(|r| r.kind == kind && r.id == id)
        .map_or(0, |r| r.need)
}

#[test]
fn skills_from_lv1_to_the_top_take_the_known_credits_and_three_notes() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let target = Target {
        level: 1,
        equipment: [1, 1, 1],
        ..Target::default()
    };
    let (rows, _, unknown) = recognizer.plan_rows(
        &BTreeMap::from([(10000, target)]),
        &[aru(1)],
        &[],
        &[],
        None,
    );
    assert!(unknown.is_empty(), "{unknown:?}");
    // EX Lv1 to 5, and three skills Lv1 to 10.
    assert_eq!(need(&rows, Kind::Credits, 0), 13_580_000 + 3 * 8_812_500);
    assert_eq!(need(&rows, Kind::Item, 9999), 3);
}

#[test]
fn lv1_to_90_takes_the_game_tables_exp() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let target = Target {
        skills: [1; 4],
        equipment: [1, 1, 1],
        ..Target::default()
    };
    let (rows, _, _) = recognizer.plan_rows(
        &BTreeMap::from([(10000, target)]),
        &[aru(1)],
        &[],
        &[],
        None,
    );
    assert_eq!(need(&rows, Kind::ReportExp, 0), 1_249_185);
    assert_eq!(need(&rows, Kind::Credits, 0), 1_249_185 * 7);
}

/// What the plan asks for of items and blueprints shows by icons the window
/// loads, and the kinds of four tiers go by their first tier.
#[test]
fn the_planned_goods_have_icons_and_the_tiered_kinds_families() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let (rows, blueprints, _) = recognizer.plan_rows(
        &BTreeMap::from([(10000, Target::default())]),
        &[aru(1)],
        &[],
        &[],
        None,
    );
    let goods: Vec<(&str, &str)> = rows
        .iter()
        .filter(|r| r.kind == Kind::Item)
        .map(|r| (r.name.as_str(), r.icon.as_str()))
        .collect();
    assert!(goods.len() > 10, "{goods:?}");
    let slots = blueprints
        .iter()
        .map(|b| (b.name.as_str(), b.icon.as_str()));
    let tiers = blueprints
        .iter()
        .flat_map(|b| b.tiers.iter().chain([&b.universal]))
        .map(|t| (t.name.as_str(), t.icon.as_str()));
    for (name, icon) in goods.iter().copied().chain(slots).chain(tiers) {
        assert!(aoi::icon(icon).is_some(), "{name} {icon:?}");
    }
    // Aru's skills take Gehenna's tech notes, 4030 the first tier.
    let family = |id| rows.iter().find(|r| r.id == id).and_then(|r| r.family);
    assert_eq!(family(4031), Some((4030, 2)));
    assert_eq!(family(9999), None);
}

#[test]
fn equipment_from_t1_to_t10_takes_each_slots_blueprints_the_universal_ones_paying_t2_to_t9() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let target = Target {
        level: 1,
        skills: [1; 4],
        ..Target::default()
    };
    let (rows, blueprints, _) = recognizer.plan_rows(
        &BTreeMap::from([(10000, target)]),
        &[aru(1)],
        &[],
        &[],
        None,
    );
    // Aru's hat, hairpin and watch, none of their blueprints listed alone.
    let names: Vec<&str> = blueprints.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["帽子", "ヘアピン", "腕時計"]);
    assert!(rows.iter().all(|r| r.kind != Kind::Equipment));
    // A hat: 60 T10 blueprints, and T2 to T9's 5,175 universal ones
    // (schale-calc's equipment-upgrade.md), none held.
    let hat = &blueprints[0];
    assert_eq!(hat.tiers.len(), 9);
    assert_eq!((hat.short, hat.universal.need), (60, 5175));
}
