//! The work tab over the built-in data: the blueprints a piece from T1 to T10
//! takes, what the daily stages drop, and a raise from Lv1.

use std::path::Path;

use aoi::{Recognizer, Row, quantity::Count, work::Stages};

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap()
}

fn row(id: i64, count: u32) -> Row {
    Row {
        id,
        name: String::new(),
        icon: String::new(),
        tier: None,
        count: Count::Exact(count),
        rarity: None,
        eleph: false,
    }
}

#[test]
fn each_slot_counts_pieces_from_its_blueprints() {
    // 270 T10 hat blueprints; 1,000 hat universal blueprints and 290 of T9,
    // 30 universal each.
    let equipment = [row(101_009, 270), row(501_000, 1000), row(101_008, 290)];
    let work = recognizer().work_rows(&[], &equipment, None);
    assert_eq!(work.slots.len(), 9);
    let hat = work.slots.iter().find(|s| s.name == "帽子").unwrap();
    // A T10 takes 60 T10 blueprints; T2 to T10 take 430 of T9 and below,
    // 5,175 universal ones (schale-calc's equipment-upgrade.md).
    assert_eq!((hat.top.need, hat.top.pieces), (60, 4.5));
    assert_eq!((hat.universal.held, hat.universal.need), (9700, 5175));
    assert_eq!(hat.golden.students, 3);
}

#[test]
fn the_farm_reaches_the_golden_ratio_from_nothing_in_normal_missions() {
    let work = recognizer().work_rows(&[], &[], None);
    let farm = work.farm.unwrap();
    assert_eq!(farm.multiplier, 2);
    assert!(farm.runs > 0 && farm.ap == farm.runs * 10, "{farm:?}");
    // Every slot is served by some mission of the farm.
    for slot in &work.slots {
        assert!(
            farm.stages.iter().any(|s| s.slots.contains(&slot.name)),
            "{}",
            slot.name
        );
    }
}

#[test]
fn the_farm_plays_for_t10_blueprints_and_gives_the_rest_where_the_ratio_wants_it() {
    // By slot, hat to necklace: hats and hairpins far short of the ratio's
    // T10 blueprints, watches and necklaces a little, bags and badges well
    // over; and the universal ones, in thousands.
    let top = [17, 154, 534, 441, 203, 36, 185, 139, 85];
    let universal = [51, 16, 82, 64, 30, 19, 29, 26, 19];
    let equipment: Vec<Row> = (1..=9)
        .zip(top.into_iter().zip(universal))
        .flat_map(|(n, (top, universal))| {
            [
                row(100_009 + n * 1000, top),
                row(500_000 + n * 1000, universal * 1000),
            ]
        })
        .collect();
    let work = recognizer().work_rows(&[], &equipment, None);
    let farm = work.farm.unwrap();
    let names: Vec<&str> = farm.stages.iter().map(|s| s.name.as_str()).collect();
    // A mission's slots, the one it drops most of first: 29-2 hairpins.
    let slots = |name| {
        farm.stages
            .iter()
            .find(|s| s.name == name)
            .map(|s| &s.slots)
    };
    assert_eq!(slots("29-2").unwrap(), &["ヘアピン", "シューズ", "腕時計"]);
    // 28-4 drops hairpins as 29-2 does, but bags and badges with them.
    assert!(
        names.contains(&"29-2") && !names.contains(&"28-4"),
        "{names:?}"
    );
    // Each slot's T10 blueprints reach the ratio's from their drops alone,
    // a piece taking 60.
    let stages: Stages = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("data/schaledb/stages.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for ((n, slot), held) in (1..=9).zip(&work.slots).zip(top) {
        let dropped: f64 = farm
            .stages
            .iter()
            .map(|s| {
                let mission = stages.missions.iter().find(|m| m.name == s.name).unwrap();
                let each: f64 = mission
                    .drops
                    .iter()
                    .filter(|d| d.0 == 100_009 + n * 1000)
                    .map(|d| d.1)
                    .sum();
                each * 2.0 * f64::from(s.runs)
            })
            .sum();
        let want = f64::from(slot.golden.students * 60);
        assert!(f64::from(held) + dropped >= want, "{}", slot.name);
    }
}

#[test]
fn a_student_takes_their_school_notes_and_their_ooparts_with_the_potential() {
    let work = recognizer().work_rows(&[], &[], None);
    let need = |id: i64| {
        work.bounties
            .iter()
            .flat_map(|p| &p.kinds)
            .flatten()
            .flatten()
            .find(|c| c.id == id)
            .and_then(|c| c.need)
    };
    // Gehenna's first notes: 5 + 8 + 12 a skill, three skills.
    assert_eq!(need(4030), Some(75));
    // Phaistos: a potential takes 225 of the first tier a stat, three stats.
    assert!(need(110).unwrap() >= 675, "{:?}", need(110));
}

#[test]
fn the_bounties_and_scrimmages_show_what_they_drop() {
    let work = recognizer().work_rows(&[row(4030, 12)], &[row(10, 5)], None);
    assert_eq!(work.bounties.len(), 5);
    // Two or three schools' tactical BDs and tech notes, and two ooparts, in
    // four tiers.
    for place in &work.bounties {
        assert!([6, 8].contains(&place.kinds.len()), "{}", place.name);
        assert!(
            place.kinds.iter().all(|k| k.iter().all(Option::is_some)),
            "{}",
            place.name
        );
    }
    let held: Vec<_> = work
        .bounties
        .iter()
        .flat_map(|p| &p.kinds)
        .flatten()
        .flatten()
        .filter(|c| c.held.is_some())
        .map(|c| c.id)
        .collect();
    assert_eq!(held, [4030]);
    assert_eq!(work.scrimmages.len(), 3);
    assert!(work.scrimmages.iter().all(|p| p.kinds.len() == 1));
    assert_eq!(
        work.scrimmages[0].kinds[0][0].as_ref().unwrap().held,
        Some(Count::Exact(5))
    );
}

#[test]
fn a_raise_from_lv1_takes_the_game_tables_exp() {
    let work = recognizer().work_rows(&[row(13, 1000)], &[], Some(100_000_000));
    let c = &work.commissions;
    assert_eq!(c.exp_each, 1_249_185);
    // Skills alone take 13,580,000 + 3 × 8,812,500 credits; the stars to the
    // weapon's 4th 5,750,000 and its Lv1 to 60 49,605 exp at 180.
    assert!(
        c.credits_each > 13_580_000 + 3 * 8_812_500 + 5_750_000 + 49_605 * 180,
        "{}",
        c.credits_each
    );
    assert!(c.report_exp > 0 && c.credit_students.is_some());
}
