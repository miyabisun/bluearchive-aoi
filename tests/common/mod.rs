//! What the recognition tests share.

use image::{
    RgbImage,
    imageops::{FilterType, resize},
};

/// `image` as a screen drawing the game `width` wide shows it (see
/// `aoi::screen::drawn_width`), a window's, a tablet's or a stream's alike,
/// and as Aoi reads it (`aoi::screen::normalize`).
pub fn drawn_at(image: &RgbImage, width: u32) -> RgbImage {
    let scale = width as f32 / aoi::screen::drawn_width(image);
    let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
    let screen = resize(
        image,
        w.round() as u32,
        h.round() as u32,
        FilterType::Triangle,
    );
    aoi::screen::normalize(&screen).into_owned()
}
