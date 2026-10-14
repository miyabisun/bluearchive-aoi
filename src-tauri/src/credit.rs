//! The credits the top bar shows ("243,797,319"), read digit by digit on the
//! screens that have the bar.

use image::RgbImage;

use crate::{
    glyph::{self, Fonts, Style},
    student::{Area, rect},
};

/// The dark digits on a light ground: the credits, and the coins held over a
/// shop's cards. The commas are too short to count.
pub const DARK_DIGITS: Style = Style {
    ink: |c| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2]) < 140.0,
    tall: 0.6,
    framed: false,
    clear_top: false,
    widest: 1.3,
    width: 1.2,
    skew: 0.0,
};

/// The credits, a number up to the bar's next field (window heights from the
/// right edge and the top; measured on 2026-10-12 at 2222x1250: the digits of
/// "243,797,319" span 0.777 to 0.621 from the right, 0.020 to 0.044 down).
pub const NUMBER: Area = Area {
    from_right: true,
    x: (0.779, 0.556),
    y: (0.012, 0.050),
};

/// The yellow card standing for credits, left of the number.
pub const ICON: Area = Area {
    from_right: true,
    x: (0.834, 0.797),
    y: (0.020, 0.047),
};

/// The card's yellow covers at least this share of its area.
const ICON_SHARE: f32 = 0.15;

/// Whether the bar shows the credits: their yellow card is there.
pub fn bar_shows(image: &RgbImage) -> bool {
    let Some((x0, y0, x1, y1)) = rect(image, ICON) else {
        return false;
    };
    let yellow = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let c = image.get_pixel(x, y).0;
            c[0] > 230 && c[1] > 180 && c[2] < 100
        })
        .count();
    yellow as f32 >= ICON_SHARE * ((x1 - x0) * (y1 - y0)) as f32
}

/// The digits of the credits, when the bar shows them.
pub fn shapes(image: &RgbImage) -> Option<Vec<Vec<f32>>> {
    if !bar_shows(image) {
        return None;
    }
    Some(glyph::shapes(image, rect(image, NUMBER)?, DARK_DIGITS))
}

/// The credits the bar shows, when its every digit reads.
pub fn read(image: &RgbImage, fonts: &Fonts) -> Option<u64> {
    let digits = fonts.read("digits", &shapes(image)?)?;
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}
