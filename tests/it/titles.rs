//! The screens told by the title at their top left (`aoi::title`), against
//! screenshots labeled with it and the screen read where the title tells more
//! than one (`tests/fixtures/titles.json`), and every other fixture, whose
//! screens write another title or none.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    title: String,
    screen: Option<String>,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn recognizer() -> &'static aoi::Recognizer {
    crate::common::recognizer()
}

fn open(path: &Path) -> image::RgbImage {
    image::open(path).expect("the image opens").to_rgb8()
}

/// A labeled screen reads as its title (or its screen) at its own size and
/// drawn at the widths a window, a phone or a tablet draws the game at.
#[test]
fn a_screen_reads_as_its_title_at_every_width() {
    let recognizer = recognizer();
    let fixtures: Vec<Fixture> =
        serde_json::from_str(&std::fs::read_to_string(fixtures_dir().join("titles.json")).unwrap())
            .unwrap();
    // The screens side by side, each opened once.
    let wrong: Vec<String> = crate::common::each(&fixtures, |fixture| {
        let mut wrong = Vec::new();
        let screen = Some(fixture.screen.as_deref().unwrap_or(&fixture.title));
        let image = open(&fixtures_dir().join(&fixture.image));
        let read = recognizer.read_title(&aoi::screen::normalize(&image));
        if read != screen {
            wrong.push(format!("{} own size: {read:?}", fixture.image));
        }
        for width in [2222, 2100, 1920, 1760, 1600, 1550, 1500] {
            let read = recognizer.read_title(&crate::common::drawn_at(&image, width));
            if read != screen {
                wrong.push(format!("{} at {width}: {read:?}", fixture.image));
            }
        }
        wrong
    })
    .concat();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The 任務 screen tells no screen where it shows no campaign: with the label
/// or the text after it hidden under the band's navy, the title alone.
#[test]
fn the_mission_screen_without_its_campaign_reads_as_no_screen() {
    use aoi::{campaign, student};
    let recognizer = recognizer();
    let image = open(&fixtures_dir().join("titles/ninmu-normal.png"));
    let image = aoi::screen::normalize(&image).into_owned();
    assert_eq!(recognizer.read_title(&image), Some("任務 Normal"));
    let parts = [
        campaign::LABEL_AREA,
        campaign::text_area(campaign::TEXTS[0].to),
    ];
    for (i, area) in parts.into_iter().enumerate() {
        let mut hidden = image.clone();
        let (x0, y0, x1, y1) = student::rect(&hidden, area).expect("the part lies on the screen");
        for y in y0..y1 {
            for x in x0..x1 {
                hidden.put_pixel(x, y, image::Rgb([36, 47, 70]));
            }
        }
        assert_eq!(recognizer.read_title(&hidden), None, "part {i} hidden");
    }
}

/// The shop screen tells the 通常アイテム shop by the menu's choice alone: with
/// a month-end shop chosen there (each shops fixture's menu over it), no screen;
/// those are told by their offers (`aoi::shop`).
#[test]
fn the_shop_screen_with_another_shop_chosen_reads_as_no_screen() {
    use aoi::{shop, student};
    let recognizer = recognizer();
    let image = open(&fixtures_dir().join("titles/shop-normal.png"));
    let image = aoi::screen::normalize(&image).into_owned();
    assert_eq!(
        recognizer.read_title(&image),
        Some("ショップ: 通常アイテム")
    );
    let mut read = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir().join("shops")).unwrap() {
        let path = entry.unwrap().path();
        // Drawn as wide as the 通常アイテム screen was captured.
        let other = crate::common::drawn_at(&open(&path), 2370);
        assert_eq!(other.dimensions(), image.dimensions(), "{}", path.display());
        let mut chosen = image.clone();
        for area in shop::areas() {
            let (x0, y0, x1, y1) =
                student::rect(&chosen, area).expect("the menu lies on the screen");
            for y in y0..y1 {
                for x in x0..x1 {
                    chosen.put_pixel(x, y, *other.get_pixel(x, y));
                }
            }
        }
        // The other shop's choice shows there, so it is what tells no screen.
        assert!(shop::menu_ink(&chosen).is_some(), "{}", path.display());
        if let Some(screen) = recognizer.read_title(&chosen) {
            read.push(format!("{}: {screen}", path.display()));
        }
    }
    assert!(read.is_empty(), "\n{}", read.join("\n"));
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
