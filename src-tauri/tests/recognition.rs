//! Accuracy against screenshots labeled by eye (tests/fixtures/labels.json).
//! Labels list the fully visible cells in reading order; cut-off cells are
//! not listed and must not be reported.

use std::path::{Path, PathBuf};

use aoi::{Cell, Recognizer};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    cells: Vec<Label>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
struct Label {
    id: Option<i64>,
    quantity: Option<u32>,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixtures() -> Vec<Fixture> {
    let json = std::fs::read_to_string(fixtures_dir().join("labels.json")).unwrap();
    serde_json::from_str(&json).unwrap()
}

#[test]
fn labels_follow_ascending_item_ids() {
    for fixture in fixtures() {
        let ids: Vec<i64> = fixture.cells.iter().filter_map(|c| c.id).collect();
        assert!(
            ids.windows(2).all(|w| w[0] < w[1]),
            "{}: ids are not ascending: {ids:?}",
            fixture.image
        );
    }
}

#[test]
fn reads_every_fully_visible_cell() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let image = image::open(fixtures_dir().join(&fixture.image))
            .unwrap()
            .to_rgb8();
        let read: Vec<Label> = recognizer
            .read(&image)
            .into_iter()
            .map(|Cell { id, quantity }| Label { id, quantity })
            .collect();
        if read.len() != fixture.cells.len() {
            failures.push(format!(
                "{}: read {} cells, expected {}",
                fixture.image,
                read.len(),
                fixture.cells.len()
            ));
        }
        for (i, (got, want)) in read.iter().zip(&fixture.cells).enumerate() {
            if got != want {
                failures.push(format!(
                    "{} cell {i}: got {got:?}, want {want:?}",
                    fixture.image
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
