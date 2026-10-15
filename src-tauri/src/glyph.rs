//! Reads the short texts of the screens ("Lv.65", "T10", "243,797,319") one
//! character at a time: each character is a blob of one ink, scaled to a common
//! height and compared with references taken from labeled screenshots (kept in
//! the glyph bank, `bank`). Longer texts drawn from a known list, the
//! students' and the shops' names, are told whole (`word`).

pub mod bank;
pub mod word;

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
    /// What of the area's edge is the ground around the text.
    pub ground: Ground,
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
    /// A piece left out from among the characters (shorter than `tall`) leaves
    /// the text unread: a character of a smaller screen's broken in pieces
    /// ("Lv70" losing its "7") would read short. A dot on the line (`DOT`, the
    /// one of "Lv."), a speck (`SPECK`) or a hole within a character is no
    /// piece; a comma ("243,819") is, so not for such texts.
    pub entire: bool,
    /// The badge the text is drawn on, told by its colour (a video halves the
    /// colour's resolution, which a thin stroke loses and a badge keeps): the
    /// text is read within the extent of the badge's largest part alone.
    pub within: Option<Ink>,
}

/// Where blobs touching the edge of a text's area are the ground around the
/// text, no characters.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    /// Nowhere: characters may touch the edge.
    Free,
    /// At the top: a card's edge.
    Top,
    /// All around: the text is a fill its outline encloses, however bright
    /// the ground (on a video the fill dims to the ground's brightness, and
    /// only the outline keeps them apart).
    Around,
}

/// A dot is at most this share of the tallest character high (the one of
/// "Lv." 3 of 19 pixels, at 2222 wide).
const DOT: f32 = 0.3;
/// A speck is at most this share of the tallest character wide and high (a
/// pixel or two of light among the outlines; the pieces of a "7" broken on a
/// smaller screen measure 4 by 6 of 13).
const SPECK: f32 = 0.15;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Blob {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

/// Whether a piece of the `short` ones lies among the characters `kept` (see
/// `Style::entire`).
fn piece_left_out(kept: &[Blob], short: &[Blob], tallest: u32) -> bool {
    let first = kept.iter().map(|b| b.left).min().unwrap_or(0);
    let last = kept.iter().map(|b| b.right).max().unwrap_or(0);
    let line = kept.iter().map(|b| b.bottom).max().unwrap_or(0);
    let dot =
        |b: &Blob| b.bottom + 1 >= line && (b.bottom - b.top + 1) as f32 <= tallest as f32 * DOT;
    // A speck of light (a pixel of the picture behind, enclosed by the
    // outline) is no piece.
    let speck = |b: &Blob| {
        ((b.right - b.left + 1).max(b.bottom - b.top + 1)) as f32 <= tallest as f32 * SPECK
    };
    // The hole of a character (the inside of an enclosed "0") is no piece.
    let hole = |b: &Blob| {
        kept.iter().any(|k| {
            k.left <= b.left && b.right <= k.right && k.top <= b.top && b.bottom <= k.bottom
        })
    };
    short
        .iter()
        .any(|b| b.left > first && b.right < last && !dot(b) && !hole(b) && !speck(b))
}

/// `blobs` with the pieces of a character side by side joined: those sharing
/// columns, together no wider than `tallest` (the fill of an outlined
/// character comes apart on a smaller screen; neighbours' fills, kept apart by
/// their outlines, share none).
fn side_by_side(mut blobs: Vec<Blob>, tallest: u32) -> Vec<Blob> {
    blobs.sort_by_key(|b| b.left);
    let mut out: Vec<Blob> = Vec::new();
    for b in blobs {
        match out.last_mut() {
            Some(a) if b.left <= a.right && b.right.max(a.right) - a.left < tallest => {
                a.right = a.right.max(b.right);
                a.top = a.top.min(b.top);
                a.bottom = a.bottom.max(b.bottom);
            }
            _ => out.push(b),
        }
    }
    out
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

/// The extent of `badge`'s colour in `rect`: of its parts (4-connected) at
/// least `BADGE_PART` of the largest one, which the text may cut the badge
/// into where the area leaves out its edge; not a thin text of a like colour.
fn badge_extent(image: &RgbImage, rect: Rect, badge: Ink) -> Option<Rect> {
    let (x0, y0, x1, y1) = rect;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut on: Vec<bool> = (0..w * h)
        .map(|i| badge(image.get_pixel(x0 + (i % w) as u32, y0 + (i / w) as u32).0))
        .collect();
    let mut parts: Vec<(usize, Rect)> = Vec::new();
    for start in 0..w * h {
        if !on[start] {
            continue;
        }
        on[start] = false;
        let (mut stack, mut count) = (vec![start], 0);
        let mut extent = (u32::MAX, u32::MAX, 0, 0);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            count += 1;
            extent = (
                extent.0.min(x),
                extent.1.min(y),
                extent.2.max(x + 1),
                extent.3.max(y + 1),
            );
            let near = [
                (x > 0).then(|| i - 1),
                (x + 1 < w as u32).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h as u32).then(|| i + w),
            ];
            for j in near.into_iter().flatten() {
                if on[j] {
                    on[j] = false;
                    stack.push(j);
                }
            }
        }
        parts.push((count, extent));
    }
    let most = parts.iter().map(|p| p.0).max()?;
    let (left, top, right, bottom) = parts
        .iter()
        .filter(|p| p.0 as f32 >= most as f32 * BADGE_PART)
        .map(|p| p.1)
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))?;
    Some((x0 + left, y0 + top, x0 + right, y0 + bottom))
}

/// A part of a badge is at least this share of its largest part.
const BADGE_PART: f32 = 0.1;

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
    let rect = match style.within {
        Some(badge) => match badge_extent(image, rect, badge) {
            Some(extent) => extent,
            None => return Vec::new(),
        },
        None => rect,
    };
    let (raw, width, height) = mask(image, rect, style.ink, style.skew);
    let (mut blobs, on) = blobs(raw.clone(), width, height, framed);
    let (w, h) = (width as u32, height as u32);
    blobs.retain(|b| match style.ground {
        Ground::Free => true,
        Ground::Top => b.top > 0,
        Ground::Around => b.left > 0 && b.top > 0 && b.right + 1 < w && b.bottom + 1 < h,
    });
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
    let (mut kept, short): (Vec<Blob>, Vec<Blob>) = blobs
        .into_iter()
        .partition(|b| (b.bottom - b.top + 1) as f32 >= tallest as f32 * style.tall);
    if style.entire {
        kept = side_by_side(kept, tallest);
        if piece_left_out(&kept, &short, tallest) {
            return Vec::new();
        }
    }
    let blobs: Vec<Blob> = kept
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
            // Each cell the share of its area the ink covers, so a character
            // drawn small and one drawn large come out alike.
            let right = (b.right + 1) as f32;
            for gy in 0..gh {
                for gx in 0..gw {
                    let (ax, ay) = (
                        b.left as f32 + gx as f32 * scale,
                        b.top as f32 + gy as f32 * scale,
                    );
                    let (bx, by) = ((ax + scale).min(right), ay + scale);
                    if ax >= right {
                        continue;
                    }
                    let (mut ink, mut area) = (0.0, 0.0);
                    for py in ay.floor() as usize..(by.ceil() as usize).min(height) {
                        let wy = by.min(py as f32 + 1.0) - ay.max(py as f32);
                        for px in ax.floor() as usize..bx.ceil() as usize {
                            let wx = bx.min(px as f32 + 1.0) - ax.max(px as f32);
                            area += wx * wy;
                            if on[py * width + px] {
                                ink += wx * wy;
                            }
                        }
                    }
                    out[gy * gw + gx] = if area > 0.0 { ink / area } else { 0.0 };
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
        shapes.iter().map(|s| self.character(style, s)).collect()
    }

    /// How far `shape` lies from the nearest reference of `style`.
    pub fn nearest(&self, style: &str, shape: &[f32]) -> Option<f32> {
        let refs = self.0.get(style)?.values().flatten();
        Some(
            refs.map(|r| distance(r, shape))
                .fold(f32::INFINITY, f32::min),
        )
    }

    /// The character `shape` is in `style`, if it is close to a reference and
    /// clearly nearer to it than to any other character.
    pub fn character(&self, style: &str, shape: &[f32]) -> Option<char> {
        let mut nearest: Vec<(char, f32)> = self
            .0
            .get(style)?
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(left: u32, right: u32, top: u32, bottom: u32) -> Blob {
        Blob {
            left,
            right,
            top,
            bottom,
        }
    }

    /// A badge's extent is that of its colour's large parts, the text's
    /// strokes cutting it apart, not of a thinner text of a like colour
    /// beside it (a navy stat beside the ability's slate badge).
    #[test]
    fn a_badge_is_its_colours_largest_part() {
        let slate = image::Rgb([66, 102, 146]);
        let mut image = RgbImage::from_pixel(60, 20, image::Rgb([240, 240, 240]));
        for y in 2..18 {
            // A stroke of the text beside, taller than the badge is wide.
            image.put_pixel(5, y, slate);
        }
        for x in 20..50 {
            for y in 4..16 {
                image.put_pixel(x, y, slate);
            }
        }
        // A stroke of the badge's text cuts its right end apart.
        for y in 4..16 {
            image.put_pixel(44, y, image::Rgb([240, 240, 0]));
        }
        let is_slate: Ink = |c| c == [66, 102, 146];
        assert_eq!(
            badge_extent(&image, (0, 0, 60, 20), is_slate),
            Some((20, 4, 50, 16))
        );
        assert_eq!(
            badge_extent(&image, (0, 0, 10, 20), is_slate),
            Some((5, 2, 6, 18))
        );
        let none: Ink = |_| false;
        assert_eq!(badge_extent(&image, (0, 0, 60, 20), none), None);
    }

    /// The "Lv.70" of an equipment level at 1920 wide (characters 15 px
    /// high): the fill of the "0" came apart in two pieces side by side,
    /// sharing columns. They are one character; neighbours stay apart.
    #[test]
    fn the_pieces_of_a_character_side_by_side_are_joined() {
        let (l, v, seven) = (
            blob(11, 18, 9, 23),
            blob(25, 34, 13, 23),
            blob(43, 52, 10, 23),
        );
        let pieces = vec![
            l.clone(),
            v.clone(),
            seven.clone(),
            blob(54, 61, 10, 23),
            blob(60, 64, 9, 19),
        ];
        assert_eq!(
            side_by_side(pieces, 15),
            vec![l, v, seven, blob(54, 64, 9, 23)]
        );
    }
}
