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
            equipment: [Some(Gear { tier: 1, level: 1 }); 3],
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
    let (rows, unknown) = recognizer.plan_rows(
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
    let (rows, _) = recognizer.plan_rows(
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
    let (rows, _) = recognizer.plan_rows(
        &BTreeMap::from([(10000, Target::default())]),
        &[aru(1)],
        &[],
        &[],
        None,
    );
    let goods: Vec<_> = rows
        .iter()
        .filter(|r| matches!(r.kind, Kind::Item | Kind::Equipment))
        .collect();
    assert!(goods.len() > 10, "{goods:?}");
    for row in goods {
        assert!(
            aoi::icon(&row.icon).is_some(),
            "{} {:?}",
            row.name,
            row.icon
        );
    }
    // Aru's skills take Gehenna's tech notes, 4030 the first tier.
    let family = |id| rows.iter().find(|r| r.id == id).and_then(|r| r.family);
    assert_eq!(family(4031), Some((4030, 2)));
    assert_eq!(family(9999), None);
}
