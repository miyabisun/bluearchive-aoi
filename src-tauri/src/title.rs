//! The title the game writes at a screen's top left ("学園交流会"), read as a
//! word among the titles Aoi tells screens by (`TITLES`), as the shops' names
//! are (see `glyph::word`): which screen the game shows where no list, student
//! or shop is read. The 任務 screen, "任務" on its Normal and Hard tabs alike,
//! is told by the campaign it shows (see `campaign`).

use image::RgbImage;

use crate::{
    campaign,
    glyph::{
        bank::Bank,
        word::{self, Ink},
    },
    screen::{self, Anchor},
    student::{self, Area},
};

/// The screens told by their title, by the title.
pub const TITLES: [&str; 3] = ["学園交流会", "指名手配", "任務"];

/// Where the title is written, in units (see `screen::unit`), kept to the
/// top. Measured on 2026-11-06 at 2370x1333: "学園交流会" from 0.150 to 0.326
/// across and 0.014 to 0.050 down; the back button ends at 0.122, the top bar
/// at 0.056; the AP's icon starts at 0.67.
pub const AREA: Area = Area {
    from_right: false,
    x: (0.13, 0.62),
    y: (0.006, 0.056),
    anchor: Anchor::Top,
};
/// A gap at least this wide ends the title: the "?" button after it starts
/// 0.017 on, the letters part by less than 0.006 (same screen).
const GAP: f32 = 0.011;
/// Parts of the letters smaller than this share of the unit squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// No letter of the title lies farther than this from the word's. Measured on
/// 2026-11-06 over the fixtures: 学園交流会, 指名手配 and 任務 drawn from 1500
/// to 2222 wide within 14.2 of their own titles, 61 or farther from each
/// other's, and every other screen's top left 53 or farther (a phone's student
/// screen, nearest 任務).
const WORST: f32 = 24.0;

/// The title's dark letters on the light top bar.
fn dark(c: [u8; 3]) -> bool {
    let [r, g, b] = c.map(u32::from);
    (299 * r + 587 * g + 114 * b) / 1000 < 128
}

/// The columns of the first run of `inked` ones, up to a gap of `gap` empty
/// ones or the end: its first and its last.
pub fn span(inked: &[bool], gap: usize) -> Option<(usize, usize)> {
    let first = inked.iter().position(|&on| on)?;
    let mut last = first;
    for (x, &on) in inked.iter().enumerate().skip(first) {
        if on {
            last = x;
        } else if x - last >= gap {
            break;
        }
    }
    Some((first, last))
}

/// The letters of the title, the button after it left out; `None` where the
/// top left is too small or holds no ink.
pub fn ink(image: &RgbImage) -> Option<Ink> {
    let u = screen::unit(image.width(), image.height());
    let (x0, y0, x1, y1) = student::rect(image, AREA)?;
    let inked: Vec<bool> = (x0..x1)
        .map(|x| (y0..y1).any(|y| dark(image.get_pixel(x, y).0)))
        .collect();
    let gap = (GAP * u).round().max(1.0) as usize;
    let (first, last) = span(&inked, gap)?;
    // Room for the letters short of the area's edges: what touches them is
    // the ground around.
    let room = (gap / 2) as u32;
    let left = (x0 + first as u32).saturating_sub(room).max(x0);
    let right = (x0 + last as u32 + 1 + room).min(x1);
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, (left, y0, right, y1), dark, speck);
    ink.bounds().map(|_| ink)
}

/// The screen whose title the top left writes, among `TITLES`; the 任務
/// screen's while it shows a campaign alone, by the campaign ("任務 Normal").
pub fn read(image: &RgbImage, glyphs: &Bank) -> Option<&'static str> {
    let ink = ink(image)?;
    let none = std::collections::BTreeMap::new();
    let named = word::nearest(&ink, &glyphs.index.titles, &glyphs.tiles, &none, WORST)?;
    match *TITLES.get(usize::try_from(named.id).ok()?)? {
        "任務" => campaign::read(image, glyphs),
        title => Some(title),
    }
}

#[cfg(test)]
mod tests {
    use super::span;

    #[test]
    fn the_title_runs_from_its_first_ink_to_a_gap_as_wide_as_the_one_before_the_button() {
        let columns = |s: &str| s.chars().map(|c| c == '#').collect::<Vec<_>>();
        // Letters parted by narrower gaps, then the button past a wide one.
        assert_eq!(span(&columns("..##.###..##....###"), 4), Some((2, 11)));
        // Ink up to the end.
        assert_eq!(span(&columns(".#.#"), 4), Some((1, 3)));
        assert_eq!(span(&columns("...."), 4), None);
    }
}
