//! Times `Recognizer::read_list` (the whole read of a frame) on each fixture screenshot.
//! Run with `cargo run --release --example time_read`.

use std::path::Path;
use std::time::Instant;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let started = Instant::now();
    let recognizer = aoi::Recognizer::load(&root.join("data")).expect("the recognizer loads data/");
    println!("load: {:?}", started.elapsed());
    for entry in std::fs::read_dir(root.join("tests/fixtures")).expect("the folder lists") {
        let path = entry.expect("the folder entry reads").path();
        if path.extension().is_none_or(|e| e != "png") {
            continue;
        }
        let image = image::open(&path).expect("the image opens").to_rgb8();
        let runs = 20;
        let started = Instant::now();
        for _ in 0..runs {
            std::hint::black_box(recognizer.read_list(&image));
        }
        let whole = started.elapsed() / runs;
        let started = Instant::now();
        for _ in 0..runs {
            std::hint::black_box(aoi::grid::find_cards(&image));
        }
        let grid = started.elapsed() / runs;
        let name = path
            .file_name()
            .expect("a file in a folder has a name")
            .to_string_lossy();
        println!("{name}: {whole:?} per frame, of which finding cards {grid:?}");
    }
}
