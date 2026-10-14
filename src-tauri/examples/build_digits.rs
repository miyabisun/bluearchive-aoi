//! Writes `data/digits.json`: the average glyph of each character of a count
//! (the digits and "K") at each of `WIDTHS`, taken from the fixtures labeled
//! `"role": "tune"` in `labels.json`, scaled to that width. Evaluation
//! fixtures are never used here.
//! Run with `cargo run --example build_digits`.

use std::{collections::BTreeMap, path::Path};

use aoi::{grid, quantity};
use image::imageops::{FilterType, resize};
use serde::Deserialize;

const CHARACTERS: &str = "0123456789K";
/// The widths the list is drawn at: a PC window (the fixtures' own, 2222)
/// down to a small phone's screen or a stream of it.
const WIDTHS: [u32; 5] = [2222, 1920, 1600, 1280, 1024];

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

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let labels: Vec<ItemFixture> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap()).unwrap();
    let tuning: Vec<ItemFixture> = labels.into_iter().filter(|f| f.role == "tune").collect();

    let size = quantity::GLYPH.0 * quantity::GLYPH.1;
    let mut glyphs: BTreeMap<char, Vec<Vec<f32>>> =
        CHARACTERS.chars().map(|c| (c, Vec::new())).collect();
    let mut samples = Vec::new();
    for width in WIDTHS {
        let mut sums: BTreeMap<char, (Vec<f32>, u32)> = CHARACTERS
            .chars()
            .map(|c| (c, (vec![0.0; size], 0)))
            .collect();
        for fixture in &tuning {
            let image = image::open(dir.join(&fixture.image)).unwrap().to_rgb8();
            let height = image.height() * width / image.width();
            let image = resize(&image, width, height, FilterType::Triangle);
            for (card, label) in grid::find_cards(&image).iter().zip(&fixture.cells) {
                let (Some(text), Some(found)) = (&label.quantity, quantity::glyphs(&image, card))
                else {
                    continue;
                };
                if text.chars().count() != found.len() {
                    continue;
                }
                for (c, g) in text.chars().zip(found) {
                    let (sum, n) = sums.get_mut(&c).unwrap();
                    sum.iter_mut().zip(g).for_each(|(s, v)| *s += v);
                    *n += 1;
                }
            }
        }
        for (c, (sum, n)) in sums {
            if n == 0 {
                continue;
            }
            let mean = sum
                .into_iter()
                .map(|v| (v / n as f32 * 1000.0).round() / 1000.0);
            glyphs.get_mut(&c).unwrap().push(mean.collect());
            samples.push(format!("{c}@{width}:{n}"));
        }
    }
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
