//! Telling which of some known words a text is, without reading its letters
//! one by one: the text is cut where each word's letters would part, and each
//! cut compared with the word's letter there. The students' names and the
//! shops' are told so.

use std::collections::BTreeMap;

use image::RgbImage;

use super::{
    Rect,
    bank::{TILE, Tile, Word},
};

/// How much wider or narrower than a word a text may be and still be it.
const ASPECT: (f32, f32) = (0.75, 1.33);
/// How much a text's width off the word's counts, against its letters'
/// distance (measured on 2026-10-14 over the students' names: with 20 the
/// right name was the nearest on every plate).
const WIDE: f32 = 20.0;

/// Which pixels of a text's area are ink, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ink {
    pub width: u32,
    pub height: u32,
    pub on: Vec<bool>,
}

impl Ink {
    fn at(&self, x: u32, y: u32) -> bool {
        self.on[(y * self.width + x) as usize]
    }

    /// The parts of `ink` in `rect` (8-connected), but those touching its
    /// edges (what lies around, as the light end of a chosen menu entry) and
    /// those of fewer than `least` pixels (specks).
    pub fn parts(image: &RgbImage, rect: Rect, ink: fn([u8; 3]) -> bool, least: usize) -> Self {
        let (x0, y0, x1, y1) = rect;
        let (width, height) = (x1 - x0, y1 - y0);
        let raw: Vec<bool> = (0..width * height)
            .map(|i| ink(image.get_pixel(x0 + i % width, y0 + i / width).0))
            .collect();
        let mut on = vec![false; raw.len()];
        let mut seen = vec![false; raw.len()];
        for start in 0..raw.len() {
            if !raw[start] || seen[start] {
                continue;
            }
            seen[start] = true;
            let (mut part, mut stack) = (Vec::new(), vec![start]);
            while let Some(i) = stack.pop() {
                part.push(i);
                let (x, y) = (i64::from(i as u32 % width), i64::from(i as u32 / width));
                for (nx, ny) in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (x + dx, y + dy))) {
                    if nx < 0 || ny < 0 || nx >= i64::from(width) || ny >= i64::from(height) {
                        continue;
                    }
                    let j = (ny * i64::from(width) + nx) as usize;
                    if raw[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            let edge = part.iter().any(|&i| {
                let (x, y) = (i as u32 % width, i as u32 / width);
                x == 0 || y == 0 || x + 1 == width || y + 1 == height
            });
            if !edge && part.len() >= least {
                for i in part {
                    on[i] = true;
                }
            }
        }
        Self { width, height, on }
    }

    /// The box the ink fills: left, right, top and bottom, the ends past it.
    pub fn bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let inked = (0..self.width * self.height).filter(|&i| self.on[i as usize]);
        let (mut left, mut right, mut top, mut bottom) = (u32::MAX, 0, u32::MAX, 0);
        for i in inked {
            let (x, y) = (i % self.width, i / self.width);
            (left, right) = (left.min(x), right.max(x + 1));
            (top, bottom) = (top.min(y), bottom.max(y + 1));
        }
        (left < right).then_some((left, right, top, bottom))
    }

    /// The ink of columns `x0` to `x1` across rows `y0` to `y1`, narrowed to
    /// the columns it reaches, scaled to a tile.
    pub fn tile(&self, x0: f32, x1: f32, y0: f32, y1: f32) -> Tile {
        let rows = (y0.max(0.0) as u32)..(y1.ceil() as u32).min(self.height);
        let columns = (x0.max(0.0) as u32)..(x1.ceil() as u32).min(self.width);
        let inked: Vec<u32> = columns
            .filter(|&x| rows.clone().any(|y| self.at(x, y)))
            .collect();
        let (x0, x1) = match (inked.first(), inked.last()) {
            (Some(&a), Some(&b)) => (a as f32, b as f32 + 1.0),
            _ => (x0, x1),
        };
        let (sx, sy) = ((x1 - x0) / TILE as f32, (y1 - y0) / TILE as f32);
        let mut tile = [0; TILE * TILE];
        for (at, value) in tile.iter_mut().enumerate() {
            let (c, r) = ((at % TILE) as f32, (at / TILE) as f32);
            let (ax, bx) = (x0 + c * sx, x0 + (c + 1.0) * sx);
            let (ay, by) = (y0 + r * sy, y0 + (r + 1.0) * sy);
            let (mut seen, mut inked) = (0u32, 0u32);
            let mut y = ay.floor().max(0.0) as u32;
            while (y as f32) < by && y < self.height {
                let mut x = ax.floor().max(0.0) as u32;
                while (x as f32) < bx && x < self.width {
                    seen += 1;
                    inked += u32::from(self.at(x, y));
                    x += 1;
                }
                y += 1;
            }
            *value = (inked * 255).checked_div(seen).map_or(0, |v| v as u8);
        }
        tile
    }

    /// The ink's letters as `word` parts them, each a tile.
    pub fn letters(&self, word: &Word) -> Vec<Tile> {
        let Some((left, right, top, bottom)) = self.bounds() else {
            return Vec::new();
        };
        let width = (right - left) as f32;
        let at = |b: u8| left as f32 + f32::from(b) / 255.0 * width;
        word.bounds
            .windows(2)
            .map(|b| self.tile(at(b[0]), at(b[1]), top as f32, bottom as f32))
            .collect()
    }
}

/// How far apart two tiles are: the squared differences of their pixels (ink
/// counted 0 to 1), the second moved up to a pixel either way to meet the first.
pub fn distance(a: &Tile, b: &Tile) -> f32 {
    let side = TILE as i32;
    let mut nearest = u32::MAX;
    for (dx, dy) in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))) {
        let mut sum = 0u32;
        for y in 0..side {
            for x in 0..side {
                let (bx, by) = (x + dx, y + dy);
                let moved = if (0..side).contains(&bx) && (0..side).contains(&by) {
                    b[(by * side + bx) as usize]
                } else {
                    0
                };
                let d = i32::from(a[(y * side + x) as usize]) - i32::from(moved);
                sum += d.unsigned_abs() * d.unsigned_abs();
            }
            // No nearer than the nearest already: the rest need not be added.
            if sum >= nearest {
                break;
            }
        }
        nearest = nearest.min(sum);
    }
    nearest as f32 / (255.0 * 255.0)
}

/// How `ink` compares with `word`: the mean distance of its letters, with how
/// far its width over height lies from the word's, and the farthest letter.
/// The game's own `letters` stand in for the word's where there are some.
/// `None` where the widths differ too much for the same word.
pub fn compare(
    ink: &Ink,
    word: &Word,
    tiles: &[Tile],
    letters: &BTreeMap<char, Vec<usize>>,
) -> Option<(f32, f32)> {
    within(ink, word, tiles, letters, f32::INFINITY)
}

/// As `compare`, but `None` too as soon as the mean comes past `limit`.
fn within(
    ink: &Ink,
    word: &Word,
    tiles: &[Tile],
    letters: &BTreeMap<char, Vec<usize>>,
    limit: f32,
) -> Option<(f32, f32)> {
    let (left, right, top, bottom) = ink.bounds()?;
    let ratio = ((right - left) as f32 / (bottom - top) as f32) / word.aspect;
    if !(ASPECT.0..ASPECT.1).contains(&ratio) {
        return None;
    }
    let wide = WIDE * ratio.ln().abs();
    let count = word.tiles.len().max(1) as f32;
    let (mut sum, mut worst) = (0.0, 0.0f32);
    let cut = ink.letters(word);
    let chars = word.text.chars().map(Some).chain(std::iter::repeat(None));
    for ((tile, &font), c) in cut.iter().zip(&word.tiles).zip(chars) {
        let own = c.and_then(|c| letters.get(&c));
        let near = own
            .map_or(&[font][..], Vec::as_slice)
            .iter()
            .map(|&r| distance(tile, &tiles[r]))
            .fold(f32::INFINITY, f32::min);
        sum += near;
        worst = worst.max(near);
        if sum / count + wide > limit {
            return None;
        }
    }
    Some((sum / count + wide, worst))
}

/// The word among `words` that `ink` is nearest to, when none of its letters
/// lies farther than `worst` from the word's.
pub fn nearest<'a>(
    ink: &Ink,
    words: impl IntoIterator<Item = &'a Word>,
    tiles: &[Tile],
    letters: &BTreeMap<char, Vec<usize>>,
    worst: f32,
) -> Option<&'a Word> {
    let (left, right, top, bottom) = ink.bounds()?;
    let aspect = (right - left) as f32 / (bottom - top) as f32;
    // The words as wide as the ink first: the nearest is likely among them,
    // and the rest are left as soon as they come past it.
    let mut words: Vec<(f32, &Word)> = words
        .into_iter()
        .map(|w| (WIDE * (aspect / w.aspect).ln().abs(), w))
        .collect();
    words.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<((f32, f32), &Word)> = None;
    for (wide, word) in words {
        let limit = best.map_or(f32::INFINITY, |((mean, _), _)| mean);
        if wide > limit {
            break;
        }
        if let Some(compared) = within(ink, word, tiles, letters, limit) {
            best = Some((compared, word));
        }
    }
    best.filter(|((_, far), _)| *far <= worst).map(|(_, w)| w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ink drawn from rows of "#" and ".".
    fn ink(rows: &[&str]) -> Ink {
        Ink {
            width: rows[0].len() as u32,
            height: rows.len() as u32,
            on: rows
                .iter()
                .flat_map(|r| r.chars().map(|c| c == '#'))
                .collect(),
        }
    }

    /// Two letters side by side: a bar standing up, then a box.
    fn bar_and_box() -> Ink {
        ink(&[
            "..##......####..",
            "..##......#..#..",
            "..##......#..#..",
            "..##......####..",
        ])
    }

    #[test]
    fn the_ink_is_its_parts_off_the_edges_and_specks_left_out() {
        let mut image = RgbImage::new(8, 6);
        let white = image::Rgb([255, 255, 255]);
        // A part touching the top, one touching the right edge (the light
        // end of a menu entry), a speck, and a part of four pixels.
        image.put_pixel(1, 0, white);
        image.put_pixel(1, 1, white);
        for y in 2..5 {
            image.put_pixel(7, y, white);
        }
        image.put_pixel(1, 4, white);
        for (x, y) in [(3, 2), (4, 2), (3, 3), (4, 3)] {
            image.put_pixel(x, y, white);
        }
        let parts = Ink::parts(&image, (0, 0, 8, 6), |c| c[0] > 200, 2);
        assert_eq!(parts.bounds(), Some((3, 5, 2, 4)));
        assert_eq!(parts.on.iter().filter(|&&on| on).count(), 4);
    }

    #[test]
    fn a_letter_tile_is_its_ink_narrowed_to_its_columns() {
        let ink = bar_and_box();
        // The bar's window is wide, but the tile holds the bar alone: full.
        let tile = ink.tile(0.0, 8.0, 0.0, 4.0);
        assert!(tile.iter().all(|&v| v == 255));
        // The box: its outline on, its middle off.
        let tile = ink.tile(8.0, 16.0, 0.0, 4.0);
        assert_eq!(tile[0], 255);
        assert_eq!(tile[TILE * TILE / 2 + TILE / 2], 0);
    }

    #[test]
    fn a_text_is_nearest_the_word_it_spells_and_far_from_another() {
        let ink = bar_and_box();
        let mut tiles = Vec::new();
        let bar = ink.tile(0.0, 8.0, 0.0, 4.0);
        let square = ink.tile(8.0, 16.0, 0.0, 4.0);
        tiles.extend([bar, square]);
        let word = |id, tiles: Vec<usize>| Word {
            id,
            text: String::new(),
            aspect: 12.0 / 4.0,
            bounds: vec![0, 128, 255],
            tiles,
        };
        let (right, wrong) = (word(1, vec![0, 1]), word(2, vec![1, 0]));
        let none = BTreeMap::new();
        let (mean, worst) = compare(&ink, &right, &tiles, &none).unwrap();
        assert!(mean < 1.0 && worst < 1.0, "{mean} {worst}");
        assert!(compare(&ink, &wrong, &tiles, &none).unwrap().1 > 10.0);
        let words = [wrong.clone(), right.clone()];
        assert_eq!(nearest(&ink, &words, &tiles, &none, 5.0), Some(&words[1]));
        // Only the wrong word known: nothing is near enough.
        assert_eq!(nearest(&ink, [&wrong], &tiles, &none, 5.0), None);
        // A word twice as wide is no match at all.
        let wide = Word {
            aspect: 6.0,
            ..right
        };
        assert_eq!(compare(&ink, &wide, &tiles, &none), None);
    }

    #[test]
    fn the_games_letters_stand_in_for_the_fonts() {
        let ink = bar_and_box();
        let tiles = vec![[0; TILE * TILE], ink.tile(0.0, 8.0, 0.0, 4.0)];
        let word = Word {
            id: 1,
            text: "I口".into(),
            aspect: 3.0,
            bounds: vec![0, 128, 255],
            tiles: vec![0, 0],
        };
        let letters = BTreeMap::from([('I', vec![1])]);
        let without = compare(&ink, &word, &tiles, &BTreeMap::new()).unwrap();
        let with = compare(&ink, &word, &tiles, &letters).unwrap();
        assert!(with.0 < without.0, "{with:?} {without:?}");
    }

    #[test]
    fn tiles_a_pixel_apart_are_near() {
        let mut a = [0; TILE * TILE];
        let mut b = [0; TILE * TILE];
        a[5 * TILE + 5] = 255;
        b[5 * TILE + 6] = 255;
        assert!(distance(&a, &b) < 0.01);
        b[5 * TILE + 6] = 0;
        b[12 * TILE + 12] = 255;
        assert!((distance(&a, &b) - 2.0).abs() < 0.01);
    }
}
