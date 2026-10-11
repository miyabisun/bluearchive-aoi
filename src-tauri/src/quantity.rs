//! Reads the "xN" count at the bottom right of a card.
//!
//! The count is drawn in one flat ink colour with a white outline, so each
//! character is a separate blob of ink. Blobs are scaled to a common height
//! and compared with reference digits taken from labeled screenshots
//! (`data/digits.json`, written by `cargo run --example build_digits`).

use image::RgbImage;
use serde::{Deserialize, Serialize};

use crate::grid::Card;

const INK: [f32; 3] = [45.0, 70.0, 99.0];
/// Least share of ink (versus white) for a pixel to belong to a character.
const INK_SHARE: f32 = 0.5;
/// Most distance from the ink-to-white line, which keeps out other colours.
const OFF_LINE: f32 = 30.0;
/// The count sits in the bottom part of the card (fraction of card height).
const COUNT_TOP: f32 = 0.7;
/// Digit height relative to the card height.
const DIGIT_HEIGHT: (f32, f32) = (0.11, 0.2);
/// The "x" is shorter than the digits it precedes.
const X_HEIGHT: (f32, f32) = (0.45, 0.85);
/// Normalized glyph size: characters keep their width relative to their height.
pub const GLYPH: (usize, usize) = (16, 20);

/// How much a pixel looks like ink, 0 (white) to 1 (ink), if it lies on the
/// ink-to-white blend at all.
fn ink_share(p: [u8; 3]) -> Option<f32> {
    let d: Vec<f32> = (0..3).map(|c| 255.0 - f32::from(p[c])).collect();
    let axis: Vec<f32> = INK.iter().map(|v| 255.0 - v).collect();
    let len_sq: f32 = axis.iter().map(|a| a * a).sum();
    let t = d.iter().zip(&axis).map(|(a, b)| a * b).sum::<f32>() / len_sq;
    let off: f32 = d
        .iter()
        .zip(&axis)
        .map(|(a, b)| (a - t * b).powi(2))
        .sum::<f32>()
        .sqrt();
    (off <= OFF_LINE).then_some(t.clamp(0.0, 1.0))
}

#[derive(Debug, Clone)]
struct Blob {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl Blob {
    fn height(&self) -> u32 {
        self.bottom - self.top + 1
    }
}

/// Ink blobs (8-connected) inside the rectangle `[x0, x1) x [y0, y1)`.
fn blobs(image: &RgbImage, x0: u32, x1: u32, y0: u32, y1: u32) -> Vec<Blob> {
    let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let ink: Vec<bool> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .map(|(x, y)| ink_share(image.get_pixel(x, y).0).is_some_and(|t| t >= INK_SHARE))
        .collect();
    let mut seen = vec![false; width * height];
    let mut out = Vec::new();
    for start in 0..width * height {
        if !ink[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start];
        let mut blob = Blob {
            left: u32::MAX,
            right: 0,
            top: u32::MAX,
            bottom: 0,
        };
        while let Some(i) = stack.pop() {
            let (x, y) = (i % width, i / width);
            let (px, py) = (x0 + x as u32, y0 + y as u32);
            blob.left = blob.left.min(px);
            blob.right = blob.right.max(px);
            blob.top = blob.top.min(py);
            blob.bottom = blob.bottom.max(py);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                        continue;
                    }
                    let j = ny as usize * width + nx as usize;
                    if ink[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        out.push(blob);
    }
    out
}

/// The digit blobs of the card's count, left to right, if a count is found.
fn digit_blobs(image: &RgbImage, card: &Card) -> Option<Vec<Blob>> {
    let h = card.height as f32;
    let y0 = card.top + (h * COUNT_TOP) as u32;
    let y1 = (card.top + card.height).min(image.height());
    // At the count's rows the sheared card sits left of its centre-row position.
    let shift = card.shift_at(y1 as f32);
    let x0 = (card.left + shift).max(0.0) as u32;
    let x1 = ((card.left + card.width + card.shift_at(y0 as f32)) as u32).min(image.width());
    let all = blobs(image, x0, x1, y0, y1);

    let is_digit = |b: &Blob| {
        let r = b.height() as f32 / h;
        r >= DIGIT_HEIGHT.0 && r <= DIGIT_HEIGHT.1
    };
    // The count is right-aligned: start from the rightmost digit-sized blob.
    let last = all.iter().filter(|b| is_digit(b)).max_by_key(|b| b.right)?;
    let line = |b: &Blob| b.bottom.abs_diff(last.bottom) <= 2;
    let mut digits: Vec<Blob> = vec![last.clone()];
    loop {
        let first = &digits[0];
        let gap_limit = first.height() as f32 * 0.6;
        let next = all
            .iter()
            .filter(|b| b.right < first.left + 2 && line(b))
            .filter(|b| (first.left as f32 - b.right as f32) <= gap_limit)
            .max_by_key(|b| b.right)?;
        if is_digit(next) && next.top.abs_diff(last.top) <= 2 {
            digits.insert(0, next.clone());
            continue;
        }
        // The blob before the digits must be the "x".
        let r = next.height() as f32 / last.height() as f32;
        return (r >= X_HEIGHT.0 && r <= X_HEIGHT.1).then_some(digits);
    }
}

/// A blob scaled to `GLYPH` height, keeping its width proportion, left-aligned.
fn normalize(image: &RgbImage, blob: &Blob) -> Vec<f32> {
    let (gw, gh) = GLYPH;
    let scale = (blob.height() as f32) / gh as f32;
    let mut out = vec![0.0; gw * gh];
    for gy in 0..gh {
        for gx in 0..gw {
            let x = blob.left as f32 + (gx as f32 + 0.5) * scale;
            let y = blob.top as f32 + (gy as f32 + 0.5) * scale;
            if x > blob.right as f32 + 0.5 {
                continue;
            }
            let (x, y) = (x as u32, y as u32);
            if x < image.width() && y < image.height() {
                out[gy * gw + gx] = ink_share(image.get_pixel(x, y).0).unwrap_or(0.0);
            }
        }
    }
    out
}

/// Normalized digit glyphs of the card's count, left to right.
pub fn glyphs(image: &RgbImage, card: &Card) -> Option<Vec<Vec<f32>>> {
    Some(
        digit_blobs(image, card)?
            .iter()
            .map(|b| normalize(image, b))
            .collect(),
    )
}

/// Reference glyph for each digit.
#[derive(Serialize, Deserialize)]
pub struct Digits {
    pub glyphs: Vec<Vec<f32>>,
}

impl Digits {
    fn closest(&self, glyph: &[f32]) -> u32 {
        let dist = |r: &Vec<f32>| {
            r.iter()
                .zip(glyph)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
        };
        (0u32..)
            .zip(&self.glyphs)
            .min_by(|a, b| dist(a.1).total_cmp(&dist(b.1)))
            .map(|(d, _)| d)
            .unwrap()
    }

    pub fn read(&self, image: &RgbImage, card: &Card) -> Option<u32> {
        glyphs(image, card)?
            .iter()
            .try_fold(0u32, |n, g| n.checked_mul(10)?.checked_add(self.closest(g)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::find_cards;
    use std::path::Path;

    #[derive(Deserialize)]
    struct Fixture {
        image: String,
        cells: Vec<Label>,
    }

    #[derive(Deserialize)]
    struct Label {
        quantity: Option<u32>,
    }

    #[test]
    fn finds_one_glyph_per_digit_of_each_count() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let fixtures: Vec<Fixture> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap())
                .unwrap();
        let mut wrong = Vec::new();
        for fixture in fixtures {
            let image = image::open(dir.join(&fixture.image)).unwrap().to_rgb8();
            for (i, (card, label)) in find_cards(&image).iter().zip(&fixture.cells).enumerate() {
                let want = label.quantity.map(|q| q.to_string().len());
                let got = glyphs(&image, card).map(|g| g.len());
                if got != want {
                    wrong.push(format!(
                        "{} cell {i}: {got:?} glyphs for {:?}",
                        fixture.image, label.quantity
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }
}
