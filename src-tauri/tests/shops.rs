//! The shop screen against screenshots labeled by eye
//! (`tests/fixtures/shops.json`, masked to what is read). Labels list the
//! cards shown whole: from the name to the price, which the shops' terms
//! (`data/shops.yaml`) must give, or the card itself where they leave it to it.

mod common;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use aoi::{
    Recognizer,
    icon::Rarity,
    shop::{Offer, Shelf},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    shop: String,
    /// The coins held, left one first, where the fixture shows them.
    #[serde(default)]
    held: Option<Vec<Option<u64>>>,
    offers: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    name: String,
    coin: u8,
    left: u32,
    price: u32,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Every offer read is a labeled one with its labeled values, and most
/// labeled offers read in a single frame, the fixtures read as Aoi reads them
/// (scaled down to Full HD; 104 of 105 on 2026-10-16).
#[test]
fn every_offer_read_is_right() {
    let (wrong, read, labeled) = offers_read(None);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    assert!(
        read * 20 >= labeled * 19,
        "{read} of {labeled} labeled offers read"
    );
}

/// A screen of any width read, and any between them (a window as it was
/// resized), reads every shop and offer right or not at all, and at least
/// `LEAST` of the labeled offers (measured on 2026-10-16: 74 to 104 of 105).
#[test]
fn a_smaller_screen_reads_offers_right_or_not_at_all() {
    const LEAST: f32 = 0.7;
    for width in [2222, 2100, 1920, 1760, 1700, 1600, 1550, 1500] {
        let (wrong, read, labeled) = offers_read(Some(width));
        assert!(wrong.is_empty(), "{width}:\n{}", wrong.join("\n"));
        assert!(
            read as f32 >= LEAST * labeled as f32,
            "{width}: {read} of {labeled} offers read"
        );
    }
}

/// What the shop fixtures read, scaled to `width` if given: what was read
/// wrong, how many offers were read right, and how many are labeled.
fn offers_read(width: Option<u32>) -> (Vec<String>, usize, usize) {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
        .expect("the recognizer loads data/");
    let items = recognizer.shop_goods();
    let fixtures: Vec<Fixture> = serde_json::from_str(
        &std::fs::read_to_string(fixtures_dir().join("shops.json")).expect("the file reads"),
    )
    .expect("the JSON parses");
    let (mut wrong, mut read, mut labeled) = (Vec::new(), 0, 0);
    for fixture in fixtures {
        let image = image::open(fixtures_dir().join(&fixture.image))
            .expect("the image opens")
            .to_rgb8();
        let image = match width {
            Some(width) => common::drawn_at(&image, width),
            None => image,
        };
        let image = aoi::screen::normalize(&image).into_owned();
        let shown = recognizer.reads_price(&fixture.shop);
        let labels: Vec<Offer> = fixture
            .offers
            .iter()
            .map(|l| Offer {
                id: items
                    .iter()
                    .find(|i| i.name == l.name)
                    .unwrap_or_else(|| panic!("no item {}", l.name))
                    .id,
                coin: l.coin,
                left: l.left,
                price: shown.then_some(l.price),
            })
            .collect();
        labeled += labels.len();
        let Some(seen) = recognizer.read_shop(&image) else {
            // A window reads every shop; a smaller screen may read none.
            if width.is_none() {
                wrong.push(format!("{}: no shop read", fixture.image));
            }
            continue;
        };
        if seen.shop != fixture.shop {
            wrong.push(format!("{}: read as {}", fixture.image, seen.shop));
        }
        if let Some(held) = &fixture.held {
            // Unread is no wrong count.
            let held_wrong = seen.held.len() != held.len()
                || seen
                    .held
                    .iter()
                    .zip(held)
                    .any(|(s, h)| s.is_some() && s != h);
            if held_wrong {
                wrong.push(format!("{}: held read {:?}", fixture.image, seen.held));
            }
        }
        for offer in &seen.offers {
            let right = labels.contains(offer);
            if right {
                read += 1;
            } else {
                wrong.push(format!("{}: read {offer:?}", fixture.image));
            }
        }
    }
    (wrong, read, labeled)
}

/// The shops' terms give each labeled offer the price its card shows.
#[test]
fn the_terms_price_every_labeled_offer_as_its_card() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.shop_goods();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("shops.json")).unwrap())
            .unwrap();
    let mut wrong = Vec::new();
    for fixture in fixtures {
        let shown = recognizer.reads_price(&fixture.shop);
        for label in &fixture.offers {
            let id = items.iter().find(|i| i.name == label.name).unwrap().id;
            let shelf = Shelf {
                read_at: String::new(),
                offers: vec![Offer {
                    id,
                    coin: label.coin,
                    left: label.left,
                    price: shown.then_some(label.price),
                }],
                held: Vec::new(),
            };
            let rows = recognizer.shop_rows(
                &BTreeMap::from([(fixture.shop.clone(), shelf)]),
                &std::collections::HashMap::new(),
            );
            let price = rows[0].goods[0]
                .offers
                .iter()
                .flatten()
                .find(|o| o.coin == label.coin)
                .and_then(|o| o.price)
                .map(|p| p.price);
            if price != Some(label.price) {
                wrong.push(format!(
                    "{} {}: {price:?} for {}",
                    fixture.shop, label.name, label.price
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// A shop lists what is worth buying first (the user's order of 2026-10-13):
/// 秘伝ノート, 神名のカケラ, the elephs (those of the plain coin first, then by
/// how the student is had: fest, limited, an event's last), テイラーストーン, BDs,
/// notes, ooparts, the goods of no such kind, チタン撃針, credits, then reports,
/// booster tickets and blueprint boxes; the goods bought up go last.
#[test]
fn a_shop_lists_its_goods_worth_buying_first_and_those_bought_up_last() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.shop_goods();
    let offer = |name: &str, coin: u8, left: u32| Offer {
        id: items.iter().find(|i| i.name == name).unwrap().id,
        coin,
        left,
        price: None,
    };
    let listed = |shop: &str, offers: Vec<Offer>| {
        let shelf = Shelf {
            read_at: String::new(),
            offers,
            held: Vec::new(),
        };
        let rows = recognizer.shop_rows(
            &BTreeMap::from([(shop.to_owned(), shelf)]),
            &std::collections::HashMap::new(),
        );
        rows[0]
            .goods
            .iter()
            .map(|g| format!("{}{}", g.name, if g.sold { " 売切れ" } else { "" }))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        listed(
            "熟達証書",
            vec![
                offer("T3装備設計図選択ボックス", 0, 100),
                offer("最上級レポート", 0, 4),
                offer("ノノミの神名文字", 0, 6),
                offer("アズサ（水着）の神名文字", 0, 2),
                offer("ミカの神名文字", 0, 0),
                offer("ワカモの神名文字", 0, 2),
                offer("クレジットポイント", 0, 4),
                offer("チタン撃針", 0, 2),
                offer("神名のカケラ", 0, 5),
            ]
        ),
        [
            "神名のカケラ",
            "ワカモの神名文字",
            "アズサ（水着）の神名文字",
            "ノノミの神名文字",
            "チタン撃針",
            "クレジットポイント",
            "最上級レポート",
            "T3装備設計図選択ボックス",
            "ミカの神名文字 売切れ",
        ]
    );
    // 総力戦's coins from the left: the rare, then the plain.
    assert_eq!(
        listed(
            "総力戦",
            vec![
                offer("ブースターチケット", 1, 5),
                offer("エーテルの粉", 1, 5),
                offer("初級技術ノート（百鬼夜行）", 0, 16),
                offer("初級技術ノート（百鬼夜行）", 1, 50),
                offer("初級戦術教育BD（百鬼夜行）", 0, 0),
                offer("初級戦術教育BD（百鬼夜行）", 1, 40),
                offer("マキの神名文字", 0, 10),
                offer("ミドリの神名文字", 1, 20),
                offer("秘伝ノート", 1, 3),
            ]
        ),
        [
            "秘伝ノート",
            "ミドリの神名文字",
            "マキの神名文字",
            "初級戦術教育BD（百鬼夜行）",
            "初級技術ノート（百鬼夜行）",
            "エーテルの粉",
            "ブースターチケット",
        ]
    );
}

/// A good tells its rarity, which the game shows in the colour behind its icon
/// (初級 N, 中級 R, 上級 SR, 最上級 SSR), and whether it is a student's eleph;
/// its month-end cards tell the same.
#[test]
fn a_good_tells_its_rarity_and_whether_it_is_an_eleph() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.shop_goods();
    let expected: BTreeMap<String, (Option<Rarity>, bool)> = [
        ("初級テイラーストーン", Rarity::N, false),
        ("中級テイラーストーン", Rarity::R, false),
        ("上級テイラーストーン", Rarity::SR, false),
        ("最上級テイラーストーン", Rarity::SSR, false),
        ("ミドリの神名文字", Rarity::N, true),
    ]
    .into_iter()
    .map(|(name, rarity, eleph)| (name.to_owned(), (Some(rarity), eleph)))
    .collect();
    let offers = expected
        .keys()
        .map(|name| Offer {
            id: items.iter().find(|i| &i.name == name).unwrap().id,
            coin: 0,
            left: 1,
            price: None,
        })
        .collect();
    let shelf = Shelf {
        read_at: String::new(),
        offers,
        held: Vec::new(),
    };
    let rows = recognizer.shop_rows(
        &BTreeMap::from([("合同火力演習".to_owned(), shelf)]),
        &std::collections::HashMap::new(),
    );
    let goods: BTreeMap<String, (Option<Rarity>, bool)> = rows[0]
        .goods
        .iter()
        .map(|g| (g.name.clone(), (g.rarity, g.eleph)))
        .collect();
    assert_eq!(goods, expected);
    let plan = recognizer.month_end(&rows, &[], |_| None, &std::collections::BTreeSet::new());
    let cards: BTreeMap<String, (Option<Rarity>, bool)> = plan
        .iter()
        .find(|s| s.name == "合同火力演習")
        .unwrap()
        .cards
        .iter()
        .map(|c| (c.name.clone(), (c.rarity, c.eleph)))
        .collect();
    assert_eq!(cards, expected);
}

/// The chosen menu entry is read again whenever its letters change, though the
/// cards stay: a tap's sparkle over the entry hides the shop's name, and the
/// name reads once the sparkle is gone (the user's of 2026-10-18: 「変わってるのに
/// 変わらないとサボっている」).
#[test]
fn the_menu_entry_reads_again_when_its_letters_change_though_the_cards_stay() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let recognizer = Recognizer::load(&data).unwrap();
    let image = image::open(fixtures_dir().join("shops/jukutatsu-credits.png"))
        .unwrap()
        .to_rgb8();
    let image = aoi::screen::normalize(&image).into_owned();
    // The chosen entry's rows, mostly dark at the menu's left, with their
    // letters painted over in the entry's navy, as a sparkle may hide them.
    let mut unlettered = image.clone();
    let menu = (0.30 * f64::from(image.height())) as u32;
    let luminance =
        |c: [u8; 3]| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2]);
    let dark = |c: [u8; 3]| luminance(c) < 90.0;
    let light = |c: [u8; 3]| luminance(c) > 110.0;
    for y in 0..image.height() {
        let row = |x| image.get_pixel(x, y).0;
        if (0..menu).filter(|&x| dark(row(x))).count() * 2 < menu as usize {
            continue;
        }
        let navy = row(2);
        for x in 0..menu {
            if light(row(x)) {
                unlettered.put_pixel(x, y, image::Rgb(navy));
            }
        }
    }
    assert_eq!(recognizer.read_shop(&unlettered), None);
    let seen = recognizer
        .read_shop(&image)
        .expect("the shop reads once its entry does");
    assert_eq!(seen.shop, "熟達証書");
    assert_eq!(recognizer.read_shop(&unlettered), None);
}

/// A still screen's card panel stays the same to the pixel, and a frame whose
/// panel is the last one's reads as it did at a fraction of the time; any
/// other frame is read afresh.
#[test]
fn a_frame_like_the_last_reads_as_it_did_and_another_afresh() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let (recognizer, fresh) = (
        Recognizer::load(&data).unwrap(),
        Recognizer::load(&data).unwrap(),
    );
    // As Aoi reads them, scaled down to a width read at.
    let open = |name: &str| {
        let image = image::open(fixtures_dir().join("shops").join(name))
            .unwrap()
            .to_rgb8();
        aoi::screen::normalize(&image).into_owned()
    };
    let (one, other) = (open("jukutatsu-credits.png"), open("daikessen.png"));
    let timed = |image: &image::RgbImage| {
        let at = std::time::Instant::now();
        (recognizer.read_shop(image), at.elapsed())
    };
    let (first, took) = timed(&one);
    assert!(first.is_some());
    // The quickest of a few, as a busy machine slows any one.
    let retook = (0..3)
        .map(|_| {
            let (again, retook) = timed(&one);
            assert_eq!(again, first);
            retook
        })
        .min()
        .unwrap();
    assert!(retook * 2 < took, "{retook:?} again, {took:?} first");
    assert_eq!(recognizer.read_shop(&other), fresh.read_shop(&other));
    assert_eq!(recognizer.read_shop(&one), first);
}
