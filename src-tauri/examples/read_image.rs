//! Prints what the recognizer reads from one screenshot, cell by cell, as the
//! list the screen shows (or as `items` / `equipment` when given).
//! Run with `cargo run --release --example read_image -- <screenshot.png> [items|equipment]`.

use std::path::Path;

use aoi::List;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("screenshot path");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let recognizer = aoi::Recognizer::load(&root.join("data")).unwrap();
    let image = image::open(&path).unwrap().to_rgb8();
    let (list, cells) = match args.next().as_deref() {
        Some("items") => (List::Items, recognizer.read(&image, List::Items)),
        Some("equipment") => (List::Equipment, recognizer.read(&image, List::Equipment)),
        Some(other) => panic!("unknown list {other}"),
        None => recognizer
            .read_list(&image)
            .map(|(list, cells, _)| (list, cells))
            .expect("no list in its default order"),
    };
    println!("{list:?}");
    let entries = recognizer.catalog(list).entries();
    for (i, cell) in cells.iter().enumerate() {
        let name = cell
            .id
            .and_then(|id| entries.iter().find(|e| e.id == id))
            .map_or("?", |e| e.name.as_str());
        let ignored = if cell.ignored { " (ignored)" } else { "" };
        println!(
            "{i:2} {:>8} x{:<6} {name}{ignored}",
            format!("{:?}", cell.id),
            cell.quantity.map_or("?".to_owned(), |q| q.to_string())
        );
    }
}
