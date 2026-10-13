//! The work tab over the built-in data: the blueprints a piece from T1 to T10
//! takes, what the daily stages drop, and a raise from Lv1.

use std::path::Path;

use aoi::{Recognizer, Row, quantity::Count};

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
    assert!(hat.top.best.is_some() && hat.universal.best.is_some());
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
