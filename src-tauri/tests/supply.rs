//! What to buy for what falls short, over the built-in shops' terms: the
//! shops' stock and the coins held as read, the user's order of shops.

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use aoi::{
    List, Recognizer,
    plan::Kind,
    quantity::Count,
    shop::{Offer, Shelf},
    supply::ShortRow,
};

fn short(recognizer: &Recognizer, name: &str, n: u64) -> ShortRow {
    let item = recognizer
        .shop_goods()
        .iter()
        .find(|i| i.name == name)
        .unwrap();
    ShortRow {
        kind: Kind::Item,
        id: item.id,
        name: item.name.clone(),
        icon: item.icon.clone(),
        short: n,
        boxes: Vec::new(),
        left: n,
    }
}

#[test]
fn the_shops_sell_what_is_short_in_the_users_order_as_far_as_the_coins_go() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let id = |name: &str| {
        recognizer
            .shop_goods()
            .iter()
            .find(|i| i.name == name)
            .unwrap()
            .id
    };
    let offer = |name: &str, coin: u8, left: u32| Offer {
        id: id(name),
        coin,
        left,
        price: None,
    };
    let shelf = |offers: Vec<Offer>| Shelf {
        read_at: "2026-10-13 12:00:00".to_owned(),
        offers,
        held: Vec::new(),
    };
    // 総力戦's coins from the left: the rare, then the plain.
    let shelves = BTreeMap::from([
        (
            "総力戦".to_owned(),
            shelf(vec![
                offer("最上級技術ノート（ゲヘナ）", 0, 6),
                offer("最上級技術ノート（ゲヘナ）", 1, 20),
                offer("上級戦術教育BD（ゲヘナ）", 1, 25),
            ]),
        ),
        (
            "指名手配".to_owned(),
            shelf(vec![
                offer("最上級技術ノート（ゲヘナ）", 0, 4),
                offer("上級戦術教育BD（ゲヘナ）", 0, 4),
            ]),
        ),
    ]);
    let items = recognizer.catalog(List::Items).entries();
    let coin = |name: &str| items.iter().find(|i| i.name == name).unwrap().id;
    let held = HashMap::from([
        (coin("総力戦コイン"), Count::Exact(700)),
        (coin("総力戦レアコイン"), Count::Exact(530)),
        (coin("指名手配コイン"), Count::Exact(5000)),
    ]);
    let shops = recognizer.shop_rows(&shelves, &held);
    let shorts = [
        short(&recognizer, "最上級技術ノート（ゲヘナ）", 29),
        short(&recognizer, "上級戦術教育BD（ゲヘナ）", 6),
    ];
    let plan = recognizer.buy_plan(&shorts, &shops, "2026-10-13 20:00:00");
    // 総力戦 alone, in its order (BDs before notes): 6 BDs at 50 plain coins, 16
    // notes with the 400 left at 25, then 6 at 25 rare. 指名手配 sells notes and
    // BDs the user does not buy there.
    assert_eq!(plan.shops.len(), 1);
    let bought: Vec<(String, usize, u64, u64)> = plan.shops[0]
        .goods
        .iter()
        .map(|g| (g.name.clone(), g.coin, g.times, g.cost))
        .collect();
    assert_eq!(
        bought,
        [
            ("上級戦術教育BD（ゲヘナ）".to_owned(), 0, 6, 300),
            ("最上級技術ノート（ゲヘナ）".to_owned(), 0, 16, 400),
            ("最上級技術ノート（ゲヘナ）".to_owned(), 1, 6, 150),
        ]
    );
    assert_eq!(
        plan.shops[0]
            .coins
            .iter()
            .map(|c| (c.name.clone(), c.held, c.spent))
            .collect::<Vec<_>>(),
        [
            ("総力戦コイン".to_owned(), Some(700), 700),
            ("総力戦レアコイン".to_owned(), Some(530), 150),
        ]
    );
    assert_eq!(
        plan.left
            .iter()
            .map(|l| (l.name.clone(), l.left))
            .collect::<Vec<_>>(),
        [("最上級技術ノート（ゲヘナ）".to_owned(), 7)]
    );
    assert_eq!(plan.unread, ["大決戦", "合同火力演習", "熟達証書"]);
    // A shop read last month is not counted: its stock has come back since.
    let stale = recognizer.buy_plan(&shorts, &shops, "2026-11-01 08:00:00");
    assert!(stale.shops.is_empty());
    assert_eq!(stale.left.len(), 2);
}
