//! The student screen read against screenshots labeled by eye
//! (tests/fixtures/students.json): every field reads right or unknown, never wrong.

use std::path::Path;

use aoi::student::{self, Label, LevelLabel, Training, glyph::Fonts};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn labels() -> Vec<Label> {
    serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/students.json")).unwrap(),
    )
    .unwrap()
}

fn fonts() -> Fonts {
    serde_json::from_str(&std::fs::read_to_string(root().join("data/student-fonts.json")).unwrap())
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

#[test]
fn every_field_reads_right_or_unknown() {
    let fonts = fonts();
    let (mut wrong, mut unknown, mut total) = (Vec::new(), Vec::new(), 0);
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        let read = fields(&student::read(&image, &fonts));
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
    eprintln!(
        "{} of {total} fields unknown:\n{}",
        unknown.len(),
        unknown.join("\n")
    );
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

fn recognizer() -> aoi::Recognizer {
    aoi::Recognizer::load(&root().join("data")).unwrap()
}

/// Every student screen is told as such, with the student named from the OCR.
#[test]
fn each_student_screen_names_its_student() {
    let recognizer = recognizer();
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        match recognizer.read_frame(&image) {
            Some(aoi::Frame::Student(id, _)) => assert_eq!(id, Some(label.id), "{}", label.image),
            other => panic!("{}: {other:?}", label.image),
        }
    }
}

/// A student missing from the snapshot (a new release) is never named as
/// another: each labeled student is left out in turn.
#[test]
fn a_student_missing_from_the_snapshot_is_never_named_as_another() {
    let recognizer = recognizer();
    let ocr = aoi::ocr::Ocr::japanese().unwrap();
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        let (x0, y0, x1, y1) = student::rect(&image, student::NAME).unwrap();
        let text = ocr
            .read(&image::imageops::crop_imm(&image, x0, y0, x1 - x0, y1 - y0).to_image())
            .unwrap();
        let without: Vec<(i64, String)> = recognizer
            .students()
            .iter()
            .filter(|(id, _)| *id != label.id)
            .cloned()
            .collect();
        assert_eq!(
            student::name::find(&text, &without),
            None,
            "{}: {text}",
            label.image
        );
    }
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

/// Folding the characters the OCR mixes up never makes two different names one.
#[test]
fn folded_names_stay_apart() {
    let recognizer = recognizer();
    let mut names: Vec<&String> = recognizer.students().iter().map(|(_, n)| n).collect();
    names.sort();
    names.dedup();
    let mut folded: Vec<String> = names.iter().map(|n| student::name::fold(n)).collect();
    folded.sort();
    folded.dedup();
    assert_eq!(folded.len(), names.len());
}

/// Whether `c` is the light blue of the unique weapon's stars.
fn weapon_blue(c: [u8; 3]) -> bool {
    c[2] > 220 && c[1] > 180 && c[0] < 180
}

/// The weapon's stars are counted on every screen with a weapon, Eimi's too:
/// the tag hanging from her gun beside them is no star.
#[test]
fn the_weapon_stars_read_on_every_screen() {
    let fonts = fonts();
    for label in labels() {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        let read = student::read(&image, &fonts);
        assert_eq!(read.weapon, label.training().weapon, "{}", label.image);
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
    let read = student::read(&image, &fonts());
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
    let fonts = fonts();
    let levels: Vec<LevelLabel> = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/fixtures/levels.json")).unwrap(),
    )
    .unwrap();
    let mut unknown = Vec::new();
    for label in &levels {
        let image = image::open(root().join("tests/fixtures").join(&label.image))
            .unwrap()
            .to_rgb8();
        match student::level(&image, &fonts) {
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
/// the name plate is read and nothing else.
#[test]
fn the_level_up_tab_reads_the_name_plate() {
    let image = image::open(root().join("tests/fixtures/students/koyuki-pajama-levelup.png"))
        .unwrap()
        .to_rgb8();
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
