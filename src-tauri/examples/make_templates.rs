//! Writes the pictures the screen is compared with: `data/sort-default.png`, the
//! sort buttons in the default order ("デフォルト ≡↑"), and `data/title-items.png`
//! and `data/title-equipment.png`, the screens' titles, from screenshots of the
//! item list and the equipment list sorted that way; `data/title-students.png`
//! and `data/student-tab.png` (the selected "基本情報" tab) from one of the
//! student screen, and `data/student-tab-other.png` (the same tab while
//! another is selected) from one of the student screen on the レベルアップ tab.
//! Run with `cargo run --example make_templates -- <item list.png> <equipment list.png> <student.png> <student on レベルアップ.png>`.

use std::path::Path;

use aoi::{grid, screen};

fn main() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut args = std::env::args().skip(1);
    let (items, equipment) = (
        args.next().expect("item list"),
        args.next().expect("equipment list"),
    );
    let student = args.next().expect("student screen");
    let other_tab = args.next().expect("student screen on another tab");
    let open = |path: &str| {
        let image = image::open(path).unwrap().to_rgb8();
        let unit = grid::find_panel(&image).expect("a list").card_height;
        (image, unit)
    };
    let (image, unit) = open(&items);
    let save = |name: &str, picture: image::RgbImage| {
        picture.save(data.join(name)).unwrap();
        println!("{name}: {}x{}", picture.width(), picture.height());
    };
    save(
        "sort-default.png",
        screen::crop(
            &image,
            unit,
            screen::SORT_BUTTONS,
            (0.0, 0.0),
            screen::HEIGHT,
        )
        .expect("sort buttons"),
    );
    save(
        "title-items.png",
        screen::crop(
            &image,
            unit,
            screen::TITLE,
            (0.0, 0.0),
            screen::TITLE_HEIGHT,
        )
        .expect("title"),
    );
    let (image, unit) = open(&equipment);
    save(
        "title-equipment.png",
        screen::crop(
            &image,
            unit,
            screen::TITLE,
            (0.0, 0.0),
            screen::TITLE_HEIGHT,
        )
        .expect("title"),
    );
    let image = image::open(&student).unwrap().to_rgb8();
    let unit = (screen::unit(image.width(), image.height()) * screen::STUDENT_UNIT).round() as u32;
    save(
        "title-students.png",
        screen::crop(
            &image,
            unit,
            screen::TITLE,
            (0.0, 0.0),
            screen::TITLE_HEIGHT,
        )
        .expect("title"),
    );
    save(
        "student-tab.png",
        screen::crop(
            &image,
            unit,
            screen::STUDENT_TAB,
            (0.0, 0.0),
            screen::HEIGHT,
        )
        .expect("tab"),
    );
    let image = image::open(&other_tab).unwrap().to_rgb8();
    let unit = (screen::unit(image.width(), image.height()) * screen::STUDENT_UNIT).round() as u32;
    save(
        "student-tab-other.png",
        screen::crop(
            &image,
            unit,
            screen::STUDENT_TAB,
            (0.0, 0.0),
            screen::HEIGHT,
        )
        .expect("tab"),
    );
}
