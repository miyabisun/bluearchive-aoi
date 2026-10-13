//! The shop screen against screenshots labeled by eye
//! (`tests/fixtures/shops.json`, masked to what is read). Labels list the
//! cards shown whole: from the name to the price, which the shops' terms
//! (`data/shops.yaml`) must give, or the card itself where they leave it to it.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use aoi::{
    List, Recognizer,
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

/// Every offer read is a labeled one with its labeled values, and most labeled
/// offers read in a single frame.
#[test]
fn every_offer_read_is_right() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.catalog(List::Items).entries();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("shops.json")).unwrap())
            .unwrap();
    let (mut wrong, mut read, mut labeled) = (Vec::new(), 0, 0);
    for fixture in fixtures {
        let image = image::open(fixtures_dir().join(&fixture.image))
            .unwrap()
            .to_rgb8();
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
            wrong.push(format!("{}: no shop read", fixture.image));
            continue;
        };
        if seen.shop != fixture.shop {
            wrong.push(format!("{}: read as {}", fixture.image, seen.shop));
        }
        if let Some(held) = &fixture.held
            && seen.held != *held
        {
            wrong.push(format!("{}: held read {:?}", fixture.image, seen.held));
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
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    assert!(
        read * 4 >= labeled * 3,
        "{read} of {labeled} labeled offers read"
    );
}

/// The shops' terms give each labeled offer the price its card shows.
#[test]
fn the_terms_price_every_labeled_offer_as_its_card() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let items = recognizer.catalog(List::Items).entries();
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
            let rows = recognizer.shop_rows(&BTreeMap::from([(fixture.shop.clone(), shelf)]));
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
