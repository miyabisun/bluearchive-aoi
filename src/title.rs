//! The title the game writes at a screen's top left ("学園交流会"), read as a
//! word among the titles Aoi tells screens by (`TITLES`), as the shops' names
//! are (see `glyph::word`): which screen the game shows where no list, student
//! or shop is read. The 任務 screen, "任務" on its Normal and Hard tabs alike,
//! is told by the campaign it shows (see `campaign`), and the shop screen, whose
//! offers are not read, by the shop its menu has chosen (see `shop::told`). A
//! title ends at the "?" button the game draws after it; the schedule's with a
//! location chosen has none (see `OPEN`), and which location is chosen there is
//! read apart (see `schedule`).

use image::RgbImage;

use crate::{
    campaign,
    glyph::{
        bank::{Bank, Word},
        word::{self, Ink},
    },
    schedule,
    screen::{self, Anchor},
    shop,
    student::{self, Area},
};

/// The screens told by their title, by the title.
pub const TITLES: [&str; 7] = [
    "学園交流会",
    "指名手配",
    "任務",
    "特別依頼",
    "ショップ",
    "スケジュール",
    schedule::CHOSEN,
];

/// The titles no "?" button follows: the schedule's once a location is chosen
/// (the user's 2026-11-23 「「スケジュールを選択」にタイトルが切り替わるため、
/// 対象外の画面になってしまう」). A title without a button is compared with
/// these alone and a title before one with the others alone, so the top left
/// of a screen without the button reads as none of the others.
pub const OPEN: [&str; 1] = [schedule::CHOSEN];

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
/// least `BUTTON` wide whose box is as tall and `BUTTON_FILL` dark or more. The
/// gaps tell it not: measured on 2026-11-06 over the titles drawn 1500 and 2370
/// wide, the button starts 0.0142 to 0.0154 after the title, and "ショップ"'s
/// letters part by up to 0.0130. The button is 0.034 wide, 0.033 tall and 0.85
/// dark; no letter as wide is darker than 0.66 (学). The "ー" of "スケジュール"
/// is as wide (0.033) and dark all over (1.00), but 0.004 tall (measured on
/// 2026-11-22 at 2370x1333).
const BUTTON: f32 = 0.025;
const BUTTON_FILL: f32 = 0.75;
/// A title no button follows ends at the first gap this wide, or the area's
/// end. Measured on 2026-11-23 at 2370x1333 on "スケジュールを選択": its
/// letters part by up to 0.0068 and nothing follows it in the area (the AP's
/// icon starts at 0.67); "ショップ"'s part by up to 0.0130.
const GAP: f32 = 0.03;
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
/// screen, nearest 任務). スケジュール, measured so on 2026-11-22: within 6.8 of
/// its own title, 68.5 or farther from the others'. スケジュールを選択, measured
/// so on 2026-11-23: within 17.3 of its own title; no other title is as wide
/// for its height (`word::compare` tells none), nor the top left of any other
/// screen, with a button or without (275 fixtures and 804 captures kept
/// outside the repository).
const WORST: f32 = 32.0;

/// The title's dark letters on the light top bar.
fn dark(c: [u8; 3]) -> bool {
    let [r, g, b] = c.map(u32::from);
    (299 * r + 587 * g + 114 * b) / 1000 < 128
}

/// The columns the title spans across `rows` (dark or not, each left to
/// right): from the first dark column to the last before the button, the
/// first run of dark columns at least `button` wide whose box is at least
/// `button` tall and `BUTTON_FILL` dark. `None` without letters before a
/// button.
pub fn span(rows: &[Vec<bool>], button: usize) -> Option<(usize, usize)> {
    let runs = runs(rows);
    let at = button_at(rows, &runs, button)?;
    Some((runs.first()?.0, runs[at.checked_sub(1)?].1))
}

/// The columns a title no button follows spans (see `OPEN`): from the first
/// dark column to the last before a gap at least `gap` wide, what lies past it
/// (the AP's digits) left out. `None` where a button shows (see `span`: its
/// screens are read up to it) or nothing is dark.
pub fn open_span(rows: &[Vec<bool>], button: usize, gap: usize) -> Option<(usize, usize)> {
    let runs = runs(rows);
    if button_at(rows, &runs, button).is_some() {
        return None;
    }
    let first = runs.first()?.0;
    let parted = runs.windows(2).find(|pair| pair[1].0 - pair[0].1 > gap);
    Some((
        first,
        parted.map_or(runs[runs.len() - 1].1, |pair| pair[0].1),
    ))
}

/// The runs of dark columns across `rows`, each its first and last column.
fn runs(rows: &[Vec<bool>]) -> Vec<(usize, usize)> {
    let width = rows.first().map_or(0, Vec::len);
    word::runs((0..width).map(|x| rows.iter().any(|row| row[x])))
}

/// Which of `runs` is the button: the first at least `button` wide whose box
/// is at least `button` tall and `BUTTON_FILL` dark.
fn button_at(rows: &[Vec<bool>], runs: &[(usize, usize)], button: usize) -> Option<usize> {
    let filled = |&(first, last): &(usize, usize)| {
        let on = |row: &Vec<bool>| row[first..=last].iter().filter(|&&on| on).count();
        let dark: Vec<usize> = rows.iter().map(on).collect();
        let top = dark.iter().position(|&n| n > 0).unwrap_or(0);
        let bottom = dark.iter().rposition(|&n| n > 0).unwrap_or(0);
        let height = bottom - top + 1;
        let box_size = (last - first + 1) * height;
        height >= button && dark.iter().sum::<usize>() as f32 >= BUTTON_FILL * box_size as f32
    };
    runs.iter()
        .position(|run| run.1 - run.0 + 1 >= button && filled(run))
}

/// The letters of the title and whether no button follows them (see `OPEN`);
/// `None` where the top left is too small or holds no title.
fn cut(image: &RgbImage) -> Option<(Ink, bool)> {
    let u = screen::unit(image.width(), image.height());
    let (x0, y0, x1, y1) = student::rect(image, AREA)?;
    let rows: Vec<Vec<bool>> = (y0..y1)
        .map(|y| (x0..x1).map(|x| dark(image.get_pixel(x, y).0)).collect())
        .collect();
    let button = (BUTTON * u).round().max(1.0) as usize;
    let gap = (GAP * u).round().max(1.0) as usize;
    let ((first, last), open) = match span(&rows, button) {
        Some(span) => (span, false),
        None => (open_span(&rows, button, gap)?, true),
    };
    // Room for the letters short of the area's edges: what touches them is
    // the ground around.
    let room = (ROOM * u).round() as u32;
    let left = (x0 + first as u32).saturating_sub(room).max(x0);
    let right = (x0 + last as u32 + 1 + room).min(x1);
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, (left, y0, right, y1), dark, speck);
    ink.bounds().map(|_| (ink, open))
}

/// The letters of the title, the button after it left out; `None` where the
/// top left is too small or holds no title.
pub fn ink(image: &RgbImage) -> Option<Ink> {
    cut(image).map(|(ink, _)| ink)
}

/// The screen whose title the top left writes, among `TITLES`; the 任務
/// screen's while it shows a campaign alone, by the campaign ("任務 Normal"),
/// and the shop screen's by the shop chosen ("ショップ: 通常アイテム"). A title
/// before a button is one of those written so, a title without one of `OPEN`.
pub fn read(image: &RgbImage, glyphs: &Bank) -> Option<&'static str> {
    let (ink, open) = cut(image)?;
    let none = std::collections::BTreeMap::new();
    let ended_so = |word: &&Word| OPEN.contains(&word.text.as_str()) == open;
    let titles = glyphs.index.titles.iter().filter(ended_so);
    let named = word::nearest(&ink, titles, &glyphs.tiles, &none, WORST)?;
    match *TITLES.get(usize::try_from(named.id).ok()?)? {
        "任務" => campaign::read(image, glyphs),
        "ショップ" => shop::told(image, glyphs),
        title => Some(title),
    }
}

#[cfg(test)]
mod tests {
    use super::{open_span, span};

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
            "..#...#..#..####.",
        ]);
        assert_eq!(span(&screen, 4), Some((2, 9)));
    }

    #[test]
    fn a_long_flat_stroke_among_the_letters_is_not_the_button() {
        // The "ー" of "スケジュール": as wide as the button and dark all over, a line high.
        let screen = grid(&[
            "..##........#.####.",
            "..#..#####..#.####.",
            "..##........#.####.",
            "..#.........#.####.",
        ]);
        assert_eq!(span(&screen, 4), Some((2, 12)));
    }

    #[test]
    fn a_title_without_a_button_runs_to_its_last_ink_or_the_first_wide_gap() {
        // No box as wide and tall as the button anywhere: the letters, parted by narrow gaps.
        let alone = grid(&[
            "..##..#..#.......",
            "..#...####.......",
            "..##..#..#.......",
            "..#...#..#.......",
        ]);
        assert_eq!(open_span(&alone, 4, 4), Some((2, 9)));
        // What lies past a gap as wide as `gap` (the AP's digits) is no part of it.
        let then_more = grid(&[
            "..##..#..#....#.#",
            "..#...####....#.#",
            "..##..#..#....###",
            "..#...#..#......#",
        ]);
        assert_eq!(open_span(&then_more, 4, 4), Some((2, 9)));
        assert_eq!(
            open_span(&then_more, 4, 5),
            Some((2, 16)),
            "a narrower gap parts no title"
        );
    }

    #[test]
    fn a_title_before_a_button_is_none_without_one() {
        // The button's screens are read up to the button (`span`), never this way.
        let buttoned = grid(&[
            "..##..#..#..####.",
            "..#...####..####.",
            "..##..#..#..####.",
            "..#...#..#..####.",
        ]);
        assert_eq!(open_span(&buttoned, 4, 4), None);
        assert_eq!(open_span(&grid(&["....", "...."]), 4, 4), None, "no ink");
    }

    #[test]
    fn no_title_is_told_without_the_button_or_before_it() {
        let square = |row: &'static str| grid(&[row; 4]);
        assert_eq!(span(&grid(&["##..##", "#...##"]), 4), None);
        assert_eq!(span(&square("####..#"), 4), None);
        assert_eq!(span(&square("...."), 4), None);
    }
}
