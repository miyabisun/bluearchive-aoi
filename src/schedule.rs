//! The schedule's screens past its list of locations, which the title alone
//! does not tell apart: with a location chosen, the screen "スケジュールを選択"
//! writes the location's name after its rank at the top right, and its
//! "すべてのスケジュール" dialog, which dims the title and covers that name,
//! lists the location's rooms as cards, the first at its top left. Both are
//! read as words (see `glyph::word`), as the titles are, and tell the school
//! whose row the work tab's schedule marks (the user's 2026-11-23
//! 「この「ゲヘナ学園・中央区」を認識したらゲヘナ学園の行を黄色のborderで括る」
//! 「モーダルが表示されるので、左上の「ゲヘナ第一校舎」を認識する」).

use image::RgbImage;

use crate::{
    glyph::{
        bank::{Bank, Word},
        word::{self, Ink},
    },
    screen::{self, Anchor},
    student::{self, Area},
};

/// The title of the schedule's list of locations, the name the work tab's
/// fold and the badge know the schedule by.
pub const TITLE: &str = "スケジュール";
/// The title once a location is chosen; no "?" button follows it (see
/// `title::OPEN`).
pub const CHOSEN: &str = "スケジュールを選択";

/// A location of the schedule: its name as the chosen screen writes it, the
/// first room its dialog lists (open to every account from the start; the
/// user's same request: 「新規ユーザーは時間を掛けて全ての場所をアンロックする
/// 必要があるため」), and the school whose BDs and tech notes its rooms give,
/// where they give one school's alone.
#[derive(Debug, PartialEq, Eq)]
pub struct Location {
    pub name: &'static str,
    pub room: &'static str,
    pub school: Option<&'static str>,
}

const fn location(name: &'static str, room: &'static str, school: &'static str) -> Location {
    Location {
        name,
        room,
        school: Some(school),
    }
}

const fn shared(name: &'static str, room: &'static str) -> Location {
    Location {
        name,
        room,
        school: None,
    }
}

/// The schedule's locations in the game's order, read off the user's game on
/// 2026-11-23 (every location at rank 12) and the same in SchaleDB's schedule
/// data of that day. The four without a school give choice boxes and a
/// school's goods at random among six (the user's 2026-11-23 「シャーレ・共通
/// エリアは6学園のランダムな選択ボックスがドロップする」).
pub const LOCATIONS: [Location; 12] = [
    shared("シャーレオフィス", "視聴覚室"),
    shared("シャーレ居住区", "自習室"),
    location("ゲヘナ学園・中央区", "ゲヘナ第一校舎", "ゲヘナ"),
    location("アビドス高等学校", "アビドス本館", "アビドス"),
    location(
        "ミレニアム・スタディーエリア",
        "ミレニアムタワー",
        "ミレニアム",
    ),
    location("トリニティ・スクエア", "トリニティ大聖堂", "トリニティ"),
    location(
        "レッドウィンター連邦学園",
        "レッドウィンター事務局",
        "レッドウィンター",
    ),
    location("百鬼夜行中心部", "百鬼夜行陰陽部本館", "百鬼夜行"),
    shared("D.U.シラトリ区", "ヴァルキューレ警察学校シラトリ支所"),
    location("山海経中央特区", "六和閣", "山海経"),
    shared("春葉原", "春葉原駅"),
    location(
        "ワイルドハント総合芸術地区",
        "ワイルドハント本館",
        "ワイルドハント",
    ),
];

/// The schools the schedule's locations give the goods of, in their order.
pub fn schools() -> impl Iterator<Item = &'static str> {
    LOCATIONS.iter().filter_map(|location| location.school)
}

/// Where a chosen location's rank and name are written, in units (see
/// `screen::unit`), from the right edge, kept to the top: inside the panel,
/// short of its dark edge, and above the bar under them ("MAX" at a location
/// of the highest rank). Measured on 2026-11-23 at 2370x1333 over the twelve
/// locations: the panel's edge ends at 0.504 and starts again at 0.035 from the
/// right; the rank's box from 0.486 to 0.371, 0.141 to 0.176 down; the names
/// from 0.363 at the nearest to 0.069 at the longest, 0.142 to 0.171 down (the
/// long ones in smaller letters); "MAX" from 0.182 down.
pub const NAME_AREA: Area = Area {
    from_right: true,
    x: (0.499, 0.0375),
    y: (0.1335, 0.1785),
    anchor: Anchor::Top,
};
/// The rank's box before the name: the first run of dark columns, at least
/// `BOX` wide, `BOX_TALL` tall and `BOX_FILL` dark (its letters white). Measured
/// so: "RANK12" 0.1155 wide, 0.036 tall and 0.74 dark, a digit about 0.019
/// wide, so a rank of one digit about 0.097; no letter of a name is wider than
/// 0.030. The name starts 0.008 to 0.016 after the box.
const BOX: f32 = 0.06;
const BOX_TALL: f32 = 0.025;
const BOX_FILL: f32 = 0.6;

/// Where the name of the first room is written on its card, in units, kept to
/// the middle (the dialog lies in the middle of the screen): right of the
/// level's circle, inside the card's white head. Measured on 2026-11-23 at
/// 2370x1333 over the twelve dialogs: the head white from 0.180 to 0.645 across
/// and 0.252 to 0.345 down, the circle's edge ending at 0.287; the names from
/// 0.301 to 0.614 across at the longest, a name of one line 0.284 to 0.311
/// down, one of two lines (D.U.シラトリ区's) 0.268 to 0.295 and 0.299 to 0.326.
pub const ROOM_AREA: Area = Area {
    from_right: false,
    x: (0.2941, 0.6392),
    y: (0.2573, 0.3391),
    anchor: Anchor::Middle,
};
/// A line of a room's name is at least this tall (a line 0.028, the gap
/// between two 0.004).
const LINE: f32 = 0.012;

/// Room kept beside a name's letters, short of the box before it.
const ROOM: f32 = 0.005;
/// Parts of the letters smaller than this share of the unit squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// A name's letters lie within this of the word's on the mean, as the
/// campaign's text is told (see `campaign`): the letters are cut at even
/// widths, and as the width changes a cut may take the edge of the letter
/// beside it. Measured on 2026-11-23 over the twenty-four screens drawn from
/// 1500 to 2370 wide by 10: a location's name within 18.7 of its own word on
/// the mean (one letter up to 75.9: ゲヘナ学園・中央区's, where a cut takes the
/// "・" or leaves it) and 44.4 or farther from another location's; a room's
/// within 19.4 of its own and 46.4 or farther from another's. Over 804
/// captures of other screens kept outside the repository, what the card's
/// area shows came no nearer a room than 50.4 (the 特別依頼 screen), and
/// nothing after a box at the top right was as wide for its height as a
/// location's name.
const MEAN: f32 = 30.0;

/// The dark letters on the light panel and the white card.
fn dark(c: [u8; 3]) -> bool {
    student::luminance(c) < 128.0
}

/// Which pixels of a rectangle are dark, row by row.
fn rows(image: &RgbImage, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> Vec<Vec<bool>> {
    (y0..y1)
        .map(|y| (x0..x1).map(|x| dark(image.get_pixel(x, y).0)).collect())
        .collect()
}

/// The columns a location's name spans across `rows` (dark or not, each left
/// to right), after the rank's box: the first run of dark columns, when it is
/// at least `wide`, `tall` and `BOX_FILL` dark. The box is not looked for at a
/// fixed place, as a rank of one digit draws it narrower and the name nearer
/// (not seen: the user's locations are all of rank 12). `None` without the
/// box first or without letters after it.
pub fn after_box(rows: &[Vec<bool>], wide: usize, tall: usize) -> Option<(usize, usize)> {
    let width = rows.first()?.len();
    let runs = word::runs((0..width).map(|x| rows.iter().any(|row| row[x])));
    let &(first, last) = runs.first()?;
    let on = |row: &Vec<bool>| row[first..=last].iter().filter(|&&on| on).count();
    let dark: Vec<usize> = rows.iter().map(on).collect();
    let top = dark.iter().position(|&n| n > 0)?;
    let height = dark.iter().rposition(|&n| n > 0)? - top + 1;
    let size = (last - first + 1) * height;
    let filled = dark.iter().sum::<usize>() as f32 >= BOX_FILL * size as f32;
    if last - first + 1 < wide || height < tall || !filled {
        return None;
    }
    Some((runs.get(1)?.0, runs[runs.len() - 1].1))
}

/// The rows the first line of a text spans across `rows`: the first run of
/// dark rows at least `least` tall (a speck's rows are no line). A name too
/// long for a line is written on two, and is told by its first.
pub fn first_line(rows: &[Vec<bool>], least: usize) -> Option<(usize, usize)> {
    word::runs(rows.iter().map(|row| row.contains(&true)))
        .into_iter()
        .find(|&(top, bottom)| bottom - top + 1 >= least)
}

/// The letters of the chosen location's name, the rank's box before them left
/// out; `None` where the top right shows no box with letters after it.
pub fn name_ink(image: &RgbImage) -> Option<Ink> {
    let u = screen::unit(image.width(), image.height());
    let (x0, y0, x1, y1) = student::rect(image, NAME_AREA)?;
    let px = |units: f32| (units * u).round().max(1.0) as usize;
    let (first, last) = after_box(&rows(image, (x0, y0, x1, y1)), px(BOX), px(BOX_TALL))?;
    // Room for the letters short of the box and the area's end: what touches
    // them is the ground around.
    let room = (ROOM * u).round() as u32;
    let left = (x0 + first as u32).saturating_sub(room).max(x0);
    let right = (x0 + last as u32 + 1 + room).min(x1);
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, (left, y0, right, y1), dark, speck);
    ink.bounds().map(|_| ink)
}

/// The letters of the first line of the first room's name on its card; `None`
/// where the area shows no line of letters clear of its edges.
pub fn room_ink(image: &RgbImage) -> Option<Ink> {
    let u = screen::unit(image.width(), image.height());
    let (x0, y0, x1, y1) = student::rect(image, ROOM_AREA)?;
    let least = (LINE * u).round().max(1.0) as usize;
    let (top, bottom) = first_line(&rows(image, (x0, y0, x1, y1)), least)?;
    // A row of ground above and below the line: what touches the edges is
    // left out, as the second line would be.
    let (top, bottom) = (y0 + top as u32, y0 + bottom as u32 + 1);
    if top == y0 || bottom == y1 {
        return None;
    }
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, (x0, top - 1, x1, bottom + 1), dark, speck);
    ink.bounds().map(|_| ink)
}

/// The location among `LOCATIONS` that `ink` is the word of among `words`
/// (the nearest, within `MEAN`), each word's id its place there.
fn told<'a>(
    ink: &Ink,
    words: impl IntoIterator<Item = &'a Word>,
    glyphs: &Bank,
) -> Option<&'static Location> {
    let none = std::collections::BTreeMap::new();
    let named = word::nearest(ink, words, &glyphs.tiles, &none, f32::INFINITY)?;
    let (mean, _) = word::compare(ink, named, &glyphs.tiles, &none)?;
    LOCATIONS
        .get(usize::try_from(named.id).ok()?)
        .filter(|_| mean <= MEAN)
}

/// The location the chosen screen names at its top right. Call it where the
/// title reads `CHOSEN`: the list of locations writes names there as well.
pub fn named(image: &RgbImage, glyphs: &Bank) -> Option<&'static Location> {
    told(&name_ink(image)?, &glyphs.index.locations, glyphs)
}

/// The location whose first room the dialog's top left card names.
pub fn first_room(image: &RgbImage, glyphs: &Bank) -> Option<&'static Location> {
    told(&room_ink(image)?, &glyphs.index.rooms, glyphs)
}

#[cfg(test)]
mod tests {
    use super::{after_box, first_line};

    fn grid(rows: &[&str]) -> Vec<Vec<bool>> {
        rows.iter()
            .map(|row| row.chars().map(|c| c == '#').collect())
            .collect()
    }

    #[test]
    fn a_location_s_name_is_what_follows_the_rank_s_box() {
        // The box (dark, its white letters holes in it), a gap, then the letters.
        let two_digits = grid(&[
            ".########..#..##.#.",
            ".#.##.#.#..#..#..#.",
            ".########..##.##.#.",
            ".########..#...#.#.",
        ]);
        assert_eq!(after_box(&two_digits, 5, 4), Some((11, 17)));
        // A rank of one digit: the box narrower, the name as far after it.
        let one_digit = grid(&[
            ".######..#..##.#.",
            ".#.##.#..#..#..#.",
            ".######..##.##.#.",
            ".######..#...#.#.",
        ]);
        assert_eq!(after_box(&one_digit, 5, 4), Some((9, 15)));
    }

    #[test]
    fn no_name_is_told_without_the_box_before_it_or_after_it_alone() {
        // Letters alone, the first no box: too narrow, too low, or too light.
        let narrow = grid(&["..##..#..#.", "..##..####.", "..##..#..#."]);
        assert_eq!(after_box(&narrow, 5, 3), None);
        let low = grid(&["...........", ".#####..#..", "........#.."]);
        assert_eq!(after_box(&low, 5, 3), None);
        let light = grid(&[".#...#..#..", ".#...#..#..", ".#...#..#..", ".#####..#.."]);
        assert_eq!(after_box(&light, 5, 3), None);
        // The box alone, and nothing dark.
        assert_eq!(
            after_box(&grid(&[".#####...", ".#####...", ".#####..."]), 5, 3),
            None
        );
        assert_eq!(after_box(&grid(&["....", "...."]), 2, 2), None);
    }

    #[test]
    fn a_room_s_name_of_two_lines_is_read_by_its_first() {
        let two = grid(&[
            ".....", ".#.#.", ".###.", ".....", ".##..", ".#.#.", ".....",
        ]);
        assert_eq!(first_line(&two, 2), Some((1, 2)));
        let one = grid(&[".....", ".#.#.", ".###.", ".#.#.", "....."]);
        assert_eq!(first_line(&one, 2), Some((1, 3)));
        // A speck's row above the letters is no line.
        let speck = grid(&["..#..", ".....", ".#.#.", ".###.", "....."]);
        assert_eq!(first_line(&speck, 2), Some((2, 3)));
        assert_eq!(first_line(&grid(&["....", "...."]), 2), None);
    }
}
