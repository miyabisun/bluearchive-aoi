//! Writes `data/digits.json`: the average glyph of each digit, taken from the
//! fixtures labeled `"role": "tune"`. Evaluation fixtures are never used here.
//! Run with `cargo run --example build_digits`.

use std::path::Path;

use aoi::{grid, quantity};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    role: String,
    cells: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    quantity: Option<u32>,
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap()).unwrap();
    let size = quantity::GLYPH.0 * quantity::GLYPH.1;
    let mut sums = vec![vec![0.0f32; size]; 10];
    let mut counts = [0u32; 10];
    for fixture in fixtures.iter().filter(|f| f.role == "tune") {
        let image = image::open(dir.join(&fixture.image)).unwrap().to_rgb8();
        for (card, label) in grid::find_cards(&image).iter().zip(&fixture.cells) {
            let (Some(q), Some(glyphs)) = (label.quantity, quantity::glyphs(&image, card)) else {
                continue;
            };
            let digits: Vec<usize> = q
                .to_string()
                .bytes()
                .map(|b| usize::from(b - b'0'))
                .collect();
            if digits.len() != glyphs.len() {
                continue;
            }
            for (d, g) in digits.into_iter().zip(glyphs) {
                sums[d].iter_mut().zip(g).for_each(|(s, v)| *s += v);
                counts[d] += 1;
            }
        }
    }
    let missing: Vec<usize> = (0..10).filter(|&d| counts[d] == 0).collect();
    assert!(missing.is_empty(), "no samples for digits {missing:?}");
    let glyphs = sums
        .into_iter()
        .zip(counts)
        .map(|(s, n)| {
            s.into_iter()
                .map(|v| (v / n as f32 * 1000.0).round() / 1000.0)
                .collect()
        })
        .collect();
    let json = serde_json::to_string(&quantity::Digits { glyphs }).unwrap();
    std::fs::write(root.join("data/digits.json"), json + "\n").unwrap();
    println!("samples per digit: {counts:?}");
}
