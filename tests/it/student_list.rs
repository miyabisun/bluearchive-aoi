//! The student list in the order of acceptance (`aoi::student::list`), against
//! screenshots labeled by eye (`tests/fixtures/student-lists.json`): every
//! place reads as what stands there or not at all, and the list filtered,
//! searched, on another tab or sorted by another key is not read. A fixture
//! that differs from another in a part of the screen keeps that part alone
//! (`parts`) and is laid over the other (`over`), as `examples/mask_fixture.rs`
//! checked it.

use std::path::{Path, PathBuf};

use aoi::{
    Frame,
    student::{
        accepted::{Counting, Took},
        list::{self, Cell, Seen},
    },
};
use image::RgbImage;
use serde::Deserialize;

use crate::common::{drawn_at, each, recognizer};

#[derive(Deserialize)]
struct Fixture {
    image: String,
    /// The fixture this one is laid over, and the parts it keeps of its own.
    over: Option<String>,
    #[serde(default)]
    parts: Vec<String>,
    /// Which way the list runs; none for a list that is not to be read.
    ascending: Option<bool>,
    first: Option<usize>,
    /// The places in view: a student's name, `held` (a held card whose name is
    /// not whole in view), `grey` (a card not held) or `-` (no card).
    #[serde(default)]
    cells: Vec<String>,
}

/// The widths a window, a phone or a tablet draws the game at.
const WIDTHS: [u32; 9] = [2222, 2100, 1980, 1920, 1840, 1760, 1680, 1600, 1500];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixtures() -> Vec<Fixture> {
    let labels = std::fs::read_to_string(fixtures_dir().join("student-lists.json")).unwrap();
    serde_json::from_str(&labels).unwrap()
}

fn open(path: &str) -> RgbImage {
    image::open(fixtures_dir().join(path))
        .expect("the image opens")
        .to_rgb8()
}

/// The screen a fixture stands for: its picture, laid over another's where it
/// keeps parts of the screen alone.
fn screen(fixture: &Fixture) -> RgbImage {
    let own = open(&fixture.image);
    let Some(over) = &fixture.over else {
        return own;
    };
    let mut whole = open(over);
    assert_eq!(whole.dimensions(), own.dimensions(), "{}", fixture.image);
    let areas = list::areas(&whole);
    for (name, rects) in &areas.parts {
        if !fixture.parts.iter().any(|part| part == name) {
            continue;
        }
        for &(x0, y0, x1, y1) in rects {
            for (x, y) in (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))) {
                whole.put_pixel(x, y, *own.get_pixel(x, y));
            }
        }
    }
    whole
}

fn id_of(name: &str) -> i64 {
    let students = recognizer().students();
    students
        .iter()
        .find(|(_, known)| known == name)
        .unwrap_or_else(|| panic!("{name} is no student of data/"))
        .0
}

/// The place a label stands for.
fn labeled(cell: &str) -> Cell {
    match cell {
        "held" => Cell::Held(None),
        "grey" => Cell::Lacked,
        "-" => Cell::Empty,
        name => Cell::Held(Some(id_of(name))),
    }
}

/// What is wrong with a reading against its label: a place told otherwise
/// than labeled (a held card not named, or a place not told, is not wrong),
/// the list's start told elsewhere, or another count of places.
fn wrong(fixture: &Fixture, seen: &Seen, at: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    let sight = &seen.sight;
    if Some(sight.ascending) != fixture.ascending {
        wrong.push(format!("{} {at}: runs the other way", fixture.image));
    }
    if sight.first.is_some() && sight.first != fixture.first {
        wrong.push(format!(
            "{} {at}: starts at {:?}",
            fixture.image, sight.first
        ));
    }
    if sight.cells.len() != fixture.cells.len() {
        wrong.push(format!(
            "{} {at}: {} places, labeled {}",
            fixture.image,
            sight.cells.len(),
            fixture.cells.len()
        ));
    }
    for (place, (read, label)) in sight.cells.iter().zip(&fixture.cells).enumerate() {
        let is = labeled(label);
        let fits = match read {
            None => true,
            Some(Cell::Held(None)) => matches!(is, Cell::Held(_)),
            Some(read) => *read == is,
        };
        if !fits {
            wrong.push(format!(
                "{} {at}: place {place} read {read:?}, is {label}",
                fixture.image
            ));
        }
    }
    wrong
}

/// At the size the screens were taken (read scaled down to Full HD, as a
/// window of that size is), every place reads as labeled, the names all told.
#[test]
fn every_place_reads_as_labeled() {
    for fixture in fixtures().iter().filter(|f| f.ascending.is_some()) {
        let image = aoi::screen::normalize(&screen(fixture)).into_owned();
        let seen = recognizer()
            .read_student_list(&image)
            .unwrap_or_else(|| panic!("{} is not read", fixture.image));
        let cells: Vec<Option<Cell>> = fixture.cells.iter().map(|c| Some(labeled(c))).collect();
        assert_eq!(seen.sight.cells, cells, "{}", fixture.image);
        assert_eq!(seen.sight.first, fixture.first, "{}", fixture.image);
        assert_eq!(
            Some(seen.sight.ascending),
            fixture.ascending,
            "{}",
            fixture.image
        );
        // The whole frame reads as the list, too.
        assert!(
            matches!(recognizer().read_frame(&image), Some(Frame::Students(_))),
            "{}",
            fixture.image
        );
    }
}

/// At any width a screen is read at, and between them, every place reads
/// right or not at all, and most names are told (measured on 2026-11-24 over
/// these screens at these widths: 231 of 234 names, the kanji of a long name
/// drawn small going untold first).
#[test]
fn every_place_reads_right_or_not_at_all_at_other_widths() {
    const LEAST: f32 = 0.9;
    let fixtures = fixtures();
    let read: Vec<(Fixture, RgbImage)> = fixtures
        .into_iter()
        .filter(|f| f.ascending.is_some())
        .map(|f| {
            let screen = screen(&f);
            (f, screen)
        })
        .collect();
    let cases: Vec<(&Fixture, &RgbImage, u32)> = read
        .iter()
        .flat_map(|(f, screen)| WIDTHS.map(|width| (f, screen, width)))
        .collect();
    let told = each(&cases, |&(fixture, screen, width)| {
        let image = drawn_at(screen, width);
        let at = format!("at {width}");
        let Some(seen) = recognizer().read_student_list(&image) else {
            return (vec![format!("{} {at}: not read", fixture.image)], 0, 0);
        };
        let written = fixture.cells.iter().filter(|c| is_name(c)).count();
        let named = seen
            .sight
            .cells
            .iter()
            .filter(|c| matches!(c, Some(Cell::Held(Some(_)))))
            .count();
        (wrong(fixture, &seen, &at), named, written)
    });
    let wrong: Vec<String> = told.iter().flat_map(|t| t.0.clone()).collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    let (named, written) = told
        .iter()
        .fold((0, 0), |sum, t| (sum.0 + t.1, sum.1 + t.2));
    assert!(
        named as f32 >= LEAST * written as f32,
        "{named} of {written} names told"
    );
}

fn is_name(cell: &str) -> bool {
    !["held", "grey", "-"].contains(&cell)
}

/// The list searched, filtered, narrowed by the button left of the tabs, on
/// another tab, sorted by another key, or under the display settings' dialog
/// is not read at any width: its places then say nothing of the order of
/// acceptance.
#[test]
fn the_list_in_another_state_is_not_read() {
    let fixtures = fixtures();
    let others: Vec<(&Fixture, RgbImage)> = fixtures
        .iter()
        .filter(|f| f.ascending.is_none())
        .map(|f| (f, screen(f)))
        .collect();
    assert_eq!(others.len(), 7, "every state not read has its screen");
    let cases: Vec<(&Fixture, &RgbImage, Option<u32>)> = others
        .iter()
        .flat_map(|(f, screen)| {
            WIDTHS
                .map(Some)
                .into_iter()
                .chain([None])
                .map(move |width| (*f, screen, width))
        })
        .collect();
    let read = each(&cases, |&(fixture, screen, width)| {
        let image = match width {
            Some(width) => drawn_at(screen, width),
            None => aoi::screen::normalize(screen).into_owned(),
        };
        let header = list::header(&image, recognizer().student_sorts());
        let seen = recognizer().read_student_list(&image);
        (header.is_some() || seen.is_some()).then(|| format!("{} at {width:?}", fixture.image))
    });
    let read: Vec<String> = read.into_iter().flatten().collect();
    assert!(read.is_empty(), "read as the whole list: {read:?}");
}

/// No other screen of the fixtures is taken for the student list.
#[test]
fn no_other_screen_is_the_student_list() {
    fn pictures(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "student-list") {
                    pictures(&path, found);
                }
            } else if path.extension().is_some_and(|e| e == "png") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    pictures(&fixtures_dir(), &mut found);
    assert!(found.len() > 100, "the other fixtures are found");
    let taken = each(&found, |path| {
        let image = image::open(path).expect("the image opens").to_rgb8();
        let image = aoi::screen::normalize(&image);
        recognizer()
            .read_student_list(&image)
            .is_some()
            .then(|| path.display().to_string())
    });
    let taken: Vec<String> = taken.into_iter().flatten().collect();
    assert!(taken.is_empty(), "taken for the student list: {taken:?}");
}

/// The screens through the counting the watcher does: the list's end with the
/// newest first, and its start with the oldest first, give the same students
/// the same numbers.
#[test]
fn the_lists_end_and_its_start_number_the_same_students_alike() {
    let fixtures = fixtures();
    let numbered = |image: &str| {
        let fixture = fixtures.iter().find(|f| f.image == image).unwrap();
        let screen = aoi::screen::normalize(&screen(fixture)).into_owned();
        let seen = recognizer().read_student_list(&screen).unwrap();
        let mut counting = Counting::default();
        let mut took = Took::Nothing;
        for _ in 0..aoi::tally::CONFIRM {
            took = counting.frame(&seen);
        }
        (took, counting.known().clone())
    };
    let (took, from_end) = numbered("student-list/newest-end.png");
    assert_eq!(took, Took::Numbered(8));
    let wanted = [
        "ユウカ",
        "スズミ",
        "ハスミ",
        "チナツ",
        "ハルカ",
        "ジュリ",
        "アイリ",
        "コトリ",
    ];
    for (name, number) in wanted.into_iter().zip(1..) {
        assert_eq!(from_end.get(&id_of(name)), Some(&number), "{name}");
    }
    let (took, from_start) = numbered("student-list/oldest-top.png");
    assert_eq!(took, Took::Numbered(6));
    for (id, number) in &from_start {
        assert_eq!(from_end.get(id), Some(number), "student {id}");
    }
    // The newest first and no end in view: students beside one another, no number.
    let (took, none) = numbered("student-list/newest-middle.png");
    assert_eq!(took, Took::Loose);
    assert!(none.is_empty());
}
