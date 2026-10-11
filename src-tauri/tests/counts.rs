//! Counts read from screenshots whose counts alone are labeled by eye
//! (tests/fixtures/counts.json), shown as the game shows them ("1383", "51K").

use std::path::{Path, PathBuf};

use aoi::Recognizer;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    counts: Vec<String>,
    /// Cells under the blinking selection frame: unreadable is fine there,
    /// a wrong number is not.
    selected: Vec<usize>,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn every_count_reads_as_shown_and_none_reads_wrong() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("counts.json")).unwrap())
            .unwrap();
    let mut wrong = Vec::new();
    for fixture in fixtures {
        let image = image::open(fixtures_dir().join(&fixture.image))
            .unwrap()
            .to_rgb8();
        let read = recognizer.read(&image);
        if read.len() != fixture.counts.len() {
            wrong.push(format!(
                "{}: read {} cells, expected {}",
                fixture.image,
                read.len(),
                fixture.counts.len()
            ));
        }
        for (i, (cell, want)) in read.iter().zip(&fixture.counts).enumerate() {
            let got = cell.quantity.map(|q| q.to_string());
            let fine =
                got.as_ref() == Some(want) || (got.is_none() && fixture.selected.contains(&i));
            if !fine {
                wrong.push(format!(
                    "{} cell {i}: got {got:?}, want {want}",
                    fixture.image
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
