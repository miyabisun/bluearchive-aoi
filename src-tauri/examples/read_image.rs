//! Prints what the recognizer reads from one screenshot, cell by cell.
//! Run with `cargo run --release --example read_image -- <screenshot.png>`.

use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("screenshot path");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let recognizer = aoi::Recognizer::load(&root.join("data")).unwrap();
    let image = image::open(&path).unwrap().to_rgb8();
    for (i, cell) in recognizer.read(&image).iter().enumerate() {
        let name = cell
            .id
            .and_then(|id| recognizer.items().iter().find(|item| item.id == id))
            .map_or("?", |item| item.name.as_str());
        println!(
            "{i:2} {:>8} x{:<6} {name}",
            format!("{:?}", cell.id),
            format!("{:?}", cell.quantity)
        );
    }
}
