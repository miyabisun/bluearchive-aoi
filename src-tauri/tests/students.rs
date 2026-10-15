//! The student screen read against screenshots labeled by eye
//! (tests/fixtures/students.json): every field reads right or unknown, never wrong.

mod common;

use std::path::Path;

use aoi::student::{self, Label, LevelLabel, Training};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn labels() -> Vec<Label> {
    serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/students.json")).unwrap(),
    )
    .unwrap()
}

/// The fields of a training, named, each `None` when unknown.
fn fields(t: &Training) -> Vec<(String, Option<String>)> {
    let show = |v: Option<String>| v;
    let mut out = vec![
        ("level".to_owned(), show(t.level.map(|v| v.to_string()))),
        ("rarity".to_owned(), t.rarity.map(|v| v.to_string())),
        ("bond".to_owned(), t.bond.map(|v| v.to_string())),
        ("weapon".to_owned(), t.weapon.map(|w| format!("{w:?}"))),
        ("gear".to_owned(), t.gear.map(|v| v.to_string())),
    ];
    for (i, s) in t.skills.iter().enumerate() {
        out.push((format!("skill {i}"), s.map(|v| v.to_string())));
    }
    for (i, e) in t.equipment.iter().enumerate() {
        out.push((format!("equipment {i}"), e.map(|g| format!("{g:?}"))));
    }
    for (i, a) in t.ability.iter().enumerate() {
        out.push((format!("ability {i}"), a.map(|v| v.to_string())));
    }
    out
}

/// Every field of the labeled screens reads right or not at all, and nearly
/// every one right: at most `UNKNOWN` of them unknown (measured on
/// 2026-10-15, read as Aoi reads them, scaled down to Full HD: 22 of 630).
#[test]
fn every_field_reads_right_or_unknown() {
    const UNKNOWN: f32 = 0.05;
    let recognizer = recognizer();
    let (mut wrong, mut unknown, mut total) = (Vec::new(), Vec::new(), 0);
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        // As Aoi reads it, scaled down to a width read at.
        let image = aoi::screen::normalize(&image).into_owned();
        let read = fields(&student::read(&image, recognizer.references(&image).1));
        let want = fields(&label.training());
        for ((name, got), (_, want)) in read.into_iter().zip(want) {
            total += 1;
            match got {
                None => unknown.push(format!("{} {name}", label.image)),
                Some(got) if Some(&got) != want.as_ref() => {
                    wrong.push(format!(
                        "{} {name}: read {got}, is {}",
                        label.image,
                        want.unwrap()
                    ));
                }
                Some(_) => {}
            }
        }
    }
    if unknown.len() as f32 > UNKNOWN * total as f32 {
        wrong.push(format!(
            "{} of {total} fields unknown:\n{}",
            unknown.len(),
            unknown.join("\n")
        ));
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// A screen of any width read, and any between them (a window as it was
/// resized), reads every field of the student screen right or not at all,
/// and at each width at least `LEAST` of them right. Measured on 2026-10-16
/// over the fixtures at widths from 1500 to 2222: 89% to 99%.
#[test]
fn a_smaller_screen_reads_fields_right_or_not_at_all() {
    const LEAST: f32 = 0.85;
    let recognizer = recognizer();
    let mut wrong = Vec::new();
    for width in [2222, 2100, 1920, 1760, 1600, 1550, 1500] {
        let (mut right, mut total) = (0, 0);
        for label in labels() {
            let image = image::open(root().join("tests/fixtures").join(&label.image))
                .unwrap()
                .to_rgb8();
            let image = common::drawn_at(&image, width);
            let read = fields(&student::read(&image, recognizer.references(&image).1));
            for ((name, got), (_, want)) in read.into_iter().zip(fields(&label.training())) {
                total += 1;
                if got.is_some() && got != want {
                    wrong.push(format!(
                        "{width} {} {name}: read {got:?}, is {want:?}",
                        label.image
                    ));
                } else if got.is_some() {
                    right += 1;
                }
            }
        }
        if (right as f32) < LEAST * total as f32 {
            wrong.push(format!("{width}: {right} of {total} fields read"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

fn recognizer() -> aoi::Recognizer {
    aoi::Recognizer::load(&root().join("data")).unwrap()
}

/// Every student screen is told as such, its student named right or not at
/// all, and nearly all of them named (41 of 42 on 2026-10-15).
#[test]
fn each_student_screen_names_its_student() {
    let recognizer = recognizer();
    let (mut named, mut screens) = (0, 0);
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        // As Aoi reads it, scaled down to a width read at.
        let image = aoi::screen::normalize(&image).into_owned();
        match recognizer.read_frame(&image) {
            Some(aoi::Frame::Student(id, _)) => {
                screens += 1;
                if let Some(id) = id {
                    assert_eq!(id, label.id, "{}", label.image);
                    named += 1;
                }
            }
            other => panic!("{}: {other:?}", label.image),
        }
    }
    assert!(named * 10 >= screens * 9, "{named} of {screens} named");
}

/// A student missing from the snapshot (a new release) is never named as
/// another: each labeled student is left out in turn.
#[test]
fn a_student_missing_from_the_snapshot_is_never_named_as_another() {
    let recognizer = recognizer();
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        // As Aoi reads it, scaled down to a width read at.
        let image = aoi::screen::normalize(&image).into_owned();
        let glyphs = recognizer.references(&image).0;
        let name = &recognizer
            .students()
            .iter()
            .find(|(id, _)| *id == label.id)
            .unwrap()
            .1;
        let others = glyphs.index.names.iter().filter(|w| w.text != *name);
        assert_eq!(
            student::name::read(&image, glyphs, others),
            None,
            "{}",
            label.image
        );
    }
}

/// On a smaller screen (a phone's, or a stream of one) as well, a student
/// missing from the snapshot is never named as another, and one in it is
/// named right or not at all.
#[test]
fn a_smaller_screen_names_its_student_right_or_not_at_all() {
    let recognizer = recognizer();
    let mut wrong = Vec::new();
    for width in [2222, 2100, 1920, 1760, 1600, 1550, 1500] {
        for label in labels() {
            let image = image::open(root().join("tests/fixtures").join(&label.image))
                .unwrap()
                .to_rgb8();
            let image = common::drawn_at(&image, width);
            let glyphs = recognizer.references(&image).0;
            let name = &recognizer
                .students()
                .iter()
                .find(|(id, _)| *id == label.id)
                .unwrap()
                .1;
            if let Some(id) = student::name::read(&image, glyphs, &glyphs.index.names)
                && id != label.id
            {
                wrong.push(format!("{width} {}: {id}", label.image));
            }
            let others = glyphs.index.names.iter().filter(|w| w.text != *name);
            if let Some(id) = student::name::read(&image, glyphs, others) {
                wrong.push(format!("{width} {} left out: {id}", label.image));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// Lists are never read as a student screen, nor other screens.
#[test]
fn other_screens_are_no_student_screen() {
    let recognizer = recognizer();
    let dir = root().join("tests/fixtures");
    for name in [
        "items-16x9-secretstone.png",
        "items-4x3-ooparts.png",
        "equipment-16x9-end.png",
        "other/home.png",
        "other/momotalk.png",
        "other/event-stages.png",
    ] {
        let image = image::open(dir.join(name)).unwrap().to_rgb8();
        assert!(recognizer.read_student(&image).is_none(), "{name}");
    }
}

/// The glyph bank holds every student's name: it was built again after
/// SchaleDB's students last changed (`examples/build_glyphs.rs`).
#[test]
fn every_student_has_a_name_to_be_told_by() {
    let recognizer = recognizer();
    let words = &recognizer.glyphs().index.names;
    for (id, name) in recognizer.students() {
        assert!(words.iter().any(|w| w.text == *name), "{id} {name}");
    }
}

/// A student SchaleDB does not list yet is named from the extra list
/// (`data/extra/students.json`) under an id of Aoi's own, below zero.
#[test]
fn a_student_schaledb_lacks_is_named_from_the_extra_list() {
    let recognizer = recognizer();
    let word = recognizer
        .glyphs()
        .index
        .names
        .iter()
        .find(|w| w.text == "ツムギ")
        .unwrap();
    assert!(word.id < 0, "{}", word.id);
}

fn weapon_blue(c: [u8; 3]) -> bool {
    c[2] > 220 && c[1] > 180 && c[0] < 180
}

/// The weapon's stars are counted on every screen with a weapon, Eimi's too:
/// the tag hanging from her gun beside them is no star.
#[test]
fn the_weapon_stars_read_on_every_screen() {
    let recognizer = recognizer();
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        // As Aoi reads it, scaled down to a width read at.
        let image = aoi::screen::normalize(&image).into_owned();
        let read = student::read(&image, recognizer.references(&image).1);
        // Read right or not at all; Eimi's, with the tag beside, read.
        if read.weapon.is_some() || label.image.ends_with("eimi.png") {
            assert_eq!(read.weapon, label.training().weapon, "{}", label.image);
        }
    }
}

/// No weapon has five stars yet, but one may: Eimi's four with a fifth pasted
/// a star's pitch left of them read as five.
#[test]
fn a_fifth_weapon_star_counts() {
    let mut image = image::open(root().join("tests/fixtures/students/eimi.png"))
        .unwrap()
        .to_rgb8();
    // The stars stand right-aligned 0.0228 window heights apart, in the rows
    // 0.711 to 0.734 down, ending left of the light blue 固有武器 button
    // (0.258 window heights from the right edge).
    let (h, pitch) = (
        image.height() as f32,
        (0.0228 * image.height() as f32).round() as u32,
    );
    let rows = (0.711 * h) as u32..(0.734 * h) as u32;
    let right = (0..(image.width() as f32 - 0.258 * h) as u32)
        .rev()
        .find(|&x| rows.clone().any(|y| weapon_blue(image.get_pixel(x, y).0)))
        .unwrap();
    let left_star = right + 1 - 4 * pitch;
    for x in left_star..left_star + pitch {
        for y in rows.clone() {
            let p = *image.get_pixel(x, y);
            image.put_pixel(x - pitch, y, p);
        }
    }
    let read = student::read(&image, recognizer().references(&image).1);
    assert_eq!(
        read.weapon,
        Some(Some(student::Weapon {
            level: 60,
            stars: 5
        }))
    );
}

/// Every level sample (`levels.json`) reads its level, or nothing.
#[test]
fn every_level_sample_reads_right_or_unknown() {
    let recognizer = recognizer();
    let levels: Vec<LevelLabel> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/levels.json")).unwrap(),
    )
    .unwrap();
    let mut unknown = Vec::new();
    for label in &levels {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        // As Aoi reads it, scaled down to a width read at.
        let image = aoi::screen::normalize(&image).into_owned();
        match student::level(&image, recognizer.references(&image).1) {
            None => unknown.push(label.image.as_str()),
            Some(read) => assert_eq!(read, label.level, "{}", label.image),
        }
    }
    eprintln!(
        "{} of {} levels unknown: {unknown:?}",
        unknown.len(),
        levels.len()
    );
}

/// On the other tabs of the student screen, where a student is levelled up,
/// the name plate is read and nothing else (on a Full HD screen).
#[test]
fn the_level_up_tab_reads_the_name_plate() {
    let image = image::open(root().join("tests/fixtures/students/koyuki-pajama-levelup.png"))
        .unwrap()
        .to_rgb8();
    let image = common::drawn_at(&image, 1920);
    let (id, read) = recognizer().read_student(&image).expect("a student screen");
    assert_eq!(id, Some(20056));
    assert_eq!(
        (read.level, read.rarity, read.bond),
        (Some(22), Some(3), Some(12))
    );
    assert_eq!(read.skills, [None; 4]);
    assert_eq!(read.weapon, None);
    assert_eq!(read.equipment, [None; 3]);
    assert_eq!(read.gear, None);
    assert_eq!(read.ability, [None; 3]);
}
