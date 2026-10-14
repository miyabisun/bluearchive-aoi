//! Keeps of a screenshot of the student screen only what Aoi reads (every area
//! of `student::areas`, and where the screen is told by its title and tab) with
//! room to spare, black elsewhere: a fixture then costs a fraction of the
//! screenshot. Refuses when what is kept reads otherwise than the screenshot.
//! `cargo run --release --example mask_fixture -- <screenshot.png> <fixture.png>`
//! With `level` after them it keeps the name plate's level alone, for a level
//! sample taken on any tab (`tests/fixtures/levels.json`); with `credit`, the
//! top bar's credits alone (`tests/fixtures/credits.json`); with `shop`, what
//! the shop screen is read from (`tests/fixtures/shops.json`).

use std::path::Path;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use aoi::{
    Recognizer, credit,
    glyph::bank::Bank,
    screen::{self, Region},
    shop,
    student::{self, Area},
};

/// Room kept around each area, in window heights.
const ROOM: f32 = 0.02;
/// Room kept around the title and the tab, in card heights (beyond the slack
/// the screen is searched with).
const SCREEN_ROOM: f32 = 0.5;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (from, to, only) = match &args[..] {
        [from, to] => (from, to, None),
        [from, to, only] if ["level", "credit", "shop"].contains(&only.as_str()) => {
            (from, to, Some(only.as_str()))
        }
        _ => panic!("usage: mask_fixture <screenshot.png> <fixture.png> [level|credit|shop]"),
    };
    let image = image::open(from).unwrap().to_rgb8();
    let (w, h) = (image.width() as f32, image.height() as f32);
    let areas = match only {
        Some("level") => vec![student::LEVEL],
        Some("shop") => shop::areas(),
        Some(_) => vec![credit::NUMBER, credit::ICON],
        None => student::areas(),
    };
    let mut kept: Vec<(f32, f32, f32, f32)> = areas
        .into_iter()
        .map(|Area { from_right, x, y }| {
            let (x0, x1) = if from_right {
                (w - x.0 * h, w - x.1 * h)
            } else {
                (x.0 * h, x.1 * h)
            };
            (
                x0 - ROOM * h,
                y.0 * h - ROOM * h,
                x1 + ROOM * h,
                y.1 * h + ROOM * h,
            )
        })
        .collect();
    if only == Some("shop") {
        // The cards read whole, each with a little room.
        let room = 0.005 * h;
        kept.extend(shop::bars(&image).iter().filter_map(|bar| {
            let (x0, y0, x1, y1) = shop::card_area(&image, bar)?;
            Some((x0 - room, y0 - room, x1 + room, y1 + room))
        }));
    }
    let unit = screen::STUDENT_UNIT * h;
    let screens: &[Region] = if only.is_some() {
        &[]
    } else {
        &[screen::TITLE, screen::STUDENT_TAB]
    };
    for &Region { from_right, x, y } in screens {
        let (x0, x1) = if from_right {
            (w - x.0 * unit, w - x.1 * unit)
        } else {
            (x.0 * unit, x.1 * unit)
        };
        let room = SCREEN_ROOM * unit;
        kept.push((x0 - room, y.0 * unit - room, x1 + room, y.1 * unit + room));
    }
    let masked = image::RgbImage::from_fn(image.width(), image.height(), |px, py| {
        let (fx, fy) = (px as f32, py as f32);
        if kept
            .iter()
            .any(|&(x0, y0, x1, y1)| fx >= x0 && fx < x1 && fy >= y0 && fy < y1)
        {
            *image.get_pixel(px, py)
        } else {
            image::Rgb([0, 0, 0])
        }
    });
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    if only == Some("level") {
        let fonts = Bank::load(&data).unwrap().fonts();
        let (whole, part) = (
            student::level(&image, &fonts),
            student::level(&masked, &fonts),
        );
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    let recognizer = Recognizer::load(&data).unwrap();
    if only == Some("shop") {
        let (whole, part) = (recognizer.read_shop(&image), recognizer.read_shop(&masked));
        assert!(whole.is_some(), "{from} is no shop screen");
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    if only == Some("credit") {
        let (whole, part) = (
            recognizer.read_credits(&image),
            recognizer.read_credits(&masked),
        );
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    let (whole, part) = (
        recognizer.read_student(&image),
        recognizer.read_student(&masked),
    );
    assert!(whole.is_some(), "{from} is no student screen");
    assert_eq!(whole, part, "{from} reads otherwise once masked");
    save(&masked, to);
}

/// Writes `image` with the best compression: a fixture is written once and kept for good.
fn save(image: &image::RgbImage, to: &str) {
    let file = std::io::BufWriter::new(std::fs::File::create(to).unwrap());
    let png = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
    image.write_with_encoder(png).unwrap();
}
