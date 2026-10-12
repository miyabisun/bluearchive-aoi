//! The shop screen against screenshots labeled by eye
//! (`tests/fixtures/shops.json`, masked to what is read). Labels list the
//! cards shown whole: from the name to the price.

use std::path::{Path, PathBuf};

use aoi::{List, Recognizer, shop::Offer};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    shop: String,
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

/// Every offer read is a labeled one with its labeled values (a price may be
/// unread, never wrong), and most labeled offers read in a single frame.
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
                price: Some(l.price),
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
        for offer in &seen.offers {
            let right = labels.iter().any(|l| {
                l.id == offer.id
                    && l.coin == offer.coin
                    && l.left == offer.left
                    && offer.price.is_none_or(|p| Some(p) == l.price)
            });
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
