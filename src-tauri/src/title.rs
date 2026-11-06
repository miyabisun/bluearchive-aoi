//! The title the game writes at a screen's top left ("学園交流会"), read as a
//! word among the titles Aoi tells screens by (`TITLES`), as the shops' names
//! are (see `glyph::word`): which screen the game shows where no list, student
//! or shop is read. The 任務 screen, "任務" on its Normal and Hard tabs alike,
//! is told by the campaign it shows (see `campaign`), and the shop screen, whose
//! offers are not read, by the shop its menu has chosen (see `shop::told`).

use image::RgbImage;

use crate::{
    campaign,
    glyph::{
        bank::Bank,
        word::{self, Ink},
    },
    screen::{self, Anchor},
    shop,
    student::{self, Area},
};

/// The screens told by their title, by the title.
pub const TITLES: [&str; 5] = ["学園交流会", "指名手配", "任務", "特別依頼", "ショップ"];

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
/// The "?" button after the title ends it: the first run of dark columns at
/// least `BUTTON` wide whose box is `BUTTON_FILL` dark or more. The gaps tell
/// it not: measured on 2026-11-06 over the titles drawn 1500 and 2370 wide,
/// the button starts 0.0142 to 0.0154 after the title, and "ショップ"'s letters
/// part by up to 0.0130. The button is 0.034 wide and 0.85 dark; no letter as
/// wide is darker than 0.66 (学).
const BUTTON: f32 = 0.025;
const BUTTON_FILL: f32 = 0.75;
/// Room kept around the letters, short of the area's edges and the button.
const ROOM: f32 = 0.005;
/// Parts of the letters smaller than this share of the unit squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// No letter of the title lies farther than this from the word's. Measured on
/// 2026-11-06 over the fixtures: 学園交流会, 指名手配, 任務, 特別依頼 and
/// ショップ drawn from 1500 to 2370 wide by 10 within 24.5 of their own titles
/// (特別依頼 at 1740: the tops of 特 and 別, kept in the 1600 bank's word, fall
/// out there and the tiles shift, the mean 12.7), 60 or farther from each
/// other's, and every other screen's top left 53 or farther (a phone's student
/// screen, nearest 任務).
const WORST: f32 = 32.0;

/// The title's dark letters on the light top bar.
fn dark(c: [u8; 3]) -> bool {
    let [r, g, b] = c.map(u32::from);
    (299 * r + 587 * g + 114 * b) / 1000 < 128
}

/// The columns the title spans across `rows` (dark or not, each left to
/// right): from the first dark column to the last before the button, the
/// first run of dark columns at least `button` wide and `BUTTON_FILL` dark
/// across its box. `None` without letters before a button.
pub fn span(rows: &[Vec<bool>], button: usize) -> Option<(usize, usize)> {
    let width = rows.first()?.len();
    let inked = |x: usize| rows.iter().any(|row| row[x]);
    let mut runs = Vec::new();
    let mut x = 0;
    while x < width {
        if inked(x) {
            let first = x;
            while x < width && inked(x) {
                x += 1;
            }
            runs.push((first, x - 1));
        } else {
            x += 1;
        }
    }
    let filled = |&(first, last): &(usize, usize)| {
        let on = |row: &Vec<bool>| row[first..=last].iter().filter(|&&on| on).count();
        let dark: Vec<usize> = rows.iter().map(on).collect();
        let top = dark.iter().position(|&n| n > 0).unwrap_or(0);
        let bottom = dark.iter().rposition(|&n| n > 0).unwrap_or(0);
        let box_size = (last - first + 1) * (bottom - top + 1);
        dark.iter().sum::<usize>() as f32 >= BUTTON_FILL * box_size as f32
    };
    let at = runs
        .iter()
        .position(|run| run.1 - run.0 + 1 >= button && filled(run))?;
    Some((runs.first()?.0, runs[at.checked_sub(1)?].1))
}

/// The letters of the title, the button after it left out; `None` where the
/// top left is too small or holds no title before a button.
pub fn ink(image: &RgbImage) -> Option<Ink> {
    let u = screen::unit(image.width(), image.height());
    let (x0, y0, x1, y1) = student::rect(image, AREA)?;
    let rows: Vec<Vec<bool>> = (y0..y1)
        .map(|y| (x0..x1).map(|x| dark(image.get_pixel(x, y).0)).collect())
        .collect();
    let button = (BUTTON * u).round().max(1.0) as usize;
    let (first, last) = span(&rows, button)?;
    // Room for the letters short of the area's edges: what touches them is
    // the ground around.
    let room = (ROOM * u).round() as u32;
    let left = (x0 + first as u32).saturating_sub(room).max(x0);
    let right = (x0 + last as u32 + 1 + room).min(x1);
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, (left, y0, right, y1), dark, speck);
    ink.bounds().map(|_| ink)
}

/// The screen whose title the top left writes, among `TITLES`; the 任務
/// screen's while it shows a campaign alone, by the campaign ("任務 Normal"),
/// and the shop screen's by the shop chosen ("ショップ: 通常アイテム").
pub fn read(image: &RgbImage, glyphs: &Bank) -> Option<&'static str> {
    let ink = ink(image)?;
    let none = std::collections::BTreeMap::new();
    let named = word::nearest(&ink, &glyphs.index.titles, &glyphs.tiles, &none, WORST)?;
    match *TITLES.get(usize::try_from(named.id).ok()?)? {
        "任務" => campaign::read(image, glyphs),
        "ショップ" => shop::told(image, glyphs),
        title => Some(title),
    }
}

#[cfg(test)]
mod tests {
    use super::span;

    fn grid(rows: &[&str]) -> Vec<Vec<bool>> {
        rows.iter()
            .map(|row| row.chars().map(|c| c == '#').collect())
            .collect()
    }

    #[test]
    fn the_title_runs_from_its_first_ink_to_the_last_before_the_button() {
        // Letters parted by gaps as wide as the one before the button: a run
        // as narrow as dense, a run as wide as sparse, then the filled box.
        let screen = grid(&[
            "..##..#..#..####.",
            "..#...####..####.",
            "..##..#..#..####.",
        ]);
        assert_eq!(span(&screen, 4), Some((2, 9)));
    }

    #[test]
    fn no_title_is_told_without_the_button_or_before_it() {
        assert_eq!(span(&grid(&["##..##", "#...##"]), 4), None);
        assert_eq!(span(&grid(&["####..#", "####..#"]), 4), None);
        assert_eq!(span(&grid(&["....", "...."]), 4), None);
    }
}
