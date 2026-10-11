//! Accuracy against screenshots labeled by eye (tests/fixtures/labels.json).
//! Labels list the fully visible cells in reading order; cut-off cells are
//! not listed and must not be reported.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use aoi::{
    Cell, Recognizer,
    tally::{CONFIRM, Tally},
};
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
    /// The cell shows a remaining-time badge over its icon.
    #[serde(default)]
    badge: bool,
    /// An item the user does not track: recognized, never reported.
    #[serde(default)]
    ignored: bool,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixtures() -> Vec<Fixture> {
    let json = std::fs::read_to_string(fixtures_dir().join("labels.json")).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// Items with a remaining-time badge keep an order of their own; the others
/// follow the list's ascending ids.
#[test]
fn labels_follow_ascending_item_ids() {
    for fixture in fixtures() {
        let ids: Vec<i64> = fixture
            .cells
            .iter()
            .filter(|c| !c.badge)
            .filter_map(|c| c.id)
            .collect();
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
        let read: Vec<(Option<i64>, Option<u32>, bool)> = recognizer
            .read(&image)
            .into_iter()
            .map(
                |Cell {
                     id,
                     quantity,
                     ignored,
                 }| (id, quantity, ignored),
            )
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
            let want = (want.id, want.quantity, want.ignored);
            if *got != want {
                failures.push(format!(
                    "{} cell {i}: got {got:?}, want {want:?}",
                    fixture.image
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn other_screens_give_no_cells() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    for name in [
        "other/home.png",
        "other/momotalk.png",
        "other/event-stages.png",
    ] {
        let image = image::open(fixtures_dir().join(name)).unwrap().to_rgb8();
        assert!(recognizer.read_list(&image).is_none(), "{name}");
    }
    let list = image::open(fixtures_dir().join("items-16x9-secretstone.png"))
        .unwrap()
        .to_rgb8();
    assert_eq!(
        recognizer.read_list(&list).map(|cells| cells.len()),
        Some(15)
    );
}

/// The labeled screenshots shown one after another, each for a few frames, with
/// other screens between them, tally to exactly the labeled counts.
#[test]
fn a_run_of_frames_tallies_to_the_labels() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let open = |name: &str| image::open(fixtures_dir().join(name)).unwrap().to_rgb8();
    let mut tally = Tally::default();
    let mut expected = HashMap::new();
    for fixture in fixtures() {
        for name in [fixture.image.as_str(), "other/home.png"] {
            let image = open(name);
            for _ in 0..CONFIRM {
                if let Some(cells) = recognizer.read_list(&image) {
                    tally.observe(&cells);
                }
            }
        }
        for label in fixture.cells.iter().filter(|l| !l.ignored) {
            if let (Some(id), Some(quantity)) = (label.id, label.quantity) {
                expected.insert(id, quantity);
            }
        }
    }
    assert_eq!(tally.counts(), &expected);
}
