//! Keeps of a screenshot of the student screen only what Aoi reads (every area
//! of `student::areas`, and where the screen is told by its title and tab) with
//! room to spare, black elsewhere: a fixture then costs a fraction of the
//! screenshot. Refuses when what is kept reads otherwise than the screenshot.
//! `cargo run --release --example mask_fixture -- <screenshot.png> <fixture.png>`
//! With `level` after them it keeps the name plate's level alone, for a level
//! sample taken on any tab (`tests/fixtures/levels.json`); with `credit`, the
//! top bar's credits alone (`tests/fixtures/credits.json`); with `shop`, what
//! the shop screen is read from (`tests/fixtures/shops.json`); with `list`, the
//! item or equipment list's panel, read with the list the screen names
//! (`tests/fixtures/tablet.json`); with `name`, the name plate's name alone
//! (`tests/fixtures/names.json`).

use std::path::Path;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use aoi::{
    Recognizer, credit,
    glyph::bank::Bank,
    grid,
    screen::{self, Region},
    shop, student,
};

/// Room kept around each area, in units (see `screen::unit`).
const ROOM: f32 = 0.02;
/// Room kept around the title and the tab, in card heights (beyond the slack
/// the screen is searched with).
const SCREEN_ROOM: f32 = 0.5;

/// The rectangles kept of `image` (see the top), for `only` one part or none.
fn kept(image: &image::RgbImage, only: Option<&str>) -> Vec<(f32, f32, f32, f32)> {
    let w = image.width() as f32;
    let u = screen::unit(image.width(), image.height());
    if only == Some("list") {
        let panel = grid::find_panel(image).expect("no list panel");
        return vec![(
            panel.left as f32,
            panel.top as f32,
            panel.right as f32,
            image.height() as f32,
        )];
    }
    let areas = match only {
        Some("level") => vec![student::LEVEL],
        Some("name") => vec![student::NAME],
        Some("shop") => shop::areas(),
        Some(_) => vec![credit::NUMBER, credit::ICON],
        None => student::areas(),
    };
    let room = ROOM * u;
    let mut kept: Vec<(f32, f32, f32, f32)> = areas
        .into_iter()
        .filter_map(|area| student::rect(image, area))
        .map(|(x0, y0, x1, y1)| {
            (
                x0 as f32 - room,
                y0 as f32 - room,
                x1 as f32 + room,
                y1 as f32 + room,
            )
        })
        .collect();
    if only == Some("shop") {
        // The cards read whole, each with a little room, and every card's bar,
        // which tells the screen a shop with none whole.
        let room = 0.005 * u;
        for bar in shop::bars(image) {
            kept.push((
                bar.left as f32 - room,
                bar.top as f32 - room,
                bar.right as f32 + room,
                bar.bottom as f32 + room,
            ));
            if let Some((x0, y0, x1, y1)) = shop::card_area(image, &bar) {
                kept.push((x0 - room, y0 - room, x1 + room, y1 + room));
            }
        }
    }
    let unit = screen::STUDENT_UNIT * u;
    let screens: &[Region] = if only.is_some() {
        &[]
    } else {
        &[screen::TITLE, screen::STUDENT_TAB]
    };
    for &Region {
        from_right,
        x,
        y,
        anchor,
    } in screens
    {
        let (x0, x1) = if from_right {
            (w - x.0 * unit, w - x.1 * unit)
        } else {
            (x.0 * unit, x.1 * unit)
        };
        let origin = anchor.origin(image.height(), u);
        let room = SCREEN_ROOM * unit;
        kept.push((
            x0 - room,
            origin + y.0 * unit - room,
            x1 + room,
            origin + y.1 * unit + room,
        ));
    }
    kept
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (from, to, only) = match &args[..] {
        [from, to] => (from, to, None),
        [from, to, only]
            if ["level", "credit", "shop", "list", "name"].contains(&only.as_str()) =>
        {
            (from, to, Some(only.as_str()))
        }
        _ => panic!(
            "usage: mask_fixture <screenshot.png> <fixture.png> [level|credit|shop|list|name]"
        ),
    };
    let image = image::open(from).expect("the image opens").to_rgb8();
    let kept = kept(&image, only);
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
    // Both read as Aoi reads them, scaled down to a width read at.
    let (image, read) = (
        screen::normalize(&image).into_owned(),
        screen::normalize(&masked).into_owned(),
    );
    if only == Some("level") {
        let fonts = Bank::load(&data).expect("the glyph bank loads").fonts();
        let (whole, part) = (
            student::level(&image, &fonts),
            student::level(&read, &fonts),
        );
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    if only == Some("name") {
        let (whole, part) = (student::name::plate(&image), student::name::plate(&read));
        assert!(whole.is_some(), "{from} shows no name plate");
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    let recognizer = Recognizer::load(&data).expect("the recognizer loads data/");
    if only == Some("list") {
        let (list, _, _) = recognizer.read_list(&image).expect("no list");
        let (whole, part) = (recognizer.read(&image, list), recognizer.read(&read, list));
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    if only == Some("shop") {
        let (whole, part) = (recognizer.read_shop(&image), recognizer.read_shop(&read));
        assert!(whole.is_some(), "{from} is no shop screen");
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    if only == Some("credit") {
        let (whole, part) = (
            recognizer.read_credits(&image),
            recognizer.read_credits(&read),
        );
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    let (whole, part) = (
        recognizer.read_student(&image),
        recognizer.read_student(&read),
    );
    assert!(whole.is_some(), "{from} is no student screen");
    assert_eq!(whole, part, "{from} reads otherwise once masked");
    save(&masked, to);
}

/// Writes `image` with the best compression: a fixture is written once and kept for good.
fn save(image: &image::RgbImage, to: &str) {
    let file =
        std::io::BufWriter::new(std::fs::File::create(to).expect("the output file is created"));
    let png = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
    image.write_with_encoder(png).expect("the PNG encodes");
}
