//! The training plan over the built-in data, against totals known outside it:
//! the skill credits of the archived SchaleDB site and the game's level table.

use std::{collections::BTreeMap, path::Path};

use aoi::{
    Recognizer, Row,
    plan::{Held, Kind, Part, Target},
    quantity::Count,
    shop::{GoodRow, OfferRow, ShopRow},
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
        &[],
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
        &[],
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
        &[],
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

fn held(id: i64, count: u32) -> Row {
    Row {
        id,
        name: String::new(),
        icon: String::new(),
        tier: None,
        count: Count::Exact(count),
    }
}

/// The カケラ(文字) shop selling Aru's elephs at `rate`, `left` more to buy.
fn kakera(rate: Option<u32>, left: u32) -> ShopRow {
    ShopRow {
        name: "カケラ(文字)".to_owned(),
        read_at: String::new(),
        resets: false,
        coins: Vec::new(),
        goods: vec![GoodRow {
            id: 10000,
            name: String::new(),
            icon: String::new(),
            offers: vec![Some(OfferRow {
                left,
                price: None,
                shown: rate,
                coin: 0,
            })],
        }],
    }
}

/// The elephs a star target is short of are bought with fragments at the
/// rate the カケラ(文字) shop shows for the student.
#[test]
fn elephs_short_take_fragments_at_the_shops_rate() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let target = Target {
        level: 1,
        skills: [1; 4],
        equipment: [1, 1, 1],
        star: Some(5),
    };
    let targets = BTreeMap::from([(10000, target)]);
    let plan = |elephs: u32, shops: &[ShopRow]| {
        let items = [held(10000, elephs), held(23, 300)];
        recognizer.plan_rows(&targets, &[aru(1)], &items, &[], None, shops)
    };
    // ★3 to ★5 takes 100 + 120 elephs; 50 held leaves 170 short: 20 at
    // each of 1 to 4 fragments, then 90 at 5.
    let (rows, unknown) = plan(50, &[kakera(Some(1), 720)]);
    assert_eq!(need(&rows, Kind::Item, 10000), 220);
    let fragments = rows.iter().find(|r| r.id == 23).unwrap();
    assert_eq!(fragments.name, "神名のカケラ");
    assert_eq!(fragments.need, 20 + 40 + 60 + 80 + 90 * 5);
    assert_eq!(fragments.have, Some(Held::Exact(300)));
    assert!(unknown.is_empty(), "{unknown:?}");
    // Enough elephs, or none left in the shop, take no fragments.
    assert_eq!(
        need(&plan(220, &[kakera(Some(1), 720)]).0, Kind::Item, 23),
        0
    );
    assert_eq!(need(&plan(50, &[kakera(None, 0)]).0, Kind::Item, 23), 0);
    // Without the student's rate the fragments cannot be counted.
    let (rows, unknown) = plan(50, &[]);
    assert_eq!(need(&rows, Kind::Item, 23), 0);
    assert_eq!(unknown.len(), 1);
    assert_eq!((unknown[0].id, unknown[0].part), (10000, Part::Fragments));
}
