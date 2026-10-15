//! Writes `data/digits.json`: the average glyph of each character of a count
//! (the "x" before it, the digits and "K"), from two sources.
//!
//! - The fixtures labeled `"role": "tune"` in `labels.json`, scaled to each of
//!   `WIDTHS`: a window's screen, drawn smaller.
//! - A tablet's screens as scrcpy streams them (H.264, 1920 and 1600 wide),
//!   which blur the digits beyond any scaling: a collection of stops
//!   of the item and equipment lists, `<stop>-native.png` (the screen as
//!   screencap gives it) and `<stop>-<size>.png` (a frame of the stream) each.
//!   The native screen, read as it is, labels the cards; the even-numbered
//!   stops give an average for each stop and size, the odd-numbered ones are
//!   left for evaluation (`tests/fixtures/tablet.json`). The collection
//!   (464 MB, collected on 2026-10-15) is kept outside the repository.
//!
//! Evaluation fixtures are never used here.
//! Run with `cargo run --release --example build_digits -- <collection>`.

use std::{collections::BTreeMap, path::Path};

use aoi::{Recognizer, grid, quantity, screen};
use image::{
    RgbImage,
    imageops::{FilterType, resize},
};
use serde::Deserialize;

const CHARACTERS: &str = "x0123456789K";
/// The widths the list is read at (`screen::WIDTHS`).
const WIDTHS: [u32; 3] = aoi::screen::WIDTHS;
/// The sizes the tablet's screens were streamed at that are read.
const STREAMED: [&str; 2] = ["1920", "1600"];

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

/// The sum of each character's glyphs and how many.
type Sums = BTreeMap<char, (Vec<f32>, u32)>;

fn sums() -> Sums {
    let size = quantity::GLYPH.0 * quantity::GLYPH.1;
    CHARACTERS
        .chars()
        .map(|c| (c, (vec![0.0; size], 0)))
        .collect()
}

/// Adds the glyphs of the card's count, when it cuts into as many as `text`.
fn add(sums: &mut Sums, image: &RgbImage, card: &grid::Card, text: &str) {
    let Some((x, found)) = quantity::count_glyphs(image, card) else {
        return;
    };
    if text.chars().count() != found.len() {
        return;
    }
    for (c, g) in std::iter::once('x').zip([x]).chain(text.chars().zip(found)) {
        let (sum, n) = sums.get_mut(&c).unwrap();
        sum.iter_mut().zip(g).for_each(|(s, v)| *s += v);
        *n += 1;
    }
}

/// Each character's average as a reference, named `source` in `samples`.
fn keep(
    sums: Sums,
    source: &str,
    glyphs: &mut BTreeMap<char, Vec<Vec<f32>>>,
    samples: &mut Vec<String>,
) {
    for (c, (sum, n)) in sums {
        if n == 0 {
            continue;
        }
        let mean = sum
            .into_iter()
            .map(|v| (v / n as f32 * 1000.0).round() / 1000.0);
        glyphs.get_mut(&c).unwrap().push(mean.collect());
        samples.push(format!("{c}@{source}:{n}"));
    }
}

/// The middle of each card found, as a share of the screen's width and height.
fn places(image: &RgbImage) -> Vec<(f32, f32)> {
    grid::find_cards(image)
        .iter()
        .map(|c| {
            (
                c.center_x() / image.width() as f32,
                (c.top as f32 + c.height as f32 / 2.0) / image.height() as f32,
            )
        })
        .collect()
}

/// The even-numbered stops of the tablet's collection (see the top).
fn tablet(dir: &Path, glyphs: &mut BTreeMap<char, Vec<Vec<f32>>>, samples: &mut Vec<String>) {
    let recognizer = Recognizer::embedded().unwrap();
    let even = |name: &str| {
        name.rsplit('-')
            .next()
            .and_then(|k| k.parse::<u32>().ok())
            .is_some_and(|k| k % 2 == 0)
    };
    let mut stops: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().to_string_lossy().into_owned();
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
        let native = open(format!("{stop}-native.png")).unwrap();
        let Some((_, labels, _)) = recognizer.read_list(&native) else {
            continue;
        };
        let native_places = places(&native);
        for size in STREAMED {
            let Some(image) = open(format!("{stop}-{size}.png")) else {
                continue;
            };
            let mut sums = sums();
            for (card, (x, y)) in grid::find_cards(&image).iter().zip(places(&image)) {
                // The native card at the same place: the two count the rows
                // cut off at the edges apart.
                let Some((j, far)) = native_places
                    .iter()
                    .map(|&(nx, ny)| (nx - x).hypot(ny - y))
                    .enumerate()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                else {
                    continue;
                };
                let label = &labels[j];
                if let (true, Some(_), Some(count)) = (far < 0.02, label.id, label.quantity) {
                    add(&mut sums, &image, card, &count.to_string());
                }
            }
            keep(sums, &format!("{stop}-{size}"), glyphs, samples);
        }
    }
}

fn main() {
    let collection = std::env::args()
        .nth(1)
        .expect("usage: build_digits <collection of the tablet's screens>");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let labels: Vec<ItemFixture> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap()).unwrap();
    let tuning: Vec<ItemFixture> = labels.into_iter().filter(|f| f.role == "tune").collect();

    let mut glyphs: BTreeMap<char, Vec<Vec<f32>>> =
        CHARACTERS.chars().map(|c| (c, Vec::new())).collect();
    let mut samples = Vec::new();
    for width in WIDTHS {
        let mut sums = sums();
        for fixture in &tuning {
            let image = image::open(dir.join(&fixture.image)).unwrap().to_rgb8();
            let height = image.height() * width / image.width();
            let image = resize(&image, width, height, FilterType::Triangle);
            for (card, label) in grid::find_cards(&image).iter().zip(&fixture.cells) {
                if let Some(text) = &label.quantity {
                    add(&mut sums, &image, card, text);
                }
            }
        }
        keep(sums, &width.to_string(), &mut glyphs, &mut samples);
    }
    tablet(Path::new(&collection), &mut glyphs, &mut samples);
    let missing: String = glyphs
        .iter()
        .filter(|(_, refs)| refs.is_empty())
        .map(|(c, _)| *c)
        .collect();
    assert!(missing.is_empty(), "no samples for {missing:?}");
    let json = serde_json::to_string(&quantity::Digits { glyphs }).unwrap();
    std::fs::write(root.join("data/digits.json"), json + "\n").unwrap();
    println!("samples: {}", samples.join(" "));
}
