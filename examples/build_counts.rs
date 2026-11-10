//! Writes the references of the lists' counts (`data/counts.png` and
//! `data/counts.json`, a glyph bank: see `aoi::glyph::bank`): the "x" before a
//! count, its digits and "K", from two sources.
//!
//! - The fixtures labeled `"role": "tune"` in `labels.json`, as a window drawn
//!   at each of `WIDTHS` shows them: the widths read at and those between.
//! - A tablet's screens as scrcpy streams them (H.264, 1920 and 1600 wide),
//!   which blur the digits beyond any scaling: a collection of stops
//!   of the item and equipment lists, `<stop>-native.png` (the screen as
//!   screencap gives it) and `<stop>-<size>.png` (a frame of the stream) each.
//!   The native screen's counts, read with the fixtures' references, label
//!   the cards; the even-numbered stops are taken, the odd-numbered ones are
//!   left for evaluation (`tests/fixtures/tablet.json`). The collection
//!   (464 MB, collected on 2026-10-15) is kept outside the repository.
//!
//! The cuts of a character are grouped by likeness and each group averaged
//! (`bank::join`), as the other texts' are. Evaluation fixtures are never
//! used here.
//! Run with `cargo run --release --example build_counts -- <collection>`.

use std::{collections::BTreeMap, path::Path};

use aoi::{
    glyph::{
        Fonts,
        bank::{self, Bank, Group, glyph_tile, join},
    },
    grid, quantity, screen,
};
use image::{
    RgbImage,
    imageops::{FilterType, resize},
};
use serde::Deserialize;

/// The sizes the tablet's screens were streamed at that are read.
const STREAMED: [&str; 2] = ["1920", "1600"];
/// A card of two screens is the same one within this share of the screen.
const SAME_PLACE: f32 = 0.02;

/// The widths a window is drawn at for the references: the widths read at
/// (`screen::WIDTHS`), and between them every 20 pixels, wider every 60.
fn widths() -> impl Iterator<Item = u32> {
    (1500..=1920).step_by(20).chain((1980..=2160).step_by(60))
}

#[derive(Deserialize)]
struct ItemFixture {
    image: String,
    role: String,
    cells: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    quantity: Option<String>,
}

/// The groups of each character's cuts.
type Cuts = BTreeMap<char, Vec<Group>>;

/// Adds the cuts of the card's count, when it cuts into as many as `text`.
fn add(cuts: &mut Cuts, image: &RgbImage, card: &grid::Card, text: &str) {
    let Some((x, found)) = quantity::shapes(image, card) else {
        return;
    };
    if text.chars().count() != found.len() {
        return;
    }
    for (c, shape) in std::iter::once('x').zip([x]).chain(text.chars().zip(found)) {
        join(cuts.entry(c).or_default(), shape);
    }
}

/// The cuts as references.
fn fonts(cuts: &Cuts) -> Fonts {
    let chars = cuts
        .iter()
        .map(|(c, groups)| (*c, groups.iter().map(Group::mean).collect()));
    Fonts([(quantity::FONT.to_owned(), chars.collect())].into())
}

/// The cards found and the middle of each, as a share of the screen's width
/// and height.
fn cards(image: &RgbImage) -> Vec<(grid::Card, (f32, f32))> {
    let place = |c: &grid::Card| {
        (
            c.center_x() / image.width() as f32,
            (c.top as f32 + c.height as f32 / 2.0) / image.height() as f32,
        )
    };
    grid::find_cards(image)
        .into_iter()
        .map(|c| {
            let at = place(&c);
            (c, at)
        })
        .collect()
}

/// Of the `cards` of a screen, the one at `place` in another screen showing
/// the same cards: the two may count the rows cut off at the edges apart.
fn card_at(cards: &[(grid::Card, (f32, f32))], (x, y): (f32, f32)) -> Option<&grid::Card> {
    cards
        .iter()
        .map(|(card, (cx, cy))| (card, (cx - x).hypot(cy - y)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, far)| *far < SAME_PLACE)
        .map(|(card, _)| card)
}

/// The tuning fixtures as a window drawn at each of `widths` shows them, and
/// as Aoi reads it (`screen::normalize`).
fn windows(dir: &Path, cuts: &mut Cuts) {
    let labels: Vec<ItemFixture> = serde_json::from_str(
        &std::fs::read_to_string(dir.join("labels.json")).expect("the file reads"),
    )
    .expect("the JSON parses");
    for fixture in labels.iter().filter(|f| f.role == "tune") {
        let image = image::open(dir.join(&fixture.image))
            .expect("the image opens")
            .to_rgb8();
        // The labels follow the cards of the screen as it is.
        let own = cards(&screen::normalize(&image));
        assert_eq!(own.len(), fixture.cells.len(), "{}", fixture.image);
        for width in widths() {
            let scale = width as f32 / screen::drawn_width(&image);
            let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
            let drawn = resize(
                &image,
                w.round() as u32,
                h.round() as u32,
                FilterType::Triangle,
            );
            let drawn = screen::normalize(&drawn).into_owned();
            let found = cards(&drawn);
            for ((_, place), label) in own.iter().zip(&fixture.cells) {
                if let (Some(card), Some(text)) = (card_at(&found, *place), &label.quantity) {
                    add(cuts, &drawn, card, text);
                }
            }
        }
    }
}

/// The even-numbered stops of the tablet's collection (see the top), their
/// native screens' counts read with `fonts`.
fn tablet(dir: &Path, fonts: &Fonts, cuts: &mut Cuts) {
    let even = |name: &str| {
        name.rsplit('-')
            .next()
            .and_then(|k| k.parse::<u32>().ok())
            .is_some_and(|k| k % 2 == 0)
    };
    let mut stops: Vec<String> = std::fs::read_dir(dir)
        .expect("the folder lists")
        .filter_map(|e| {
            let name = e
                .expect("the folder entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned();
            name.strip_suffix("-native.png").map(str::to_owned)
        })
        .filter(|n| (n.starts_with("items-") || n.starts_with("equip-")) && even(n))
        .collect();
    stops.sort();
    assert!(!stops.is_empty(), "no stops in {}", dir.display());
    let open = |name: String| {
        let image = image::open(dir.join(name)).ok()?.to_rgb8();
        Some(screen::normalize(&image).into_owned())
    };
    for stop in &stops {
        let native =
            open(format!("{stop}-native.png")).expect("each stop has its native screencap");
        let labels: Vec<((f32, f32), String)> = cards(&native)
            .iter()
            .filter_map(|(card, place)| {
                Some((*place, quantity::read(fonts, &native, card)?.to_string()))
            })
            .collect();
        for size in STREAMED {
            let Some(image) = open(format!("{stop}-{size}.png")) else {
                continue;
            };
            let found = cards(&image);
            for (place, text) in &labels {
                if let Some(card) = card_at(&found, *place) {
                    add(cuts, &image, card, text);
                }
            }
        }
    }
}

fn main() {
    let collection = std::env::args()
        .nth(1)
        .expect("usage: build_counts <collection of the tablet's screens>");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut cuts = Cuts::new();
    windows(&root.join("tests/fixtures"), &mut cuts);
    let from_windows = fonts(&cuts);
    tablet(Path::new(&collection), &from_windows, &mut cuts);

    let mut bank = Bank::default();
    let mut tiles = BTreeMap::new();
    for (c, groups) in &cuts {
        let of: Vec<usize> = groups
            .iter()
            .map(|g| bank.add(glyph_tile(&g.mean())))
            .collect();
        tiles.insert(*c, of);
    }
    let counts: Vec<String> = tiles
        .iter()
        .map(|(c, t)| format!("{c}:{}", t.len()))
        .collect();
    bank.index.fonts.insert(quantity::FONT.to_owned(), tiles);
    let (png, index) = bank.encode().expect("the bank encodes");
    let (picture, index_file) = bank::COUNTS;
    std::fs::write(root.join("data").join(picture), png).expect("the file is written");
    std::fs::write(root.join("data").join(index_file), index + "\n").expect("the file is written");
    println!("{} tiles: {}", bank.tiles.len(), counts.join(" "));
}
