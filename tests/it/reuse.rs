//! A list read again tells its cards apart only where they changed: a card drawn the same as
//! before is taken as it was read, so a list held still costs little, and reads the same.

use std::path::Path;

use aoi::{List, Recognizer, grid, screen};
use image::{Rgb, RgbImage};

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).expect("the data loads")
}

fn fixture(name: &str) -> RgbImage {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    screen::normalize(&image::open(path).expect("the fixture opens").to_rgb8()).into_owned()
}

#[test]
fn a_list_read_again_tells_no_card_apart_twice() {
    let recognizer = recognizer();
    let catalog = recognizer.catalog(List::Items);
    let image = fixture("items-16x9-list-top.png");
    let cards = grid::find_cards(&image).len();
    let start = catalog.cards_told_apart();
    let first = recognizer.read(&image, List::Items);
    let between = catalog.cards_told_apart();
    let again = recognizer.read(&image, List::Items);
    assert_eq!(
        between - start,
        cards,
        "every card told apart the first time"
    );
    assert_eq!(catalog.cards_told_apart(), between, "none the second");
    assert_eq!(again, first, "read the same");
}

#[test]
fn a_card_drawn_otherwise_is_told_apart_again_alone() {
    let recognizer = recognizer();
    let catalog = recognizer.catalog(List::Items);
    let image = fixture("items-16x9-list-top.png");
    let cards = grid::find_cards(&image);
    recognizer.read(&image, List::Items);
    let mut changed = image.clone();
    let card = cards[3];
    let (x, y) = (card.center_x() as u32, card.center_y() as u32);
    for dy in 0..6 {
        for dx in 0..6 {
            changed.put_pixel(x + dx, y + dy, Rgb([255, 0, 255]));
        }
    }
    let before = catalog.cards_told_apart();
    recognizer.read(&changed, List::Items);
    assert_eq!(
        catalog.cards_told_apart() - before,
        1,
        "only the card drawn otherwise"
    );
}

/// A card drawn as before, where it was, is not read again at all (its badge, its icon or its
/// count): of a list scrolling, nine cards in ten are the frame before's (measured on
/// 2026-11-10 over a recorded run of 60 frames: 792 of 880).
#[test]
fn a_list_read_again_reads_no_card_twice() {
    let recognizer = recognizer();
    let catalog = recognizer.catalog(List::Items);
    let image = fixture("items-16x9-list-top.png");
    let cards = grid::find_cards(&image).len();
    let start = catalog.cards_read();
    let first = recognizer.read(&image, List::Items);
    let between = catalog.cards_read();
    let again = recognizer.read(&image, List::Items);
    assert_eq!(between - start, cards, "every card read the first time");
    assert_eq!(catalog.cards_read(), between, "none the second");
    assert_eq!(again, first, "read the same");
}

/// A card whose count is drawn otherwise is read again, the others not; so is one whose
/// badge's rows or whose foot's slant, outside its own columns, changed.
#[test]
fn a_card_whose_count_changed_is_read_again() {
    let recognizer = recognizer();
    let catalog = recognizer.catalog(List::Items);
    let image = fixture("items-16x9-list-top.png");
    let cards = grid::find_cards(&image);
    recognizer.read(&image, List::Items);
    let card = cards[7];
    let foot = card.top + card.height - 4;
    // In the count's rows; left of the card's columns, where its slanted foot reaches; and
    // over its top, where a badge would hang.
    let spots = [
        ((card.left + card.width * 0.8) as u32, foot),
        ((card.left - 3.0) as u32, foot),
        (card.center_x() as u32, card.top - 2),
    ];
    for (x, y) in spots {
        let mut changed = image.clone();
        for d in 0..2 {
            changed.put_pixel(x + d, y, Rgb([45, 70, 99]));
        }
        let before = catalog.cards_read();
        recognizer.read(&changed, List::Items);
        let read = catalog.cards_read() - before;
        // A mark between cards moves where its row's cards are found: at most the row.
        assert!(
            (1..=5).contains(&read),
            "the card drawn otherwise at ({x}, {y}), and at most its row: {read}"
        );
    }
}
