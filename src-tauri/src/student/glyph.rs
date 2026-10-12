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
    /// A character's usual width as a share of the tallest one's height. The italic fonts
    /// draw neighbours touching ("Lv", "90"); such a blob is cut into as many
    /// equal parts as characters of this width fit in it.
    pub width: f32,
    /// How far the font leans: each row is moved right by this share of its
    /// height from the top, which stands the italic characters upright and
    /// parts neighbours that touch only along their slant.
    pub skew: f32,
}

/// Normalized character size; characters keep their width relative to their height.
pub const GLYPH: (usize, usize) = (12, 16);
/// Farthest a character may be from its nearest reference (squared difference
/// summed over the normalized character); the nearest reference names it.
/// Measured on 2026-10-12 over the evaluation screens: characters are within
/// 4.4 of their own references (95%, at most 15.8), and no nearer than 15.0 to
/// another character's (a tier's "8" to "3").
const MAX_DISTANCE: f32 = 17.0;
/// The nearest character must be at most this share of the distance to the
/// next nearest one, or the character is unknown. Measured on 2026-10-12: a
/// character read right has the share at most 0.52 (0.30 on the screens the
/// references come from); a bond's "3" few references were taken for came at
/// 1.19, nearer to "8".
const LEAD: f32 = 0.6;

/// A rectangle of the screen in pixels: left, top, right, bottom.
pub type Rect = (u32, u32, u32, u32);

struct Blob {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

/// Which pixels of `rect` are of the ink, each row moved right by `skew`
/// times its height from the top; the mask and its width and height.
fn mask(image: &RgbImage, rect: Rect, ink: Ink, skew: f32) -> (Vec<bool>, usize, usize) {
    let (x0, y0, x1, y1) = rect;
    let (w, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let width = w + (skew * height as f32).ceil() as usize;
    let mut on = vec![false; width * height];
    for y in 0..height {
        let shift = (skew * y as f32).round() as usize;
        for x in 0..w {
            on[y * width + x + shift] = ink(image.get_pixel(x0 + x as u32, y0 + y as u32).0);
        }
    }
    (on, width, height)
}

/// The blobs (8-connected) of the mask `on`, left to right, and the mask they
/// were found in. With `thin_away`, lines a pixel or two thin (the ring of the
/// tier badge, which touches its characters) are worn off first.
fn blobs(
    mut on: Vec<bool>,
    width: usize,
    height: usize,
    thin_away: bool,
) -> (Vec<Blob>, Vec<bool>) {
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

/// The pixels off the ink that the edge of the area reaches (4-connected)
/// without crossing the ink. The ink is widened by a pixel first, closing the
/// gaps a pale ring has.
fn outside(ink: &[bool], width: usize, height: usize) -> Vec<bool> {
    let on: Vec<bool> = (0..ink.len())
        .map(|i| {
            let (x, y) = (i % width, i / width);
            (y.saturating_sub(1)..=(y + 1).min(height - 1)).any(|ny| {
                (x.saturating_sub(1)..=(x + 1).min(width - 1)).any(|nx| ink[ny * width + nx])
            })
        })
        .collect();
    let edge = |i: usize| {
        let (x, y) = (i % width, i / width);
        x == 0 || y == 0 || x + 1 == width || y + 1 == height
    };
    let mut reached: Vec<bool> = (0..on.len()).map(|i| edge(i) && !on[i]).collect();
    let mut stack: Vec<usize> = (0..on.len()).filter(|&i| reached[i]).collect();
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        let near = [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ];
        for j in near.into_iter().flatten() {
            if !on[j] && !reached[j] {
                reached[j] = true;
                stack.push(j);
            }
        }
    }
    reached
}

/// The characters of `style` in `rect`, left to right, normalized.
pub fn shapes(image: &RgbImage, rect: Rect, style: Style) -> Vec<Vec<f32>> {
    let framed = style.framed;
    let (raw, width, height) = mask(image, rect, style.ink, style.skew);
    let (mut blobs, on) = blobs(raw.clone(), width, height, framed);
    if style.clear_top {
        blobs.retain(|b| b.top > 0);
    }
    if framed {
        // Only what the ring encloses belongs to the badge: a picture beside
        // it, even touching the ring, lies where the outside reaches. The
        // outside is looked for two pixels around each character (the gap to
        // the ring is wider).
        let outside = outside(&raw, width, height);
        let reached = |x: u32, y: u32| outside[y as usize * width + x as usize];
        blobs.retain(|b| {
            let (left, top) = (b.left.saturating_sub(2), b.top.saturating_sub(2));
            let right = (b.right + 2).min(width as u32 - 1);
            let bottom = (b.bottom + 2).min(height as u32 - 1);
            (left..=right).all(|x| !reached(x, top) && !reached(x, bottom))
                && (top..=bottom).all(|y| !reached(left, y) && !reached(right, y))
        });
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
            // Measured against the tallest character: a short one ("v")
            // standing alone is no wider for it.
            let w = (b.right - b.left + 1) as f32;
            let parts = (w / (tallest as f32 * style.width)).round().max(1.0) as u32;
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

/// How far apart two normalized characters are, the second moved up to a
/// pixel either way to meet the first: a character cut a pixel off from its
/// references (a thin "3" one column right) still meets them.
pub fn distance(a: &[f32], b: &[f32]) -> f32 {
    let (w, h) = (GLYPH.0 as i32, GLYPH.1 as i32);
    let at = |x: i32, y: i32| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            b[(y * w + x) as usize]
        } else {
            0.0
        }
    };
    (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| (a[(y * w + x) as usize] - at(x + dx, y + dy)).powi(2))
                .sum::<f32>()
        })
        .fold(f32::INFINITY, f32::min)
}

/// Reference characters of each text style; a character drawn in more than
/// one way (a "0" cut from a "70" keeps a piece of the "7") has a reference
/// for each way.
#[derive(Serialize, Deserialize, Default)]
pub struct Fonts(pub BTreeMap<String, BTreeMap<char, Vec<Vec<f32>>>>);

impl Fonts {
    /// The text the `shapes` spell in `style`, if every one is close to a
    /// reference and clearly nearer to it than to any other character.
    pub fn read(&self, style: &str, shapes: &[Vec<f32>]) -> Option<String> {
        let font = self.0.get(style)?;
        shapes
            .iter()
            .map(|shape| {
                let mut nearest: Vec<(char, f32)> = font
                    .iter()
                    .map(|(c, refs)| {
                        let d = refs.iter().map(|r| distance(r, shape));
                        (*c, d.fold(f32::INFINITY, f32::min))
                    })
                    .collect();
                nearest.sort_by(|a, b| a.1.total_cmp(&b.1));
                let (c, d) = *nearest.first()?;
                let next = nearest.get(1).map_or(f32::INFINITY, |n| n.1);
                (d <= MAX_DISTANCE && d <= LEAD * next).then_some(c)
            })
            .collect()
    }
}
