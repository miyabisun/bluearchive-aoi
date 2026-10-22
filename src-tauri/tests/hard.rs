//! The hard missions' elephs over the built-in data.

use std::{collections::BTreeSet, path::Path};

use aoi::{Recognizer, Row, quantity::Count, strategy::Reserve};

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap()
}

#[test]
fn each_students_eleph_shows_the_hard_missions_dropping_it() {
    let cards = recognizer().hard(&[], &[], &BTreeSet::new());
    assert_eq!(cards.len(), 44);
    let stages = |id: i64| cards.iter().find(|c| c.id == id).unwrap().stages.clone();
    // The user's examples: アル only at 18-3, ユウカ at 1, 4 and 12.
    assert_eq!(stages(10000), ["18-3"]);
    assert_eq!(stages(13010), ["1-1", "4-3", "12-2"]);
    let aru = cards.iter().find(|c| c.id == 10000).unwrap();
    assert_eq!(aru.ap, 20 * 3);
    // Those who come with three stars first: アリス, then アル by name.
    assert_eq!((cards[0].id, cards[1].id), (10015, 10000));
    // Never read, a student's star is unknown.
    assert_eq!(aru.star, None);
}

#[test]
fn a_hard_card_reserved_is_marked_and_counts_its_elephs_held() {
    let reserved = BTreeSet::from([Reserve {
        shop: "任務 Hard".into(),
        id: 10000,
        coin: "AP".into(),
    }]);
    let held = [Row {
        id: 10000,
        name: String::new(),
        icon: String::new(),
        tier: None,
        count: Count::Exact(80),
        rarity: None,
        eleph: true,
    }];
    let cards = recognizer().hard(&held, &[], &reserved);
    let aru = cards.iter().find(|c| c.id == 10000).unwrap();
    assert!(aru.reserved);
    assert_eq!(aru.held, Some(Count::Exact(80)));
    assert!(cards.iter().filter(|c| c.reserved).count() == 1);
}
