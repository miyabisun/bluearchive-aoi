//! Accuracy against screenshots labeled by eye (tests/fixtures/labels.json).
//! Labels list the fully visible cells in reading order, with their counts as
//! the game shows them ("1383", "51K"); cut-off cells are not listed and must
//! not be reported.

mod common;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use aoi::{
    Cell, List, Recognizer,
    tally::{CONFIRM, Tally},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    list: List,
    cells: Vec<Label>,
    /// Whether the references were drawn from it ("tune") or not ("eval").
    role: String,
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
    let json = std::fs::read_to_string(fixtures_dir().join("labels.json")).expect("the file reads");
    serde_json::from_str(&json).expect("the JSON parses")
}

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
        .expect("the recognizer loads data/")
}

fn open(name: &str) -> image::RgbImage {
    image::open(fixtures_dir().join(name))
        .expect("the image opens")
        .to_rgb8()
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
            shown.map(|(list, cells, _)| (list, cells.len())),
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
/// The phone's were taken another day, of another inventory.
#[test]
fn a_run_of_frames_tallies_to_the_labels() {
    let recognizer = recognizer();
    let mut tallies: HashMap<List, Tally> = HashMap::new();
    let mut expected: HashMap<List, HashMap<i64, String>> = HashMap::new();
    let mut hidden: HashMap<List, HashMap<i64, String>> = HashMap::new();
    for fixture in fixtures()
        .into_iter()
        .filter(|f| !f.image.starts_with("android/"))
    {
        for name in [fixture.image.as_str(), OTHER_SCREENS[0]] {
            let image = open(name);
            for _ in 0..CONFIRM {
                if let Some((list, cells, start)) = recognizer.read_list(&image) {
                    // What each screen shows lacking must be no labeled item.
                    let tally = tallies.entry(list).or_default();
                    let gone = recognizer
                        .catalog(list)
                        .gaps(&cells, start, |id| tally.counts().contains_key(&id));
                    tally.observe(&cells, &gone);
                }
            }
        }
        for label in fixture.cells.iter().filter(|l| !l.ignored) {
            if let (Some(id), Some(quantity)) = (label.id, &label.quantity) {
                // The selected cell's count may be hidden: counted right, or not.
                let to = if label.selected {
                    &mut hidden
                } else {
                    &mut expected
                };
                to.entry(fixture.list)
                    .or_default()
                    .insert(id, quantity.clone());
            }
        }
    }
    let counted: HashMap<List, HashMap<i64, String>> = tallies
        .iter()
        .map(|(list, tally)| {
            let counts = tally
                .counts()
                .iter()
                .map(|(id, q)| (*id, q.to_string()))
                .filter(|(id, q)| hidden.get(list).and_then(|h| h.get(id)) != Some(q));
            (*list, counts.collect())
        })
        .collect();
    assert_eq!(counted, expected);
}

/// The cells of a screen against their labels, right or not at all: how many
/// are labeled (but ignored items) and how many read right, with a failure
/// for each read wrong.
fn right_or_not_at_all(
    name: &str,
    read: &[Cell],
    labels: &[Label],
    failures: &mut Vec<String>,
) -> (usize, usize) {
    let (mut cells, mut right) = (0, 0);
    for (i, (cell, want)) in read.iter().zip(labels).enumerate() {
        if want.ignored {
            continue;
        }
        cells += 1;
        let quantity = cell.quantity.map(|q| q.to_string());
        if cell.id.is_some_and(|id| Some(id) != want.id)
            || quantity
                .as_ref()
                .is_some_and(|q| Some(q) != want.quantity.as_ref())
        {
            failures.push(format!(
                "{name} cell {i}: got {:?} x{quantity:?}, want {:?} x{:?}",
                cell.id, want.id, want.quantity
            ));
        } else if cell.id.is_some() && quantity.is_some() {
            right += 1;
        }
    }
    (cells, right)
}

/// A screen of any width read, and any between them (a window as it was
/// resized), shows the list smaller or larger: the screens kept for
/// evaluation, drawn at widths from 1500 to 2222, read every cell's item and
/// count right or not at all, and most of them right.
#[test]
fn a_smaller_screen_reads_right_or_not_at_all() {
    let recognizer = recognizer();
    let mut failures = Vec::new();
    for width in [2222, 2100, 1920, 1760, 1600, 1550, 1500] {
        let (mut cells, mut right) = (0, 0);
        for fixture in fixtures().into_iter().filter(|f| f.role == "eval") {
            let small = common::drawn_at(&open(&fixture.image), width);
            let read = recognizer.read(&small, fixture.list);
            let name = format!("{width}: {}", fixture.image);
            let (c, r) = right_or_not_at_all(&name, &read, &fixture.cells, &mut failures);
            (cells, right) = (cells + c, right + r);
        }
        if right * 10 < cells * 9 {
            failures.push(format!("{width}: {right} of {cells} cells read"));
        }
    }
    assert!(
        failures.is_empty(),
        "
{}",
        failures.join(
            "
"
        )
    );
}

/// A tablet's screen as scrcpy streams it (`tests/fixtures/tablet.json`).
#[derive(Deserialize)]
struct Stream {
    image: String,
    list: List,
    cells: Vec<Label>,
}

/// A stream blurs the counts beyond any scaling (H.264): the tablet's screens
/// read every cell's item and count right or not at all, and at each width
/// at least the share of `LEAST` right. Measured on 2026-10-15 over the
/// collection's stops kept for evaluation: 99.6% at 1920 and 1600.
#[test]
fn a_streamed_screen_reads_right_or_not_at_all() {
    const LEAST: [(u32, f32); 2] = [(1920, 0.95), (1600, 0.9)];
    let json = std::fs::read_to_string(fixtures_dir().join("tablet.json")).unwrap();
    let streams: Vec<Stream> = serde_json::from_str(&json).unwrap();
    let recognizer = recognizer();
    let mut failures = Vec::new();
    let mut by_width: HashMap<u32, (usize, usize)> = HashMap::new();
    for stream in streams {
        let image = open(&stream.image);
        let read = recognizer.read(&image, stream.list);
        if read.len() != stream.cells.len() {
            failures.push(format!(
                "{}: read {} cells, expected {}",
                stream.image,
                read.len(),
                stream.cells.len()
            ));
        }
        let (c, r) = right_or_not_at_all(&stream.image, &read, &stream.cells, &mut failures);
        let (cells, right) = by_width.entry(image.width()).or_default();
        (*cells, *right) = (*cells + c, *right + r);
    }
    for (width, least) in LEAST {
        let (cells, right) = by_width.get(&width).copied().unwrap_or_default();
        if (right as f32) < least * cells as f32 || cells == 0 {
            failures.push(format!("{width}: {right} of {cells} cells read"));
        }
    }
    assert!(
        failures.is_empty(),
        "
{}",
        failures.join(
            "
"
        )
    );
}
