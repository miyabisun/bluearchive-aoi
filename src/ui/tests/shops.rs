//! The shop tab: each shop folded with its coins on its title, its goods as tiles, and folds
//! that stay as the user left them while the game is read.

use gpui::TestAppContext;

use super::{
    app::{equipment, items, reading},
    click, has, keys, open, report, set, shows,
};
use crate::{
    icon::Rarity,
    keep::Kept,
    price::Price,
    quantity::Count,
    shop::{CoinRow, GoodRow, OfferRow, ShopRow},
    watch::{Screen, Status},
};

fn offer(left: u32, price: Option<(u32, u32)>) -> OfferRow {
    OfferRow {
        left,
        price: price.map(|(price, amount)| Price {
            price,
            amount,
            limit: left.max(1),
        }),
        shown: None,
        coin: 0,
    }
}

pub fn shops() -> Vec<ShopRow> {
    let coin = |name: &str, icon: &str, held: Option<Count>| CoinRow {
        name: name.to_owned(),
        icon: icon.to_owned(),
        held,
        at: 0,
    };
    vec![
        ShopRow {
            name: "総力戦".to_owned(),
            read_at: "2026-09-30 22:00:00".to_owned(),
            resets: true,
            coins: vec![
                coin(
                    "総力戦コイン",
                    "item_icon_raidcoin",
                    Some(Count::Exact(5940)),
                ),
                coin(
                    "総力戦レアコイン",
                    "item_icon_raidcoin_high",
                    Some(Count::Exact(530)),
                ),
            ],
            goods: vec![
                GoodRow {
                    id: 3000,
                    name: "初級戦術教育BD（百鬼夜行）".to_owned(),
                    icon: "item_icon_material_exskill_hyakkiyako_0".to_owned(),
                    offers: vec![
                        Some(offer(40, Some((10, 1)))),
                        Some(offer(12, Some((10, 1)))),
                    ],
                    sold: false,
                    rarity: Some(Rarity::N),
                    eleph: false,
                },
                GoodRow {
                    id: 10019,
                    name: "アズサの神名文字".to_owned(),
                    icon: "item_icon_secretstone_azusa".to_owned(),
                    offers: vec![None, Some(offer(0, Some((50, 5))))],
                    sold: true,
                    rarity: Some(Rarity::N),
                    eleph: true,
                },
            ],
        },
        ShopRow {
            name: "指名手配".to_owned(),
            read_at: "2999-01-01 08:30:00".to_owned(),
            resets: true,
            coins: vec![coin("指名手配コイン", "item_icon_chasercoin", None)],
            goods: vec![GoodRow {
                id: 160,
                name: "エーテルの粉".to_owned(),
                icon: "item_icon_material_ether_0".to_owned(),
                offers: vec![Some(offer(25, None))],
                sold: false,
                rarity: None,
                eleph: false,
            }],
        },
    ]
}

#[gpui::test]
fn shows_each_shop_s_goods_shop_by_shop(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = reading(Screen::Shops, 20.);
        s.items = items();
        s.equipment = equipment();
        s.shops = shops();
    });
    click(vcx, "tab:ショップ");
    assert!(shows(vcx, "ショップを集計中"), "the status");
    // Each shop names itself, when it was read, and the coins held.
    for words in [
        "総力戦",
        "09/30 22:00",
        "前の月の値",
        "5,940",
        "530",
        "指名手配",
        "01/01 08:30",
        "?",
    ] {
        assert!(shows(vcx, words), "a shop's title: {words}");
    }
    // Closed until opened.
    assert!(
        !has(vcx, "tile-総力戦-3000"),
        "the goods hide while it is closed"
    );
    click(vcx, "fold-shop-総力戦");
    assert!(has(vcx, "tile-総力戦-3000"), "opened, its goods");
    for words in ["×1", "40回@10", "12回@10", "×5", "売切れ"] {
        assert!(shows(vcx, words), "a good's tile: {words}");
    }
    click(vcx, "search");
    keys(vcx, "ｴ ｰ ﾃ ﾙ");
    assert!(!shows(vcx, "総力戦"), "a shop with no good found goes");
    assert!(shows(vcx, "25回@?"), "the good found, its shop open");
}

#[gpui::test]
fn a_shop_opened_stays_open_while_the_game_is_read(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = reading(Screen::Shops, 20.);
        s.items = items();
        s.shops = shops();
    });
    click(vcx, "tab:ショップ");
    click(vcx, "fold-shop-総力戦");
    // Every frame read sends a new snapshot; what the user opened stays open.
    for fps in [9., 8., 7.] {
        report(&core, vcx, &reading(Screen::Shops, fps), None);
    }
    assert!(has(vcx, "tile-総力戦-3000"), "still open");
}

#[gpui::test]
fn guides_to_the_shop_before_any_is_read(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = Status::OtherScreen;
        s.items = items();
    });
    click(vcx, "tab:ショップ");
    assert!(shows(vcx, crate::ui::shops::EMPTY), "how to read the shops");
}
