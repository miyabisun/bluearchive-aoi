//! Writes `data/sort-default.png`, the sort buttons in the default order
//! ("デフォルト ≡↑"), from a screenshot of the item list sorted that way.
//! Run with `cargo run --example make_sort_template -- <screenshot.png>`.

use std::path::Path;

use aoi::{grid, screen};

fn main() {
    let path = std::env::args().nth(1).expect("screenshot path");
    let image = image::open(path).unwrap().to_rgb8();
    let panel = grid::find_panel(&image).expect("an item list");
    let buttons =
        screen::sort_buttons(&image, panel.card_height, (0.0, 0.0)).expect("sort buttons");
    buttons
        .save(Path::new(env!("CARGO_MANIFEST_DIR")).join("data/sort-default.png"))
        .unwrap();
    println!("{}x{}", buttons.width(), buttons.height());
}
