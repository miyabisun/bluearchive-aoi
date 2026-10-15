//! Reads the "xN" count at the bottom right of a card.
//!
//! The count is drawn in one flat ink colour with a white outline, so each
//! character is a separate blob of ink. Blobs are scaled to a common height
//! and compared with reference characters (the digits and the "K" of counts
//! from 10,000) taken from labeled screenshots
//! (`data/digits.json`, written by `cargo run --example build_digits`).

use std::{collections::BTreeMap, fmt};

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
/// How far a digit may reach into the next, of the digits' height.
const OVERLAP: f32 = 0.25;
/// The "x" is shorter than the digits it precedes.
const X_HEIGHT: (f32, f32) = (0.45, 0.85);
/// Normalized glyph size: characters keep their width relative to their height.
pub const GLYPH: (usize, usize) = (16, 20);
/// Farthest a glyph may be from its reference (squared difference summed over
/// the normalized glyph). Measured on 2026-10-11: read characters are within
/// 35, a "K" without its reference is 73 or more from every digit.
const MAX_DISTANCE: f32 = 50.0;
/// The right-aligned count ends this close to the card's right edge (fraction
/// of the card height; measured 0.10 to 0.15). Farther, its last character is
/// missing, as under the blinking frame of the selected card (0.24).
const END_GAP: f32 = 0.19;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

/// The pieces of a character one above the other, as one blob: `a` and `b`
/// each shorter than a character (`heights`, the least and most of one),
/// sharing columns, no more than a pixel of rows, and together a character's
/// height (a thin stroke broken on a small screen); `None` for two characters
/// side by side, or a piece of a picture over one.
fn joined_pair(a: &Blob, b: &Blob, heights: (u32, u32)) -> Option<Blob> {
    let both = Blob {
        left: a.left.min(b.left),
        right: a.right.max(b.right),
        top: a.top.min(b.top),
        bottom: a.bottom.max(b.bottom),
    };
    let pieces = a.height() < heights.0 && b.height() < heights.0;
    let share_columns = a.left <= b.right && b.left <= a.right;
    let stacked = a.bottom < b.top + 2 || b.bottom < a.top + 2;
    let whole = (heights.0..=heights.1).contains(&both.height());
    (pieces && share_columns && stacked && whole).then_some(both)
}

/// `blobs` with the pieces of each character joined (see `joined_pair`).
/// Specks (under a third of a character's least height) are no pieces: a
/// card's picture leaves thousands of them, too many to pair.
fn joined(blobs: Vec<Blob>, heights: (u32, u32)) -> Vec<Blob> {
    let least = (heights.0 / 3).max(2);
    let (mut pieces, mut out): (Vec<Blob>, Vec<Blob>) = blobs
        .into_iter()
        .partition(|b| (least..heights.0).contains(&b.height()));
    let mut i = 0;
    while i < pieces.len() {
        let pair = (i + 1..pieces.len())
            .find_map(|j| Some((j, joined_pair(&pieces[i], &pieces[j], heights)?)));
        if let Some((j, both)) = pair {
            pieces.swap_remove(j);
            pieces.swap_remove(i);
            out.push(both);
        } else {
            i += 1;
        }
    }
    out.extend(pieces);
    out
}

/// The blobs of the card's count, if a count is found: the "x" before it, and
/// its digits left to right.
fn digit_blobs(image: &RgbImage, card: &Card) -> Option<(Blob, Vec<Blob>)> {
    let h = card.height as f32;
    let y0 = card.top + (h * COUNT_TOP) as u32;
    let y1 = (card.top + card.height).min(image.height());
    // At the count's rows the sheared card sits left of its centre-row position.
    let shift = card.shift_at(y1 as f32);
    let x0 = (card.left + shift).max(0.0) as u32;
    let x1 = ((card.left + card.width + card.shift_at(y0 as f32)) as u32).min(image.width());
    let heights = (
        (h * DIGIT_HEIGHT.0).ceil() as u32,
        (h * DIGIT_HEIGHT.1).floor() as u32,
    );
    let all = joined(blobs(image, x0, x1, y0, y1), heights);
    let right = card.left + card.width + card.shift_at(y0 as f32);
    count_blobs(&all, h, right)
}

/// Of the blobs at the foot of a card `h` high whose right edge is at `right`,
/// the count's: the "x" before it, and its digits left to right.
fn count_blobs(all: &[Blob], h: f32, right: f32) -> Option<(Blob, Vec<Blob>)> {
    let is_digit = |b: &Blob| {
        let r = b.height() as f32 / h;
        r >= DIGIT_HEIGHT.0 && r <= DIGIT_HEIGHT.1
    };
    // The count is right-aligned: start from the rightmost digit-sized blob,
    // which must end where counts end.
    let last = all.iter().filter(|b| is_digit(b)).max_by_key(|b| b.right)?;
    if (right - last.right as f32) / h > END_GAP {
        return None;
    }
    let line = |b: &Blob| b.bottom.abs_diff(last.bottom) <= 2;
    let mut digits: Vec<Blob> = vec![last.clone()];
    loop {
        let first = &digits[0];
        let gap_limit = first.height() as f32 * 0.6;
        // Italic digits may overlap a little ("72" by 2 pixels at 165); a
        // character is no wider than tall (the icon's foot may reach as far).
        // A speck shorter than the "x" (left by H.264 inside a digit) is
        // neither.
        let overlap = (first.height() as f32 * OVERLAP) as u32;
        let shortest = last.height() as f32 * X_HEIGHT.0;
        let next = all
            .iter()
            .filter(|b| b.right < first.left + overlap && line(b))
            .filter(|b| b.right - b.left < first.height() && b.height() as f32 >= shortest)
            .filter(|b| (first.left as f32 - b.right as f32) <= gap_limit)
            .max_by_key(|b| b.right)?;
        if is_digit(next) && next.top.abs_diff(last.top) <= 2 {
            digits.insert(0, next.clone());
            continue;
        }
        // The blob before the digits must be the "x".
        let r = next.height() as f32 / last.height() as f32;
        return (r >= X_HEIGHT.0 && r <= X_HEIGHT.1).then(|| (next.clone(), digits));
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
            // Between pixels, a blend of the four around (bilinear): a small
            // glyph keeps its strokes' share of each pixel.
            let (fx, fy) = (x - 0.5, y - 0.5);
            let (x0, y0) = (fx.floor().max(0.0), fy.floor().max(0.0));
            let (tx, ty) = ((fx - x0).clamp(0.0, 1.0), (fy - y0).clamp(0.0, 1.0));
            let at = |x: f32, y: f32| {
                let (x, y) = (x as u32, y as u32);
                if x < image.width() && y < image.height() {
                    ink_share(image.get_pixel(x, y).0).unwrap_or(0.0)
                } else {
                    0.0
                }
            };
            out[gy * gw + gx] = (1.0 - ty) * ((1.0 - tx) * at(x0, y0) + tx * at(x0 + 1.0, y0))
                + ty * ((1.0 - tx) * at(x0, y0 + 1.0) + tx * at(x0 + 1.0, y0 + 1.0));
        }
    }
    out
}

/// Normalized digit glyphs of the card's count, left to right.
pub fn glyphs(image: &RgbImage, card: &Card) -> Option<Vec<Vec<f32>>> {
    count_glyphs(image, card).map(|(_, digits)| digits)
}

/// The normalized glyphs of the card's count: its "x", and its digits left to
/// right.
pub fn count_glyphs(image: &RgbImage, card: &Card) -> Option<(Vec<f32>, Vec<Vec<f32>>)> {
    let (x, digits) = digit_blobs(image, card)?;
    Some((
        normalize(image, &x),
        digits.iter().map(|b| normalize(image, b)).collect(),
    ))
}

/// A count as the list shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Count {
    /// Below 10,000 the exact number ("x1383").
    Exact(u32),
    /// From 10,000 the thousands, rounded ("x51K" is 50,500 to 51,499).
    Thousands(u32),
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Count::Exact(n) => write!(f, "{n}"),
            Count::Thousands(n) => write!(f, "{n}K"),
        }
    }
}

/// The count spelled by the characters after the "x"; a "K" may only end it.
fn parse(text: &str) -> Option<Count> {
    match text.strip_suffix('K') {
        Some(thousands) => thousands.parse().ok().map(Count::Thousands),
        None => text.parse().ok().map(Count::Exact),
    }
}

/// Reference glyphs of each character of a count, the "x" before it, the
/// digits and "K": for each size the list is drawn at, from a PC window to a
/// small phone's screen (small digits lose their holes, an "8" coming near a
/// "6"), and for each stop and size of a tablet's stream (H.264 blurs them
/// further).
#[derive(Serialize, Deserialize)]
pub struct Digits {
    pub glyphs: BTreeMap<char, Vec<Vec<f32>>>,
}

/// The nearest character must be at most this share of the distance to the
/// next nearest one, or the character is unknown: a small "8" near a "6" is no
/// digit. Measured on 2026-10-15 over the evaluation screens: characters read
/// right are at most 0.83 at the screens' own size; over a tablet's stream at
/// 1600 wide (119 px cards), the ones read wrong 0.94 or more, 90% of those
/// read right under 0.41.
const LEAD: f32 = 0.85;

impl Digits {
    /// The character the glyph shows, if it is close enough to one and clearly
    /// nearer to it than to any other (see `LEAD`) of those `possible`.
    fn closest(&self, glyph: &[f32], possible: impl Fn(char) -> bool) -> Option<char> {
        let dist = |r: &Vec<f32>| {
            r.iter()
                .zip(glyph)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f32>()
        };
        let mut nearest: Vec<(char, f32)> = self
            .glyphs
            .iter()
            .filter(|(c, _)| possible(**c))
            .filter_map(|(c, refs)| Some((*c, refs.iter().map(dist).min_by(f32::total_cmp)?)))
            .collect();
        nearest.sort_by(|a, b| a.1.total_cmp(&b.1));
        let (c, d) = *nearest.first()?;
        let next = nearest.get(1).map_or(f32::INFINITY, |n| n.1);
        (d <= MAX_DISTANCE && d <= LEAD * next).then_some(c)
    }

    pub fn read(&self, image: &RgbImage, card: &Card) -> Option<Count> {
        let (x, digits) = count_glyphs(image, card)?;
        // What stands before the digits must be told as the "x" (a "K", which
        // only ends a count, is not taken for it): a piece of a digit broken
        // on a small screen would leave the count short.
        if self.closest(&x, |c| c != 'K') != Some('x') {
            return None;
        }
        let text: Option<String> = digits.iter().map(|g| self.closest(g, |_| true)).collect();
        parse(&text?)
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

    /// On a small screen a thin stroke breaks: the "7" of "x672" at 1280 came
    /// in a top and a foot. Pieces one above the other are one character;
    /// neighbours side by side, touching or not, stay apart.
    #[test]
    fn the_pieces_of_a_character_are_joined() {
        let x = blob(1162, 1173, 330, 340);
        let six = blob(1174, 1184, 326, 340);
        let (top, foot) = (blob(1188, 1197, 326, 332), blob(1186, 1191, 334, 341));
        let two = blob(1196, 1207, 326, 340);
        // Digits of a 95 px card are 11 to 19 px high.
        let heights = (11, 19);
        let mut joined = joined(
            vec![x.clone(), six.clone(), top, foot, two.clone()],
            heights,
        );
        joined.sort_by_key(|b| b.left);
        let seven = blob(1186, 1197, 326, 341);
        assert_eq!(joined, vec![x, six, seven, two]);
        // Italic neighbours overlapping a little ("72") are two characters.
        let (seven, two) = (blob(100, 111, 10, 25), blob(110, 121, 10, 25));
        assert_eq!(joined_pair(&seven, &two, heights), None);
        // A digit with a piece of the icon above it stays as it is.
        let (foot, nine) = (blob(150, 160, 0, 8), blob(150, 160, 10, 25));
        assert_eq!(joined_pair(&foot, &nine, heights), None);
    }

    /// The "x12" of a 143 px card on the tablet's 1920 stream, with the
    /// speck H.264 left inside the "1".
    #[test]
    fn specks_do_not_end_a_count() {
        let x = blob(1285, 1302, 951, 967);
        let (one, two) = (blob(1307, 1316, 945, 967), blob(1319, 1336, 945, 967));
        let all = [
            blob(1276, 1284, 941, 948),
            x.clone(),
            blob(1286, 1288, 942, 946),
            one.clone(),
            blob(1308, 1308, 966, 967),
            two.clone(),
        ];
        assert_eq!(count_blobs(&all, 143.0, 1354.0), Some((x, vec![one, two])));
    }

    #[test]
    fn counts_parse_as_the_list_shows_them() {
        assert_eq!(parse("1383"), Some(Count::Exact(1383)));
        assert_eq!(parse("51K"), Some(Count::Thousands(51)));
        assert_eq!(parse("5K1"), None);
        assert_eq!(parse("K"), None);
        assert_eq!(parse(""), None);
        assert_eq!(Count::Exact(1383).to_string(), "1383");
        assert_eq!(Count::Thousands(51).to_string(), "51K");
    }

    /// A character with no reference is unreadable, never the closest digit:
    /// with "K" left out, "x51K" must not read as 518.
    #[test]
    fn a_character_without_a_reference_is_unreadable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut digits: Digits =
            serde_json::from_str(&std::fs::read_to_string(root.join("data/digits.json")).unwrap())
                .unwrap();
        digits.glyphs.remove(&'K');
        let image = image::open(root.join("tests/fixtures/equipment-16x9-end.png"))
            .unwrap()
            .to_rgb8();
        let cards = find_cards(&image);
        let read: Vec<Option<Count>> = cards[16..].iter().map(|c| digits.read(&image, c)).collect();
        assert_eq!(read, vec![None; 9]);
        assert_eq!(digits.read(&image, &cards[11]), Some(Count::Exact(1444)));
    }
    use crate::grid::find_cards;
    use std::path::Path;

    #[derive(Deserialize)]
    struct Fixture {
        image: String,
        cells: Vec<Label>,
    }

    #[derive(Deserialize)]
    struct Label {
        quantity: Option<String>,
        #[serde(default)]
        selected: bool,
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
                let want = label.quantity.as_ref().map(|q| q.chars().count());
                let got = glyphs(&image, card).map(|g| g.len());
                // Under the selected card's frame the count may be unreadable.
                if got != want && !(label.selected && got.is_none()) {
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
