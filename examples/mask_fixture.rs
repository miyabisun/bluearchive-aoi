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
//! (`tests/fixtures/names.json`); with `title`, the title at the top left, the
//! 任務 screen's campaign and the shop screen's menu (`tests/fixtures/titles.json`);
//! with `place`, the title and the chosen location's name at the top right of
//! the schedule, with `location`, that name alone (the title is the same at
//! every location), and with `room`, the first room's name on the card of its
//! dialog (`tests/fixtures/schedule.json`).
//!
//! `cargo run --release --example mask_fixture -- <screenshot.png> <fixture.png> students[:<part>,...] [<fixture below.png>]`
//! keeps what the student list is read from (`aoi::student::list::areas`: the
//! parts `title`, `narrow`, `tabs`, `search`, `sort` and `view`, the insides of
//! the cards' pictures and the pins black), or the parts named alone: a screen
//! that differs from another fixture in a part of its header keeps that part,
//! and the tests lay it over the other (`tests/fixtures/student-lists.json`).
//! Refuses when what is kept, laid over the fixture below where one is given,
//! tells otherwise than the screenshot.

use std::path::Path;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use aoi::{
    Recognizer, campaign, credit, grid, schedule,
    screen::{self, Region},
    shop, student, title,
};

/// Room kept around each area, in units (see `screen::unit`).
const ROOM: f32 = 0.02;
/// Room kept around the schedule's areas: the map shows around them, which
/// costs more than their text to keep, and the black beyond comes no nearer to
/// them than a screen drawn 1500 wide blends (under two pixels at 2370).
const SCHEDULE_ROOM: f32 = 0.006;
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
        Some("title") => [title::AREA, campaign::LABEL_AREA]
            .into_iter()
            .chain(campaign::TEXTS.iter().map(|t| campaign::text_area(t.to)))
            .chain(shop::areas())
            .collect(),
        Some("place") => vec![title::AREA, schedule::NAME_AREA],
        Some("location") => vec![schedule::NAME_AREA],
        Some("room") => vec![schedule::ROOM_AREA],
        Some(_) => vec![credit::NUMBER, credit::ICON],
        None => student::areas(),
    };
    let room = match only {
        Some("place" | "location" | "room") => SCHEDULE_ROOM * u,
        _ => ROOM * u,
    };
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

/// The student list (see the top): `parts` of `from` kept, all for none
/// named, and checked laid over `below`.
fn students(from: &str, to: &str, parts: &str, below: Option<&String>) {
    let image = image::open(from).expect("the image opens").to_rgb8();
    let areas = student::list::areas(&image);
    let named: Vec<&str> = parts.split(',').filter(|part| !part.is_empty()).collect();
    for part in &named {
        assert!(
            areas.parts.iter().any(|(name, _)| name == part),
            "{part} is no part of the student list"
        );
    }
    let kept: Vec<(u32, u32, u32, u32)> = areas
        .parts
        .iter()
        .filter(|(name, _)| named.is_empty() || named.contains(name))
        .flat_map(|(_, rects)| rects.clone())
        .collect();
    let within = |rects: &[(u32, u32, u32, u32)], x: u32, y: u32| {
        rects
            .iter()
            .any(|&(x0, y0, x1, y1)| x >= x0 && x < x1 && y >= y0 && y < y1)
    };
    let masked = image::RgbImage::from_fn(image.width(), image.height(), |x, y| {
        if within(&kept, x, y) && !within(&areas.unread, x, y) {
            *image.get_pixel(x, y)
        } else {
            image::Rgb([0, 0, 0])
        }
    });
    // As the tests show it: over the fixture below, the parts kept.
    let shown = match below {
        None => masked.clone(),
        Some(below) => {
            let below = image::open(below).expect("the image opens").to_rgb8();
            assert_eq!(below.dimensions(), masked.dimensions(), "of one size");
            image::RgbImage::from_fn(below.width(), below.height(), |x, y| {
                let from = if within(&kept, x, y) { &masked } else { &below };
                *from.get_pixel(x, y)
            })
        }
    };
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let recognizer = Recognizer::load(&data).expect("the recognizer loads data/");
    // Both read as Aoi reads them, scaled down to a width read at; the faces
    // are of the pictures blacked out.
    let told = |screen: &image::RgbImage| {
        let screen = screen::normalize(screen);
        let header = student::list::header(&screen, recognizer.student_sorts());
        let stretch = recognizer.read_student_list(&screen).map(|seen| seen.sight);
        (header, stretch)
    };
    assert_eq!(
        told(&image),
        told(&shown),
        "{from} tells otherwise once masked"
    );
    save(&masked, to);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.get(2).and_then(|only| only.strip_prefix("students")) {
        Some(parts) => {
            let parts = parts.strip_prefix(':').unwrap_or(parts);
            students(&args[0], &args[1], parts, args.get(3));
        }
        None => others(&args),
    }
}

/// Every screen but the student list (see the top).
fn others(args: &[String]) {
    let (from, to, only) = match args {
        [from, to] => (from, to, None),
        [from, to, only]
            if [
                "level", "credit", "shop", "list", "name", "title", "place", "location", "room",
            ]
            .contains(&only.as_str()) =>
        {
            (from, to, Some(only.as_str()))
        }
        _ => panic!(
            "usage: mask_fixture <screenshot.png> <fixture.png> [level|credit|shop|list|name|title|place|location|room|students[:<part>,...] [<fixture below.png>]]"
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
    if only == Some("title") {
        let (whole, part) = (title::ink(&image), title::ink(&read));
        assert!(whole.is_some(), "{from} shows no title");
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        assert_eq!(
            campaign::label(&image),
            campaign::label(&read),
            "{from}'s label reads otherwise"
        );
        for t in &campaign::TEXTS {
            let (whole, part) = (campaign::text(&image, t.to), campaign::text(&read, t.to));
            assert_eq!(
                whole, part,
                "{from}'s campaign text reads otherwise once masked"
            );
        }
        assert_eq!(
            shop::menu_ink(&image),
            shop::menu_ink(&read),
            "{from}'s shop menu reads otherwise once masked"
        );
        return save(&masked, to);
    }
    if matches!(only, Some("place" | "location" | "room")) {
        schedule_reads_the_same(only, from, &image, &read);
        return save(&masked, to);
    }
    if only == Some("name") {
        let (whole, part) = (student::name::plate(&image), student::name::plate(&read));
        assert!(whole.is_some(), "{from} shows no name plate");
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
    let recognizer = Recognizer::load(&data).expect("the recognizer loads data/");
    if only == Some("level") {
        // Masking keeps the size, so both are read with the same width's references.
        let fonts = recognizer.references(&image).1;
        let (whole, part) = (student::level(&image, fonts), student::level(&read, fonts));
        assert_eq!(whole, part, "{from} reads otherwise once masked");
        return save(&masked, to);
    }
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

/// Refuses a fixture of the schedule (`only`: `place`, `location` or `room`) whose letters
/// are cut otherwise from what is kept (`read`) than from the screenshot (`image`).
fn schedule_reads_the_same(
    only: Option<&str>,
    from: &str,
    image: &image::RgbImage,
    read: &image::RgbImage,
) {
    if only == Some("place") {
        let (whole, part) = (title::ink(image), title::ink(read));
        assert!(whole.is_some(), "{from} shows no title");
        assert_eq!(whole, part, "{from}'s title reads otherwise once masked");
    }
    let (whole, part) = if only == Some("room") {
        (schedule::room_ink(image), schedule::room_ink(read))
    } else {
        (schedule::name_ink(image), schedule::name_ink(read))
    };
    assert!(whole.is_some(), "{from} names nothing of the schedule");
    assert_eq!(whole, part, "{from} reads otherwise once masked");
}

/// Writes `image` with the best compression: a fixture is written once and kept for good.
fn save(image: &image::RgbImage, to: &str) {
    let file =
        std::io::BufWriter::new(std::fs::File::create(to).expect("the output file is created"));
    let png = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
    image.write_with_encoder(png).expect("the PNG encodes");
}
