//! Accuracy against screenshots labeled by eye (tests/fixtures/labels.json).
//! Labels list the fully visible cells in reading order, with their counts as
//! the game shows them ("1383", "51K"); cut-off cells are not listed and must
//! not be reported.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use aoi::{
    List, Recognizer,
    tally::{CONFIRM, Tally},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    list: List,
    cells: Vec<Label>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
struct Label {
    id: Option<i64>,
    quantity: Option<String>,
    /// The cell shows a remaining-time badge over its icon.
    #[serde(default)]
    badge: bool,
    /// An item the user does not track: recognized, never reported.
    #[serde(default)]
    ignored: bool,
    /// The selected cell, whose blinking frame may hide the end of its count:
    /// an unreadable count is fine there, a wrong one is not.
    #[serde(default)]
    selected: bool,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixtures() -> Vec<Fixture> {
    let json = std::fs::read_to_string(fixtures_dir().join("labels.json")).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap()
}

fn open(name: &str) -> image::RgbImage {
    image::open(fixtures_dir().join(name)).unwrap().to_rgb8()
}

const OTHER_SCREENS: [&str; 3] = [
    "other/home.png",
    "other/momotalk.png",
    "other/event-stages.png",
];

/// Items with a remaining-time badge keep an order of their own; the others
/// follow the list's ascending ids.
#[test]
fn labels_follow_ascending_ids() {
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
    let recognizer = recognizer();
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let read = recognizer.read(&open(&fixture.image), fixture.list);
        if read.len() != fixture.cells.len() {
            failures.push(format!(
                "{}: read {} cells, expected {}",
                fixture.image,
                read.len(),
                fixture.cells.len()
            ));
        }
        for (i, (cell, want)) in read.iter().zip(&fixture.cells).enumerate() {
            let quantity = cell.quantity.map(|q| q.to_string());
            let quantity_ok = quantity == want.quantity || (want.selected && quantity.is_none());
            if cell.id != want.id || cell.ignored != want.ignored || !quantity_ok {
                failures.push(format!(
                    "{} cell {i}: got {:?} x{quantity:?}{}, want {:?} x{:?}{}",
                    fixture.image,
                    cell.id,
                    if cell.ignored { " ignored" } else { "" },
                    want.id,
                    want.quantity,
                    if want.ignored { " ignored" } else { "" },
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Item and equipment ids overlap, so a list read with the other list's
/// entries must name nothing.
#[test]
fn neither_list_is_read_as_the_other() {
    let recognizer = recognizer();
    for fixture in fixtures() {
        let other = match fixture.list {
            List::Items => List::Equipment,
            List::Equipment => List::Items,
        };
        let named: Vec<i64> = recognizer
            .read(&open(&fixture.image), other)
            .iter()
            .filter_map(|c| c.id)
            .collect();
        assert!(
            named.is_empty(),
            "{} as {other:?}: {named:?}",
            fixture.image
        );
    }
}

#[test]
fn each_list_is_told_apart_and_other_screens_give_nothing() {
    let recognizer = recognizer();
    for fixture in fixtures() {
        let shown = recognizer.read_list(&open(&fixture.image));
        assert_eq!(
            shown.map(|(list, cells)| (list, cells.len())),
            Some((fixture.list, fixture.cells.len())),
            "{}",
            fixture.image
        );
    }
    for name in OTHER_SCREENS {
        assert!(recognizer.read_list(&open(name)).is_none(), "{name}");
    }
}

/// The labeled screenshots shown one after another, each for a few frames, with
/// other screens between them, tally to exactly the labeled counts of each list.
#[test]
fn a_run_of_frames_tallies_to_the_labels() {
    let recognizer = recognizer();
    let mut tallies: HashMap<List, Tally> = HashMap::new();
    let mut expected: HashMap<List, HashMap<i64, String>> = HashMap::new();
    for fixture in fixtures() {
        for name in [fixture.image.as_str(), OTHER_SCREENS[0]] {
            let image = open(name);
            for _ in 0..CONFIRM {
                if let Some((list, cells)) = recognizer.read_list(&image) {
                    // What each screen shows lacking must be no labeled item.
                    let gone = recognizer.catalog(list).gaps(&cells);
                    tallies.entry(list).or_default().observe(&cells, &gone);
                }
            }
        }
        for label in fixture.cells.iter().filter(|l| !l.ignored && !l.selected) {
            if let (Some(id), Some(quantity)) = (label.id, &label.quantity) {
                expected
                    .entry(fixture.list)
                    .or_default()
                    .insert(id, quantity.clone());
            }
        }
    }
    let counted: HashMap<List, HashMap<i64, String>> = tallies
        .iter()
        .map(|(list, tally)| {
            let counts = tally.counts().iter().map(|(id, q)| (*id, q.to_string()));
            (*list, counts.collect())
        })
        .collect();
    assert_eq!(counted, expected);
}
