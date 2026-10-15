//! The credits of the top bar read against screenshots labeled by eye
//! (tests/fixtures/credits.json): each reads right or not at all, a screen
//! dimmed by a dialog and one without the bar read as nothing.

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

/// Every screen's credits read, right, as Aoi reads them (scaled down to Full
/// HD, where a "3" comes near an "8": with the tablet's credits among the
/// references, 11 of 11 on 2026-10-15); a screen with no credits shown reads
/// none.
#[test]
fn every_screen_reads_its_credits() {
    let recognizer = recognizer();
    let labels: Vec<Label> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/credits.json")).unwrap(),
    )
    .unwrap();
    let (mut read, mut shown) = (0, 0);
    for label in labels {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        let image = aoi::screen::normalize(&image).into_owned();
        let credits = recognizer.read_credits(&image);
        match label.credits {
            Some(_) => {
                shown += 1;
                if credits.is_some() {
                    assert_eq!(credits, label.credits, "{}", label.image);
                    read += 1;
                }
            }
            None => assert_eq!(credits, None, "{}", label.image),
        }
    }
    assert_eq!(read, shown, "credits read");
}

#[test]
fn a_screen_without_the_bar_has_no_credits() {
    // The student fixtures keep what the student screen reads, not the bar.
    let image = image::open(root().join("tests/fixtures/students/mika.png"))
        .unwrap()
        .to_rgb8();
    assert_eq!(recognizer().read_credits(&image), None);
}
