//! The item and equipment tabs' tables by kind over the built-in data.

use std::path::Path;

use aoi::{Recognizer, Row, quantity::Count};

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
        .expect("the recognizer loads data/")
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
fn the_skill_materials_come_by_school_the_three_great_first() {
    let kinds = recognizer().kinds(&[row(9999, 3), row(3021, 40)], &[]);
    let secret: Vec<_> = kinds.secret.iter().map(|c| (c.id, c.held)).collect();
    assert_eq!(secret, [(9999, Some(Count::Exact(3))), (9998, None)]);
    let schools: Vec<_> = kinds.schools.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        schools[..11],
        [
            "トリニティ",
            "ゲヘナ",
            "ミレニアム",
            "百鬼夜行",
            "アビドス",
            "山海経",
            "レッドウィンター",
            "ヴァルキューレ",
            "アリウス",
            "ハイランダー",
            "ワイルドハント",
        ]
    );
    for school in &kinds.schools {
        assert!(
            school.bd.iter().chain(&school.notes).all(Option::is_some),
            "{}",
            school.name
        );
    }
    let trinity = &kinds.schools[0];
    assert_eq!(trinity.icon, "school_icon_trinity");
    let bd = trinity.bd[1].as_ref().unwrap();
    assert_eq!((bd.id, bd.held), (3021, Some(Count::Exact(40))));
    // What one student takes of it colours it, as the bounties' drops.
    assert!(bd.need.is_some());
}

/// The ids of a row's tiers.
fn ids(tiers: &[Option<aoi::work::HeldCell>]) -> Vec<Option<i64>> {
    tiers.iter().map(|c| c.as_ref().map(|c| c.id)).collect()
}

#[test]
fn the_skill_materials_choice_boxes_come_by_the_schools_they_choose_from_every_school_first() {
    let kinds = recognizer().kinds(&[row(150_000, 830), row(150_020, 5)], &[]);
    let sets: Vec<(bool, Vec<&str>)> = kinds
        .skill_boxes
        .iter()
        .map(|b| (b.all, b.schools.iter().map(String::as_str).collect()))
        .collect();
    // The game's three sets of the same names: every school, then two of six
    // schools each, the one with トリニティ first; each in the order of the schools.
    assert_eq!(sets.len(), 3);
    assert!(sets[0].0 && sets[0].1.len() == 12);
    assert_eq!(
        sets[1..],
        [
            (
                false,
                vec![
                    "トリニティ",
                    "ゲヘナ",
                    "山海経",
                    "ヴァルキューレ",
                    "アリウス",
                    "ワイルドハント"
                ]
            ),
            (
                false,
                vec![
                    "ミレニアム",
                    "百鬼夜行",
                    "アビドス",
                    "レッドウィンター",
                    "ハイランダー",
                    "オデュッセイア"
                ]
            ),
        ]
    );
    // Each set as a school's: its BD boxes over its tech note boxes, by tier.
    let every = &kinds.skill_boxes[0];
    let tiers = |first: i64| (first..first + 4).map(Some).collect::<Vec<_>>();
    assert_eq!(ids(&every.bd), tiers(150_004));
    assert_eq!(ids(&every.notes), tiers(150_000));
    assert_eq!(
        every.notes[0].as_ref().unwrap().held,
        Some(Count::Exact(830))
    );
    let six = &kinds.skill_boxes[1];
    assert_eq!(ids(&six.bd), tiers(150_020));
    assert_eq!(ids(&six.notes), tiers(150_012));
    assert_eq!(six.bd[0].as_ref().unwrap().held, Some(Count::Exact(5)));
}

#[test]
fn the_coins_come_in_the_item_lists_order_without_the_events_goods() {
    let kinds = recognizer().kinds(&[row(7, 1160), row(80_001, 12)], &[]);
    let coins: Vec<_> = kinds.coins.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        coins,
        [
            "総力戦コイン",
            "戦術コイン",
            "総力戦レアコイン",
            "神名のカケラ",
            "火力演習コイン",
            "指名手配コイン",
            "大決戦コイン",
            "大決戦レアコイン",
            "アーカイブポイント",
        ]
    );
    assert_eq!(kinds.coins[0].held, Some(Count::Exact(1160)));
    assert_eq!(kinds.coins[1].held, None);
}

#[test]
fn each_oopart_comes_with_its_four_tiers() {
    let kinds = recognizer().kinds(&[row(150_008, 672)], &[]);
    assert_eq!(kinds.ooparts.len(), 20);
    assert!(kinds.ooparts.iter().flatten().all(Option::is_some));
    assert_eq!(kinds.ooparts[0][0].as_ref().unwrap().id, 100);
    // Their choice boxes by tier, (1) to (4).
    let boxes = &kinds.oopart_boxes;
    assert_eq!(ids(boxes), (150_008..150_012).map(Some).collect::<Vec<_>>());
    assert_eq!(boxes[0].as_ref().unwrap().held, Some(Count::Exact(672)));
}

#[test]
fn the_enhancement_materials_come_five_kinds_of_four_tiers() {
    let kinds = recognizer().kinds(&[], &[row(4, 1383)]);
    let names: Vec<Vec<&str>> = kinds
        .enhancements
        .iter()
        .map(|r| r.tiers.iter().flatten().map(|c| c.name.as_str()).collect())
        .collect();
    assert_eq!(
        names,
        [
            ["初級強化珠", "中級強化珠", "上級強化珠", "最上級強化珠"],
            [
                "錆びたスプリング",
                "普通のスプリング",
                "クロムスプリング",
                "チタンスプリング"
            ],
            [
                "錆びたハンマー",
                "普通のハンマー",
                "クロムハンマー",
                "チタンハンマー"
            ],
            ["錆びた銃身", "普通の銃身", "クロム銃身", "チタン銃身"],
            ["錆びた撃針", "普通の撃針", "クロム撃針", "チタン撃針"],
        ]
    );
    let top = kinds.enhancements[0].tiers[3].as_ref().unwrap();
    assert_eq!(top.held, Some(Count::Exact(1383)));
}

#[test]
fn the_enhancement_materials_raise_as_many_students_as_the_work_tab_counts() {
    // Orbs, and every kind of parts at each tier.
    let equipment: Vec<Row> = [1, 2, 3, 4]
        .into_iter()
        .chain([
            10, 11, 12, 13, 20, 21, 22, 23, 30, 31, 32, 33, 40, 41, 42, 43,
        ])
        .map(|id| row(id, 37 * u32::try_from(id).unwrap()))
        .collect();
    let recognizer = recognizer();
    let kinds = recognizer.kinds(&[], &equipment);
    let work = recognizer.work_rows(&[], &equipment, None);
    // The work tab shows the orbs as their row here, found by its category.
    assert_eq!(kinds.enhancements[0].category, "Exp");
    // Each scrimmage drops one kind of parts: its row raises as many weapons.
    for (place, school) in work.scrimmages.iter().zip(&work.schools) {
        let first = place.kinds[0][0].as_ref().unwrap().id;
        let row = kinds
            .enhancements
            .iter()
            .find(|e| e.tiers[0].as_ref().is_some_and(|c| c.id == first))
            .unwrap();
        assert_eq!(
            (row.raise.exp, row.raise.rate, row.raise.exp_each),
            (school.exp, school.rate, school.exp_each),
            "{}",
            place.name
        );
        assert!((row.raise.students - school.students).abs() < 1e-9);
    }
    // The pins suit every weapon alike: no bonus.
    let pins = &kinds.enhancements[4];
    assert_eq!(pins.category, "WeaponExpGrowthZ");
    assert!((pins.raise.rate - 1.0).abs() < f64::EPSILON);
    assert!(pins.raise.students > 0.0);
}

#[test]
fn the_blueprints_choice_boxes_come_by_the_tier_they_give() {
    let kinds = recognizer().kinds(&[row(150_030, 732)], &[]);
    // T1 to T10 as the slots' blueprints line up; no box gives T1.
    let ids: Vec<_> = ids(&kinds.print_boxes);
    let tiers = [
        150_028, 150_029, 150_030, 150_031, 150_032, 150_033, 150_041, 150_045, 150_048,
    ];
    assert_eq!(
        ids,
        std::iter::once(None)
            .chain(tiers.into_iter().map(Some))
            .collect::<Vec<_>>()
    );
    let t4 = kinds.print_boxes[3].as_ref().unwrap();
    assert_eq!(
        (t4.name.as_str(), t4.held),
        ("T4装備設計図選択ボックス", Some(Count::Exact(732)))
    );
}

#[test]
fn each_slot_shows_its_t1_piece_and_blueprints_to_t10_and_its_universal_ones() {
    let kinds = recognizer().kinds(&[], &[row(1000, 249), row(501_000, 51)]);
    let slots: Vec<_> = kinds.blueprints.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slots.len(), 9);
    assert_eq!(slots[0], "帽子");
    for slot in &kinds.blueprints {
        assert_eq!(slot.tiers.len(), 10, "{}", slot.name);
        assert!(slot.tiers.iter().all(Option::is_some), "{}", slot.name);
    }
    let hat = &kinds.blueprints[0];
    let held = |c: &Option<aoi::work::HeldCell>| c.as_ref().unwrap().held;
    assert_eq!(held(&hat.tiers[0]), Some(Count::Exact(249)));
    assert_eq!(held(&hat.universal), Some(Count::Exact(51)));
}
