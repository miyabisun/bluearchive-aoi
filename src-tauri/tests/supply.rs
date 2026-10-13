//! What could make up what the plan falls short of, over the built-in data:
//! the choice boxes held, the fragments for elephs, and this month's shops.

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use aoi::{
    Recognizer, Row,
    plan::{Held, Kind, NeedRow},
    quantity::Count,
    shop::{Offer, Shelf},
    supply::Source,
};

fn row(id: i64, count: u32) -> Row {
    Row {
        id,
        name: String::new(),
        icon: String::new(),
        tier: None,
        count: Count::Exact(count),
    }
}

/// A line of the plan for the item `id`: `need` of it, `have` held.
fn line(id: i64, need: u64, have: u64) -> NeedRow {
    NeedRow {
        kind: Kind::Item,
        id,
        name: String::new(),
        icon: String::new(),
        family: None,
        need,
        have: Some(Held::Exact(have)),
    }
}

/// A source as one line of text, to compare.
fn text(source: &Source) -> String {
    match source {
        Source::Box {
            id, held, choices, ..
        } => format!("box {id} {held} of {choices}"),
        Source::Shop {
            shop,
            coin,
            held,
            price,
            gives,
            left,
            ..
        } => format!("{shop} {coin} {held:?} @{price} x{gives} {left}"),
        Source::Fragments { .. } => "fragments".to_owned(),
    }
}

#[test]
fn a_line_short_lists_its_boxes_fragments_and_this_months_shops() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let id = |name: &str| {
        recognizer
            .shop_goods()
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("no {name}"))
            .id
    };
    let (notes, mutsuki) = (
        id("最上級技術ノート（ゲヘナ）"),
        id("ムツキ（ドレス）の神名文字"),
    );
    let plan = [line(notes, 53, 24), line(mutsuki, 420, 34)];
    let offer = |name: &str, coin: u8, left: u32, price: Option<u32>| Offer {
        id: id(name),
        coin,
        left,
        price,
    };
    let shelf = |read_at: &str, offers: Vec<Offer>| Shelf {
        read_at: read_at.to_owned(),
        offers,
        held: Vec::new(),
    };
    // 総力戦's coins from the left: the rare, then the plain.
    let shelves = BTreeMap::from([
        (
            "総力戦".to_owned(),
            shelf(
                "2026-10-13 12:00:00",
                vec![
                    offer("最上級技術ノート（ゲヘナ）", 0, 6, None),
                    offer("最上級技術ノート（ゲヘナ）", 1, 20, None),
                ],
            ),
        ),
        (
            "指名手配".to_owned(),
            shelf(
                "2026-10-13 12:00:00",
                vec![offer("最上級技術ノート（ゲヘナ）", 0, 4, None)],
            ),
        ),
        // Read last month: its stock has come back since, unknown.
        (
            "大決戦".to_owned(),
            shelf(
                "2026-09-30 12:00:00",
                vec![offer("最上級技術ノート（ゲヘナ）", 1, 20, None)],
            ),
        ),
        (
            "カケラ(文字)".to_owned(),
            shelf(
                "2026-10-01 12:00:00",
                vec![offer("ムツキ（ドレス）の神名文字", 0, 586, Some(1))],
            ),
        ),
    ]);
    let items_held = [
        row(id("最上級技術ノート選択ボックス"), 24),
        row(id("神名のカケラ"), 2941),
        row(id("総力戦コイン"), 5940),
        row(id("総力戦レアコイン"), 530),
    ];
    let held: HashMap<i64, Count> = items_held.iter().map(|r| (r.id, r.count)).collect();
    let shops = recognizer.shop_rows(&shelves, &held);
    let lines = recognizer.procure_rows(&plan, &items_held, &shops, "2026-10-13 20:00:00");
    assert_eq!(
        lines.iter().map(|l| (l.id, l.short)).collect::<Vec<_>>(),
        [(notes, 29), (mutsuki, 386)]
    );
    // The notes: the box of the few schools with Gehenna first, then any school's (the one
    // held), then 総力戦 plain and rare, then 指名手配; 大決戦 is last month's.
    let sources: Vec<String> = lines[0].sources.iter().map(text).collect();
    assert_eq!(
        sources,
        [
            "box 150015 0 of 6",
            "box 150003 24 of 12",
            "総力戦 総力戦コイン Some(5940) @25 x1 20",
            "総力戦 総力戦レアコイン Some(530) @25 x1 6",
            "指名手配 指名手配コイン None @200 x1 4",
        ]
    );
    // The elephs: from the fragments held, at the rate the shop showed.
    assert_eq!(
        lines[1].sources,
        [Source::Fragments {
            name: "神名のカケラ".to_owned(),
            icon: "item_icon_secretstone".to_owned(),
            held: 2941,
            rate: 1,
            left: 586,
        }]
    );
}
