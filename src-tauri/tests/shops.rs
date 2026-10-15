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

/// Every offer read is a labeled one with its labeled values, and over half
/// the labeled offers read in a single frame, the fixtures read as Aoi reads
/// them (scaled down to Full HD; 50 of 82 on 2026-10-15).
#[test]
fn every_offer_read_is_right() {
    let (wrong, read, labeled) = offers_read(None);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    assert!(
        read * 2 >= labeled,
        "{read} of {labeled} labeled offers read"
    );
}

/// A screen of any width read, and any between them (a window as it was
/// resized), reads every shop and offer right or not at all.
#[test]
fn a_smaller_screen_reads_offers_right_or_not_at_all() {
    for width in [2222, 2100, 1920, 1760, 1700, 1600, 1550, 1500] {
        let (wrong, ..) = offers_read(Some(width));
        assert!(wrong.is_empty(), "{width}:\n{}", wrong.join("\n"));
    }
}

/// What the shop fixtures read, scaled to `width` if given: what was read
/// wrong, how many offers were read right, and how many are labeled.
fn offers_read(width: Option<u32>) -> (Vec<String>, usize, usize) {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.shop_goods();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("shops.json")).unwrap())
            .unwrap();
    let (mut wrong, mut read, mut labeled) = (Vec::new(), 0, 0);
    for fixture in fixtures {
        let image = image::open(fixtures_dir().join(&fixture.image))
            .unwrap()
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
