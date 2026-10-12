//! Reads the short texts of the student screen ("Lv.65", "T10", "34") one
//! character at a time: each character is a blob of one ink, scaled to a common
//! height and compared with references taken from labeled screenshots.

use std::collections::BTreeMap;

use image::RgbImage;
use serde::{Deserialize, Serialize};

/// Whether a pixel is the ink of a text.
pub type Ink = fn([u8; 3]) -> bool;

/// How the characters of a text are cut out.
#[derive(Clone, Copy)]
pub struct Style {
    pub ink: Ink,
    /// Characters shorter than this share of the tallest one (the "." of
    /// "Lv.", in some fonts the "v") are left out.
    pub tall: f32,
    /// The text sits in a thin ring of its ink (the tier badge), worn off first.
    pub framed: bool,
    /// Blobs touching the top of the area are a card's edge, not characters.
    pub clear_top: bool,
    /// No character is wider than this share of its height; wider blobs are
    /// the edge of a card or a piece of a picture.
    pub widest: f32,
    /// A character's usual width as a share of its height. The italic fonts
    /// draw neighbours touching ("Lv", "90"); such a blob is cut into as many
    /// equal parts as characters of this width fit in it.
    pub width: f32,
}

/// Normalized character size; characters keep their width relative to their height.
pub const GLYPH: (usize, usize) = (12, 16);
/// Farthest a character may be from its nearest reference (squared difference
/// summed over the normalized character); the nearest reference names it.
/// Measured on 2026-10-12 over the evaluation screens: characters are within
/// 4.4 of their own references (95%, at most 15.8), and no nearer than 15.0 to
/// another character's (a tier's "8" to "3").
const MAX_DISTANCE: f32 = 17.0;

/// A rectangle of the screen in pixels: left, top, right, bottom.
pub type Rect = (u32, u32, u32, u32);

struct Blob {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

/// The ink's blobs (8-connected) in `rect`, left to right. With `thin_away`,
/// lines a pixel or two thin (the ring of the tier badge, which touches its
/// characters) are worn off first.
fn blobs(image: &RgbImage, rect: Rect, ink: Ink, thin_away: bool) -> (Vec<Blob>, Vec<bool>) {
    let (x0, y0, x1, y1) = rect;
    let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut on: Vec<bool> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .map(|(x, y)| ink(image.get_pixel(x, y).0))
        .collect();
    if thin_away {
        let at = |x: usize, y: usize| on[y * width + x];
        on = (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                x > 0
                    && y > 0
                    && x + 1 < width
                    && y + 1 < height
                    && at(x, y)
                    && at(x - 1, y)
                    && at(x + 1, y)
                    && at(x, y - 1)
                    && at(x, y + 1)
            })
            .collect();
    }
    let mut seen = vec![false; on.len()];
    let mut found = Vec::new();
    for start in 0..on.len() {
        if !on[start] || seen[start] {
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
            let (x, y) = ((i % width) as u32, (i / width) as u32);
            blob.left = blob.left.min(x);
            blob.right = blob.right.max(x);
            blob.top = blob.top.min(y);
            blob.bottom = blob.bottom.max(y);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                    if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                        continue;
                    }
                    let j = ny as usize * width + nx as usize;
                    if on[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        found.push(blob);
    }
    found.sort_by_key(|b| b.left);
    (found, on)
}

/// The characters of `style` in `rect`, left to right, normalized.
pub fn shapes(image: &RgbImage, rect: Rect, style: Style) -> Vec<Vec<f32>> {
    let (ink, framed) = (style.ink, style.framed);
    let width = (rect.2 - rect.0) as usize;
    let (mut blobs, on) = blobs(image, rect, ink, framed);
    if style.clear_top {
        blobs.retain(|b| b.top > 0);
    }
    if framed {
        // Only what lies within the ring (the widest blob before wearing off)
        // belongs to the badge; a picture beside it does not.
        let (unworn, _) = self::blobs(image, rect, ink, false);
        let Some(ring) = unworn.iter().max_by_key(|b| b.right - b.left) else {
            return Vec::new();
        };
        blobs.retain(|b| b.left >= ring.left && b.right <= ring.right);
    }
    blobs.retain(|b| (b.right - b.left + 1) as f32 <= (b.bottom - b.top + 1) as f32 * style.widest);
    let tallest = blobs
        .iter()
        .map(|b| b.bottom - b.top + 1)
        .max()
        .unwrap_or(0);
    blobs.retain(|b| (b.bottom - b.top + 1) as f32 >= tallest as f32 * style.tall);
    let blobs: Vec<Blob> = blobs
        .into_iter()
        .flat_map(|b| {
            let (w, h) = ((b.right - b.left + 1) as f32, (b.bottom - b.top + 1) as f32);
            let parts = (w / (h * style.width)).round().max(1.0) as u32;
            let step = w / parts as f32;
            (0..parts).map(move |i| Blob {
                left: b.left + (i as f32 * step) as u32,
                right: b.left + ((i + 1) as f32 * step) as u32 - 1,
                top: b.top,
                bottom: b.bottom,
            })
        })
        .collect();
    blobs
        .iter()
        .map(|b| {
            let (gw, gh) = GLYPH;
            let scale = (b.bottom - b.top + 1) as f32 / gh as f32;
            let mut out = vec![0.0; gw * gh];
            for gy in 0..gh {
                for gx in 0..gw {
                    let x = b.left as f32 + (gx as f32 + 0.5) * scale;
                    let y = b.top as f32 + (gy as f32 + 0.5) * scale;
                    if x <= b.right as f32 + 0.5 {
                        out[gy * gw + gx] =
                            f32::from(u8::from(on[y as usize * width + x as usize]));
                    }
                }
            }
            out
        })
        .collect()
}

/// How far apart two normalized characters are.
pub fn distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum()
}

/// Reference characters of each text style; a character drawn in more than
/// one way (a "0" cut from a "70" keeps a piece of the "7") has a reference
/// for each way.
#[derive(Serialize, Deserialize, Default)]
pub struct Fonts(pub BTreeMap<String, BTreeMap<char, Vec<Vec<f32>>>>);

impl Fonts {
    /// The text the `shapes` spell in `style`, if every one is close to a reference.
    pub fn read(&self, style: &str, shapes: &[Vec<f32>]) -> Option<String> {
        let font = self.0.get(style)?;
        shapes
            .iter()
            .map(|shape| {
                font.iter()
                    .flat_map(|(c, refs)| refs.iter().map(move |r| (*c, distance(r, shape))))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .filter(|(_, d)| *d <= MAX_DISTANCE)
                    .map(|(c, _)| c)
            })
            .collect()
    }
}
