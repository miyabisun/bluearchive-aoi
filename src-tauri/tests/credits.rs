//! The credits of the top bar read against screenshots labeled by eye
//! (tests/fixtures/credits.json): each reads exactly, a screen dimmed by a
//! dialog and one without the bar read as nothing.

use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Label {
    image: String,
    credits: Option<u64>,
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recognizer() -> aoi::Recognizer {
    aoi::Recognizer::load(&root().join("data")).unwrap()
}

#[test]
fn every_screen_reads_its_credits() {
    let recognizer = recognizer();
    let labels: Vec<Label> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/credits.json")).unwrap(),
    )
    .unwrap();
    for label in labels {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        assert_eq!(
            recognizer.read_credits(&image),
            label.credits,
            "{}",
            label.image
        );
    }
}

#[test]
fn a_screen_without_the_bar_has_no_credits() {
    // The student fixtures keep what the student screen reads, not the bar.
    let image = image::open(root().join("tests/fixtures/students/mika.png"))
        .unwrap()
        .to_rgb8();
    assert_eq!(recognizer().read_credits(&image), None);
}
