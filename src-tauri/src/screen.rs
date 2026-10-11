//! Whether the item list is sorted the default way, ascending ("デフォルト ≡↑").
//!
//! The list's order tells shared icons apart, so items are only read while the
//! sort buttons show the default order. The buttons sit at a fixed distance from
//! the window's top right corner, measured in card heights, at any window size.
//! They are compared with `data/sort-default.png`, written by
//! `cargo run --example make_sort_template`.

use image::{RgbImage, imageops::FilterType};

/// The sort buttons' area: from `.0` to `.1` card heights left of the right edge,
/// and from `.2` to `.3` card heights below the top.
const AREA: (f32, f32, f32, f32) = (3.2, 0.4, 0.95, 1.42);
/// The buttons may sit this far off (card heights), across and down.
const SLACK: (f32, f32) = (0.15, 0.06);
/// Height of the compared picture, in pixels.
pub const HEIGHT: u32 = 24;
/// Least similarity of the buttons to the default order's.
const MATCH: f32 = 0.85;
/// The direction button ("≡↑") starts at this share of the template's width.
const DIRECTION_FROM: f32 = 0.77;
/// Card heights off by these factors are tried too.
const SIZES: [f32; 5] = [0.94, 0.97, 1.0, 1.03, 1.06];

/// The sort buttons' area, resampled to `HEIGHT` pixels tall, with `slack` card
/// heights around it.
pub fn sort_buttons(image: &RgbImage, card_height: u32, slack: (f32, f32)) -> Option<RgbImage> {
    let unit = card_height as f32;
    let scale = HEIGHT as f32 / ((AREA.3 - AREA.2) * unit);
    let width = image.width() as f32;
    let x0 = width - (AREA.0 + slack.0) * unit;
    let x1 = width - (AREA.1 - slack.0) * unit;
    let y0 = (AREA.2 - slack.1) * unit;
    let y1 = (AREA.3 + slack.1) * unit;
    if x0 < 0.0 || x1 > width || y1 > image.height() as f32 {
        return None;
    }
    let crop = image::imageops::crop_imm(
        image,
        x0 as u32,
        y0 as u32,
        (x1 - x0) as u32,
        (y1 - y0) as u32,
    )
    .to_image();
    Some(image::imageops::resize(
        &crop,
        ((x1 - x0) * scale).round() as u32,
        ((y1 - y0) * scale).round() as u32,
        FilterType::Triangle,
    ))
}

/// Normalized cross-correlation of the template's columns `columns` with the
/// image placed at `(ox, oy)`.
fn ncc(
    template: &RgbImage,
    columns: std::ops::Range<u32>,
    image: &RgbImage,
    ox: u32,
    oy: u32,
) -> f32 {
    let (mut n, mut st, mut si, mut stt, mut sii, mut sti) = (0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0);
    for y in 0..template.height() {
        for x in columns.clone() {
            let (wanted, seen) = (template.get_pixel(x, y), image.get_pixel(ox + x, oy + y));
            for c in 0..3 {
                let (a, b) = (f32::from(wanted.0[c]), f32::from(seen.0[c]));
                n += 1.0;
                st += a;
                si += b;
                stt += a * a;
                sii += b * b;
                sti += a * b;
            }
        }
    }
    let var_t = stt - st * st / n;
    let var_i = sii - si * si / n;
    if var_t <= 0.0 || var_i <= 0.0 {
        return -1.0;
    }
    (sti - st * si / n) / (var_t * var_i).sqrt()
}

/// Whether the frame's sort buttons show the default order, ascending. The
/// order's name and the direction button must both match: the name alone
/// would pass a descending list. Window sizes and game versions shift the
/// buttons a little, so a few sizes and positions are tried.
pub fn sorted_by_default(image: &RgbImage, card_height: u32, template: &RgbImage) -> bool {
    let split = (template.width() as f32 * DIRECTION_FROM) as u32;
    let mut best = f32::MIN;
    for size in SIZES {
        let unit = (card_height as f32 * size).round() as u32;
        let Some(area) = sort_buttons(image, unit, SLACK) else {
            continue;
        };
        if area.width() < template.width() || area.height() < template.height() {
            continue;
        }
        for oy in 0..=area.height() - template.height() {
            for ox in 0..=area.width() - template.width() {
                let name = ncc(template, 0..split, &area, ox, oy);
                let direction = ncc(template, split..template.width(), &area, ox, oy);
                best = best.max(name.min(direction));
            }
        }
    }
    best >= MATCH
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use image::RgbImage;

    use super::sorted_by_default;
    use crate::grid::find_panel;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn template() -> RgbImage {
        image::open(root().join("data/sort-default.png"))
            .unwrap()
            .to_rgb8()
    }

    fn shows_default(image: &RgbImage) -> bool {
        find_panel(image).is_some_and(|p| sorted_by_default(image, p.card_height, &template()))
    }

    fn fixture(name: &str) -> RgbImage {
        image::open(root().join("tests/fixtures").join(name))
            .unwrap()
            .to_rgb8()
    }

    #[test]
    fn every_item_list_fixture_is_in_the_default_order() {
        for name in [
            "items-16x9-secretstone.png",
            "items-16x9-list-top.png",
            "items-16x9-coins-materials.png",
            "items-16x9-gifts-tickets.png",
            "items-16x9-event-items.png",
            "items-4x3-ooparts.png",
        ] {
            assert!(shows_default(&fixture(name)), "{name}");
        }
    }

    #[test]
    fn other_screens_are_not_the_item_list() {
        for name in [
            "other/home.png",
            "other/momotalk.png",
            "other/event-stages.png",
        ] {
            assert!(!shows_default(&fixture(name)), "{name}");
        }
    }

    /// No descending screenshot yet: the "≡↑" button turned upside down stands in.
    #[test]
    fn a_descending_arrow_is_not_the_default_order() {
        let mut image = fixture("items-16x9-secretstone.png");
        let (x0, y0, x1, y1) = (2050, 158, 2170, 238);
        let button = image::imageops::crop_imm(&image, x0, y0, x1 - x0, y1 - y0).to_image();
        let flipped = image::imageops::flip_vertical(&button);
        image::imageops::replace(&mut image, &flipped, i64::from(x0), i64::from(y0));
        assert!(!shows_default(&image));
    }
}
