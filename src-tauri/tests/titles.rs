//! The screens told by the title at their top left (`aoi::title`), against
//! screenshots labeled with it (`tests/fixtures/titles.json`) and every other
//! fixture, whose screens write another title or none.

mod common;

use std::path::{Path, PathBuf};

use aoi::Recognizer;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    title: String,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn recognizer() -> Recognizer {
    Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
        .expect("the recognizer loads data/")
}

fn open(path: &Path) -> image::RgbImage {
    image::open(path).expect("the image opens").to_rgb8()
}

/// A labeled screen reads as its title at its own size and drawn at the
/// widths a window, a phone or a tablet draws the game at.
#[test]
fn a_screen_reads_as_its_title_at_every_width() {
    let recognizer = recognizer();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("titles.json")).unwrap())
            .unwrap();
    let mut wrong = Vec::new();
    for fixture in &fixtures {
        let image = open(&fixtures_dir().join(&fixture.image));
        let read = recognizer.read_title(&aoi::screen::normalize(&image));
        if read != Some(fixture.title.as_str()) {
            wrong.push(format!("{} own size: {read:?}", fixture.image));
        }
        for width in [2222, 2100, 1920, 1760, 1600, 1550, 1500] {
            let read = recognizer.read_title(&common::drawn_at(&image, width));
            if read != Some(fixture.title.as_str()) {
                wrong.push(format!("{} at {width}: {read:?}", fixture.image));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// No other fixture's screen (the lists, the student screen, the shops, the
/// home screen, phones' and tablets' alike) reads as a title.
#[test]
fn other_screens_read_as_no_title() {
    let recognizer = recognizer();
    let mut read = Vec::new();
    let mut seen = 0;
    let mut dirs = vec![fixtures_dir()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "titles") {
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "png") {
                seen += 1;
                let image = aoi::screen::normalize(&open(&path)).into_owned();
                if let Some(title) = recognizer.read_title(&image) {
                    read.push(format!("{}: {title}", path.display()));
                }
            }
        }
    }
    assert!(seen > 200, "only {seen} fixtures found");
    assert!(read.is_empty(), "\n{}", read.join("\n"));
}
