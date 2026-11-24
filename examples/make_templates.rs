//! Writes the pictures the screen is compared with: `data/sort-default.png`, the
//! sort buttons in the default order ("デフォルト ≡↑"), and `data/title-items.png`
//! and `data/title-equipment.png`, the screens' titles, from screenshots of the
//! item list and the equipment list sorted that way; `data/title-students.png`
//! and `data/student-tab.png` (the selected "基本情報" tab) from one of the
//! student screen, and `data/student-tab-other.png` (the same tab while
//! another is selected) from one of the student screen on the レベルアップ tab.
//! Run with `cargo run --example make_templates -- <item list.png> <equipment list.png> <student.png> <student on レベルアップ.png>`.
//!
//! `cargo run --example make_templates -- student-list <newest first.png> <oldest first.png>`
//! writes instead `data/student-sort-newest.png` and `data/student-sort-oldest.png`
//! alone: the student list's sort key 受け入れ順 beside each direction button
//! (see `aoi::student::list::SORT`), from screenshots of the list sorted so.

use std::path::Path;

use aoi::{grid, screen};

/// The student list's sort buttons, from a screenshot of each direction
/// (see the top).
fn student_list(data: &Path, mut screenshots: impl Iterator<Item = String>) {
    for name in ["student-sort-newest.png", "student-sort-oldest.png"] {
        let path = screenshots
            .next()
            .expect("the student list, sorted each way");
        let image = image::open(&path).expect("the image opens").to_rgb8();
        let unit =
            (screen::unit(image.width(), image.height()) * screen::STUDENT_UNIT).round() as u32;
        let picture = screen::crop(
            &image,
            unit,
            aoi::student::list::SORT,
            (0.0, 0.0),
            screen::HEIGHT,
        )
        .expect("sort buttons");
        picture.save(data.join(name)).expect("the image saves");
        println!("{name}: {}x{}", picture.width(), picture.height());
    }
}

fn main() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "student-list") {
        return student_list(&data, args.skip(1));
    }
    let (items, equipment) = (
        args.next().expect("item list"),
        args.next().expect("equipment list"),
    );
    let student = args.next().expect("student screen");
    let other_tab = args.next().expect("student screen on another tab");
    let open = |path: &str| {
        let image = image::open(path).expect("the image opens").to_rgb8();
        let unit = grid::find_panel(&image).expect("a list").card_height;
        (image, unit)
    };
    let (image, unit) = open(&items);
    let save = |name: &str, picture: image::RgbImage| {
        picture.save(data.join(name)).expect("the image saves");
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
    let image = image::open(&student).expect("the image opens").to_rgb8();
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
    let image = image::open(&other_tab).expect("the image opens").to_rgb8();
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
