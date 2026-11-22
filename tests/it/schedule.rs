//! The schedule's screens past its list of locations (`aoi::schedule`), against
//! screenshots labeled with the location the chosen screen names or the first
//! room its dialog lists (`tests/fixtures/schedule.json`): each is told as the
//! schedule with its location's school (`aoi::watch::place_of`).

use std::path::{Path, PathBuf};

use aoi::{
    schedule::{self, Location},
    student, title,
    watch::Place,
};
use image::RgbImage;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    image: String,
    location: Option<String>,
    room: Option<String>,
}

/// The screen that keeps the title with its location's name; the other chosen
/// screens keep the name alone, the title being the same at every location.
const TITLED: &str = "titles/schedule-chosen.png";
/// The widths a window, a phone or a tablet draws the game at.
const WIDTHS: [u32; 7] = [2222, 2100, 1920, 1760, 1600, 1550, 1500];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixtures() -> Vec<Fixture> {
    let labels = std::fs::read_to_string(fixtures_dir().join("schedule.json")).unwrap();
    serde_json::from_str(&labels).unwrap()
}

fn open(path: &str) -> RgbImage {
    image::open(fixtures_dir().join(path))
        .expect("the image opens")
        .to_rgb8()
}

/// The location a fixture is labeled with, by its name or its first room.
fn labeled(fixture: &Fixture) -> &'static Location {
    schedule::LOCATIONS
        .iter()
        .find(|l| {
            Some(l.name) == fixture.location.as_deref() || Some(l.room) == fixture.room.as_deref()
        })
        .unwrap_or_else(|| panic!("{} is labeled with no location", fixture.image))
}

/// `image` (a chosen screen that keeps its location's name alone) with the
/// title of `titled` over its top left, as the game shows both.
fn with_title(mut image: RgbImage, titled: &RgbImage) -> RgbImage {
    assert_eq!(image.dimensions(), titled.dimensions());
    let (x0, y0, x1, y1) = student::rect(&image, title::AREA).expect("the title's area");
    for y in y0..y1 {
        for x in x0..x1 {
            image.put_pixel(x, y, *titled.get_pixel(x, y));
        }
    }
    image
}

/// The chosen screens, each whole: its title and its location's name.
fn chosen() -> Vec<(Fixture, RgbImage)> {
    let titled = open(TITLED);
    fixtures()
        .into_iter()
        .filter(|f| f.location.is_some())
        .map(|fixture| {
            let image = match fixture.image.as_str() {
                TITLED => titled.clone(),
                path => with_title(open(path), &titled),
            };
            (fixture, image)
        })
        .collect()
}

/// What `tell` tells of a screen at its own size and at each of `WIDTHS`, where
/// it is not `expected`.
fn wrong<T: PartialEq + std::fmt::Debug>(
    name: &str,
    image: &RgbImage,
    expected: &T,
    tell: impl Fn(&RgbImage) -> T,
) -> Vec<String> {
    let own = aoi::screen::normalize(image).into_owned();
    std::iter::once(("own size".to_owned(), own))
        .chain(
            WIDTHS
                .iter()
                .map(|&w| (format!("at {w}"), crate::common::drawn_at(image, w))),
        )
        .filter_map(|(at, shot)| {
            let told = tell(&shot);
            (told != *expected).then(|| format!("{name} {at}: {told:?}"))
        })
        .collect()
}

/// The user's 2026-11-23 「「スケジュールを選択」にタイトルが切り替わるため、対象外の画面に
/// なってしまう」「この「ゲヘナ学園・中央区」を認識したらゲヘナ学園の行を…括る」: every
/// location's chosen screen is the schedule, with the school of the location it
/// names (none at the four that give no one school's goods), at every width.
#[test]
fn a_chosen_screen_reads_as_the_schedule_with_its_location_s_school_at_every_width() {
    let recognizer = crate::common::recognizer();
    let screens = chosen();
    assert_eq!(
        screens.len(),
        schedule::LOCATIONS.len(),
        "a screen a location"
    );
    let wrong: Vec<String> = crate::common::each(&screens, |(fixture, image)| {
        let location = labeled(fixture);
        let told = (Some(location.name), Some(Place::Schedule(location.school)));
        wrong(&fixture.image, image, &told, |shot| {
            (
                recognizer.read_location(shot).map(|l| l.name),
                aoi::watch::place_of(recognizer, shot),
            )
        })
    })
    .concat();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The same request's 「モーダルが表示されるので、左上の「ゲヘナ第一校舎」を認識する」: every
/// location's dialog, which dims the title and covers the location's name, is
/// the schedule with the school of the location whose first room its top left
/// card names (D.U.シラトリ区's by the first of its name's two lines).
#[test]
fn a_dialog_reads_as_the_schedule_with_its_first_room_s_school_at_every_width() {
    let recognizer = crate::common::recognizer();
    let dialogs: Vec<(Fixture, RgbImage)> = fixtures()
        .into_iter()
        .filter(|f| f.room.is_some())
        .map(|fixture| {
            let image = open(&fixture.image);
            (fixture, image)
        })
        .collect();
    assert_eq!(
        dialogs.len(),
        schedule::LOCATIONS.len(),
        "a dialog a location"
    );
    let wrong: Vec<String> = crate::common::each(&dialogs, |(fixture, image)| {
        let location = labeled(fixture);
        let told = (Some(location.room), Some(Place::Schedule(location.school)));
        wrong(&fixture.image, image, &told, |shot| {
            (
                recognizer.read_first_room(shot).map(|l| l.room),
                aoi::watch::place_of(recognizer, shot),
            )
        })
    })
    .concat();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// How much narrower the rank's box is drawn for a rank of one digit, in units:
/// a digit of "RANK12" (measured on 2026-11-23 at 2370x1333: the box 0.1155
/// wide for the six letters of about 0.019 each).
const DIGIT: f32 = 0.019;

/// The rank's box on a chosen screen: the first run of dark columns in the
/// name's area, its first column and the one past its last.
fn rank_box(image: &RgbImage) -> (u32, u32) {
    let (x0, y0, x1, y1) = student::rect(image, schedule::NAME_AREA).expect("the name's area");
    let dark = |x: u32| (y0..y1).any(|y| image.get_pixel(x, y).0.iter().all(|&v| v < 128));
    let left = (x0..x1).find(|&x| dark(x)).expect("the box");
    let right = (left..x1).find(|&x| !dark(x)).expect("the box ends");
    (left, right)
}

/// `image` (a chosen screen) as a location of a one-digit rank would draw its
/// name's row: the box a digit narrower and the name as much nearer. No such
/// screen can be captured (the user's locations are all of rank 12), so the
/// columns of the box's last digit are taken out and what follows moved left,
/// the panel's ground filling the right end.
fn of_one_digit(image: &RgbImage) -> RgbImage {
    let unit = aoi::screen::unit(image.width(), image.height());
    let digit = (DIGIT * unit).round() as u32;
    let (_, y0, x1, y1) = student::rect(image, schedule::NAME_AREA).expect("the name's area");
    let (left, right) = rank_box(image);
    // Short of the box's rounded end, a digit's columns go.
    let cut = right - (0.006 * unit).round() as u32 - digit;
    assert!(cut > left, "the box is wider than a digit");
    let mut narrowed = image.clone();
    for y in y0..y1 {
        for x in cut..x1 {
            let from = (x + digit).min(x1 - 1);
            narrowed.put_pixel(x, y, *image.get_pixel(from, y));
        }
    }
    narrowed
}

/// The rank's box is not looked for at a fixed place: with the box a digit
/// narrower and the name as much nearer, as a location of a rank under 10
/// would draw them, every location reads the same.
#[test]
fn a_location_of_a_one_digit_rank_reads_the_same() {
    let recognizer = crate::common::recognizer();
    let screens = chosen();
    let wrong: Vec<String> = crate::common::each(&screens, |(fixture, image)| {
        let location = labeled(fixture);
        let narrowed = of_one_digit(image);
        // The box did narrow, by about a sixth (a letter of its six).
        let (wide, narrow) = (rank_box(image), rank_box(&narrowed));
        let share = (narrow.1 - narrow.0) as f32 / (wide.1 - wide.0) as f32;
        assert!((0.80..0.88).contains(&share), "{}: {share}", fixture.image);
        let told = (Some(location.name), Some(Place::Schedule(location.school)));
        wrong(&fixture.image, &narrowed, &told, |shot| {
            (
                recognizer.read_location(shot).map(|l| l.name),
                aoi::watch::place_of(recognizer, shot),
            )
        })
    })
    .concat();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The list of locations writes locations' names where the chosen screen names
/// its own, so a name is read under the chosen screen's title alone: with the
/// title gone (as under a dialog, or on the list), the name at the top right
/// tells no place.
#[test]
fn a_location_s_name_without_the_chosen_title_tells_no_place() {
    let recognizer = crate::common::recognizer();
    let named: Vec<Fixture> = fixtures()
        .into_iter()
        .filter(|f| f.location.is_some() && f.image != TITLED)
        .collect();
    assert_eq!(named.len(), schedule::LOCATIONS.len() - 1);
    for fixture in named {
        let shot = aoi::screen::normalize(&open(&fixture.image)).into_owned();
        assert!(
            recognizer.read_location(&shot).is_some(),
            "{}",
            fixture.image
        );
        assert_eq!(
            aoi::watch::place_of(recognizer, &shot),
            None,
            "{}",
            fixture.image
        );
    }
}
