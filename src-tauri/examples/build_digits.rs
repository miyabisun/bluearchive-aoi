//! Writes `data/digits.json`: the average glyph of each character of a count
//! (the digits and "K"), taken from the fixtures labeled `"role": "tune"` in
//! `labels.json` and `counts.json`. Evaluation fixtures are never used here.
//! Run with `cargo run --example build_digits`.

use std::{collections::BTreeMap, path::Path};

use aoi::{grid, quantity};
use serde::Deserialize;

const CHARACTERS: &str = "0123456789K";

#[derive(Deserialize)]
struct ItemFixture {
    image: String,
    role: String,
    cells: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    quantity: Option<u32>,
}

#[derive(Deserialize)]
struct CountFixture {
    image: String,
    role: String,
    counts: Vec<String>,
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let labels: Vec<ItemFixture> = serde_json::from_str(&read("labels.json")).unwrap();
    let counts: Vec<CountFixture> = serde_json::from_str(&read("counts.json")).unwrap();
    // Each tuning screenshot with the text of its counts, cell by cell.
    let texts = labels
        .into_iter()
        .filter(|f| f.role == "tune")
        .map(|f| {
            let text = f.cells.iter().map(|c| c.quantity.map(|q| q.to_string()));
            (f.image, text.collect::<Vec<_>>())
        })
        .chain(
            counts
                .into_iter()
                .filter(|f| f.role == "tune")
                .map(|f| (f.image, f.counts.into_iter().map(Some).collect())),
        );

    let size = quantity::GLYPH.0 * quantity::GLYPH.1;
    let mut sums: BTreeMap<char, (Vec<f32>, u32)> = CHARACTERS
        .chars()
        .map(|c| (c, (vec![0.0; size], 0)))
        .collect();
    for (name, cells) in texts {
        let image = image::open(dir.join(&name)).unwrap().to_rgb8();
        for (card, text) in grid::find_cards(&image).iter().zip(cells) {
            let (Some(text), Some(glyphs)) = (text, quantity::glyphs(&image, card)) else {
                continue;
            };
            if text.chars().count() != glyphs.len() {
                continue;
            }
            for (c, g) in text.chars().zip(glyphs) {
                let (sum, n) = sums.get_mut(&c).unwrap();
                sum.iter_mut().zip(g).for_each(|(s, v)| *s += v);
                *n += 1;
            }
        }
    }
    let missing: String = sums
        .iter()
        .filter(|(_, (_, n))| *n == 0)
        .map(|(c, _)| *c)
        .collect();
    assert!(missing.is_empty(), "no samples for {missing:?}");
    let samples: Vec<String> = sums.iter().map(|(c, (_, n))| format!("{c}:{n}")).collect();
    let glyphs = sums
        .into_iter()
        .map(|(c, (sum, n))| {
            let mean = sum
                .into_iter()
                .map(|v| (v / n as f32 * 1000.0).round() / 1000.0);
            (c, mean.collect())
        })
        .collect();
    let json = serde_json::to_string(&quantity::Digits { glyphs }).unwrap();
    std::fs::write(root.join("data/digits.json"), json + "\n").unwrap();
    println!("samples: {}", samples.join(" "));
}
