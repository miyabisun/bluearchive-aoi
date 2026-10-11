//! Finds the fully visible cards of the item list.
//!
//! The list draws its cards on a flat background colour. Rows of cards are
//! separated by horizontal gaps, and columns by gaps that lean with the cards'
//! shear. Sizes are measured from each frame, so any window size works.

use image::RgbImage;

const BACKGROUND: [u8; 3] = [196, 207, 212];
const BACKGROUND_TOLERANCE: u8 = 6;
/// Horizontal shift of card edges per pixel downwards (they lean right going up).
pub const SHEAR: f32 = 0.1875;
/// A row is a gap when this share of the panel width is background.
const GAP_ROW_SHARE: f32 = 0.85;
/// An edge row shorter than the card height by more than this is cut off.
const CUT_OFF_SLACK: u32 = 3;
/// Spans between gaps shorter than this are no row of cards.
const MIN_ROW: u32 = 20;

/// A card, measured at its vertical centre where the shear has no effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub left: f32,
    pub width: f32,
    pub top: u32,
    pub height: u32,
}

impl Card {
    pub fn center_x(&self) -> f32 {
        self.left + self.width / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.top as f32 + self.height as f32 / 2.0
    }

    /// Horizontal offset of the card's edges at row `y` relative to its centre row.
    pub fn shift_at(&self, y: f32) -> f32 {
        -SHEAR * (y - self.center_y())
    }
}

fn is_background(image: &RgbImage, x: u32, y: u32) -> bool {
    let p = image.get_pixel(x, y).0;
    p.iter()
        .zip(BACKGROUND)
        .all(|(&a, b)| a.abs_diff(b) <= BACKGROUND_TOLERANCE)
}

/// Longest horizontal run of background pixels in row `y`, as `(start, len)`.
fn longest_run(image: &RgbImage, y: u32) -> (u32, u32) {
    let (mut best, mut start, mut len) = ((0, 0), 0, 0);
    for x in 0..image.width() {
        if is_background(image, x, y) {
            if len == 0 {
                start = x;
            }
            len += 1;
            if len > best.1 {
                best = (start, len);
            }
        } else {
            len = 0;
        }
    }
    best
}

/// Spans `[start, end)` of consecutive `true` values.
fn spans(flags: &[bool]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &f) in flags.iter().chain(std::iter::once(&false)).enumerate() {
        let i = u32::try_from(i).unwrap();
        match (f, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// Fully visible cards in reading order (rows top to bottom, left to right).
pub fn find_cards(image: &RgbImage) -> Vec<Card> {
    let runs: Vec<(u32, u32)> = (0..image.height()).map(|y| longest_run(image, y)).collect();
    let Some(&(panel_left, panel_width)) = runs.iter().max_by_key(|r| r.1) else {
        return Vec::new();
    };
    if panel_width < image.width() / 4 {
        return Vec::new();
    }
    let panel_right = panel_left + panel_width;

    let gap_rows: Vec<bool> = (0..image.height())
        .map(|y| {
            let bg = (panel_left..panel_right)
                .filter(|&x| is_background(image, x, y))
                .count();
            bg as f32 >= GAP_ROW_SHARE * panel_width as f32
        })
        .collect();
    let gaps = spans(&gap_rows);
    // Card rows lie between two gaps; the viewport edges count as gaps too. A badge
    // spilling into a gap splits it, leaving a sliver that is no row.
    let rows: Vec<(u32, u32)> = gaps
        .windows(2)
        .map(|w| (w[0].1, w[1].0))
        .filter(|(top, bottom)| bottom - top >= MIN_ROW)
        .collect();
    let [first, .., last] = rows.as_slice() else {
        return Vec::new();
    };
    // Only the rows at the viewport's edges can be cut off. Inner rows are whole, but
    // a badge can make one look a little taller or shorter, so their middle height is
    // the card height.
    let mut inner: Vec<u32> = rows[1..rows.len() - 1].iter().map(|(a, b)| b - a).collect();
    inner.sort_unstable();
    let height = inner
        .get(inner.len() / 2)
        .copied()
        .unwrap_or_else(|| (last.1 - last.0).max(first.1 - first.0));

    let mut cards = Vec::new();
    for (i, &(top, bottom)) in rows.iter().enumerate() {
        let edge = i == 0 || i == rows.len() - 1;
        if edge && bottom - top + CUT_OFF_SLACK < height {
            continue;
        }
        let top = (top + bottom).saturating_sub(height) / 2;
        cards.extend(cards_in_row(image, top, height, panel_left, panel_right));
    }
    cards
}

fn cards_in_row(
    image: &RgbImage,
    top: u32,
    height: u32,
    panel_left: u32,
    panel_right: u32,
) -> Vec<Card> {
    let mid = top as f32 + height as f32 / 2.0;
    // The middle of the row avoids the rounded corners.
    let ys: Vec<u32> = (top + height / 5..top + height * 4 / 5).collect();
    let gap_columns: Vec<bool> = (panel_left..panel_right)
        .map(|x| {
            let bg = ys
                .iter()
                .filter(|&&y| {
                    let sx = x as f32 - SHEAR * (y as f32 - mid);
                    sx >= 0.0 && (sx as u32) < image.width() && is_background(image, sx as u32, y)
                })
                .count();
            bg * 10 >= ys.len() * 9
        })
        .collect();
    let card_columns: Vec<bool> = gap_columns.iter().map(|g| !g).collect();
    spans(&card_columns)
        .into_iter()
        .filter(|(a, b)| b - a >= height / 2)
        .map(|(a, b)| Card {
            left: (panel_left + a) as f32,
            width: (b - a) as f32,
            top,
            height,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> RgbImage {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        image::open(path).unwrap().to_rgb8()
    }

    fn check(name: &str, count: usize, height: u32, pitch: f32) {
        let cards = find_cards(&fixture(name));
        assert_eq!(cards.len(), count, "{name}: {cards:?}");
        for c in &cards {
            assert!(c.height.abs_diff(height) <= 2, "{name}: {c:?}");
        }
        for pair in cards.chunks(5) {
            for w in pair.windows(2) {
                let step = w[1].center_x() - w[0].center_x();
                assert!((step - pitch).abs() <= 3.0, "{name}: column pitch {step}");
            }
        }
    }

    #[test]
    fn finds_full_rows_in_16_by_9() {
        check("items-16x9-secretstone.png", 15, 164, 184.0);
    }

    #[test]
    fn finds_full_rows_in_4_by_3() {
        check("items-4x3-ooparts.png", 30, 133, 148.5);
    }
}
