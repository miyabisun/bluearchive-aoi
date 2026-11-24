//! Reads the student list (生徒リスト) while it is sorted by the order of
//! acceptance (受け入れ順) and shows every student (the All tab, no filter, no
//! search): which student stands at each place, so `accepted` can number the
//! students in the order they were accepted (the user's 2026-11-24 「現在のように
//! 「All」「受け入れ順」になっているときに生徒の受け入れ順番を生徒のindex値として
//! 保存出来るように」).
//!
//! The list is a grid of cards, six a row, each student's name under the card.
//! The cards keep their size and place at any 16:9 window size, so they are
//! looked for where the grid has them, measured in window heights, and the
//! frame is read only where what it shows agrees with that grid: the scroll
//! bar (or the ground) right of the sixth column, a card's frame around each
//! picture. A card is a student held (a white frame), one not held yet (the
//! game lists those who can be recruited last, grey all over) or none (the
//! list's ground). The cards not held stand under a banner of their own
//! (未所属) that starts a row anew, so their rows lie off the held cards': the
//! banner tells where both lie. The bar under a card's picture shows the
//! level or the stars, whichever the sort key before left there, and is not
//! read.
//!
//! Filtered, searched, on another tab or sorted by another key, the list's
//! places say nothing of the order of acceptance: the header's buttons tell
//! those states (see `header`), and nothing is read in them.

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Mutex, PoisonError},
};

use image::RgbImage;

use crate::{
    glyph::{
        Rect,
        bank::{Tile, Word},
        word::{self, Ink, runs},
    },
    screen::{self, Anchor, Region},
    student::luminance,
};

/// The cards of a row.
pub const COLUMNS: usize = 6;

/// What stands at a place of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// A student held; who, where the name under the card told.
    Held(Option<i64>),
    /// A student not held: a grey card.
    Lacked,
    /// No card: past the list's end.
    Empty,
}

/// A stretch of the list, as a frame shows it or as frames laid over one
/// another do (see `accepted::Steady`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sight {
    /// Whether the oldest student comes first (≡↑); the newest does by default.
    pub ascending: bool,
    /// The places in view in the list's order, row after row; `None` where the
    /// frame does not tell what stands there (a card cut by the view's edge
    /// may still tell its kind).
    pub cells: Vec<Option<Cell>>,
    /// Which of `cells` is the list's first place, where the list's start is
    /// in view.
    pub first: Option<usize>,
}

/// A card's picture in brief: the mean colour of each of `BANDS` bands down
/// its middle, `None` for a band not whole in view. The same card shows the
/// same face in every frame, wherever the list was scrolled to, so frames
/// that name no student in common are still laid over one another by it.
pub type Face = [Option<[u8; 3]>; BANDS];
pub const BANDS: usize = 16;

/// What a frame of the list shows: its stretch, and the face of the card at
/// each of its places (no band of it for a place without a card, or untold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub sight: Sight,
    pub faces: Vec<Face>,
}

/// How far apart two faces are over the bands both show: the sum of their
/// colours' differences (a channel's mean) and how many bands were compared.
pub fn apart(a: &Face, b: &Face) -> (u32, u32) {
    let (mut sum, mut bands) = (0, 0);
    for (a, b) in a.iter().zip(b) {
        if let (Some(a), Some(b)) = (a, b) {
            let off: u32 = a
                .iter()
                .zip(b)
                .map(|(a, b)| u32::from(a.abs_diff(*b)))
                .sum();
            sum += off / 3;
            bands += 1;
        }
    }
    (sum, bands)
}

// Everything below is measured in units (see `screen::unit`) on 2026-11-24 at
// 2382x1340, from the left edge and the top unless told otherwise.

/// The first column's picture starts here; the columns stand `ACROSS` apart.
const PICTURE_LEFT: f32 = 99.0 / 1340.0;
const ACROSS: f32 = 365.4 / 1340.0;
/// The rows stand this far apart.
const DOWN: f32 = 499.0 / 1340.0;
/// A card's picture, across and down.
const PICTURE: (f32, f32) = (325.0 / 1340.0, 367.0 / 1340.0);
/// The frame around a picture, on every side.
const FRAME: f32 = 10.0 / 1340.0;
/// A grey card's frame is no lighter than its picture: such a card stands out
/// from the ground this tall, its picture's top `FRAME` down.
const GREY_TALL: f32 = 389.0 / 1340.0;
/// The rows of the view whose colours are the cards' own: the view (from 0.241
/// to 0.956) fades the cards at its edges, 20 pixels at the top and 24 at the
/// bottom at 1340 high.
const VIEW: (f32, f32) = (345.0 / 1340.0, 1255.0 / 1340.0);
/// On a screen shorter than 16:9 the view ends this far above the bottom
/// (unmeasured: taken as kept to the bottom, where it ends at 16:9).
const VIEW_ABOVE_BOTTOM: f32 = 85.0 / 1340.0;
/// The first row's pictures' tops while the list rests at its start (pulled
/// past it, the list springs back: 41 pixels lower on a screen caught so).
const START: f32 = 372.0 / 1340.0;
/// A name's letters lie in these rows under its picture's top, with room
/// around them, clear of the card's shadow above and the next row's pins
/// below.
const NAME: (f32, f32) = (382.0 / 1340.0, 447.0 / 1340.0);
/// A name lies within its card's width and this much more on either side: the
/// cards stand 0.0157 apart.
const NAME_ASIDE: f32 = 7.0 / 1340.0;

/// The banner 未所属 that heads the cards not held: this tall, its top this
/// far below the pictures' tops of the held cards' last row, and the pictures'
/// tops of the first row under it this far below its own.
const BANNER_TALL: f32 = 62.0 / 1340.0;
const BANNER_BELOW: f32 = 506.5 / 1340.0;
const BANNER_ABOVE: f32 = 93.0 / 1340.0;
/// How far the banner's height found may lie from `BANNER_TALL`.
const BANNER_OFF: f32 = 0.004;

/// No screen shows as many rows of cards as this (three at 16:9).
const ROWS_MOST: u8 = 8;

/// Less of a card in view than this tells nothing of it.
const LEAST: f32 = 24.0 / 1340.0;
/// How far the pictures' tops found in the columns may lie from one another,
/// in units and at least in pixels.
const TOGETHER: (f32, f32) = (0.0025, 2.0);
/// How far a picture's height found may lie from `PICTURE.1` (or `GREY_TALL`).
const TALL_OFF: f32 = 0.0045;

/// The list's ground (243, 252, 253) and the dashes ruled on it (228, 238,
/// 239), less red than a held card's white frame: a screen scaled down
/// sharply rings a few levels around the dashes.
fn ground(c: [u8; 3]) -> bool {
    (218..=251).contains(&c[0]) && c[1] >= 228 && c[2] >= 228
}

/// A held card's white frame, as far as a screen scaled down keeps it white
/// (three pixels of it at 1500 wide; beside them it shades to 249 and 230).
fn pale_frame(c: [u8; 3]) -> bool {
    c.iter().all(|&v| v >= 240)
}

/// A grey card's frame (126, 126, 126).
fn grey(c: [u8; 3]) -> bool {
    let low = c[0].min(c[1]).min(c[2]);
    let high = c[0].max(c[1]).max(c[2]);
    low >= 100 && high <= 152 && high - low <= 18
}

/// The banner's navy (45, 76, 114).
fn banner_navy(c: [u8; 3]) -> bool {
    c[0].abs_diff(45) <= 16 && c[1].abs_diff(76) <= 16 && c[2].abs_diff(114) <= 16
}

/// The ground or a held card's frame: what a card's picture stands out from.
fn light(c: [u8; 3]) -> bool {
    c.iter().all(|&v| v > 215)
}

/// The names' dark letters (101 bright on the ground's 249; the yellow line
/// under a name is 235).
fn dark(c: [u8; 3]) -> bool {
    luminance(c) < 175.0
}

/// Where the grid lies in a frame: its unit, and the rows of the view whose
/// colours are the cards' own.
#[derive(Debug, Clone, Copy)]
struct Grid {
    unit: f32,
    view: (u32, u32),
    width: u32,
    height: u32,
}

impl Grid {
    fn of(image: &RgbImage) -> Self {
        let (width, height) = image.dimensions();
        let unit = screen::unit(width, height);
        let bottom = (VIEW.1 * unit).min(height as f32 - VIEW_ABOVE_BOTTOM * unit);
        Self {
            unit,
            view: ((VIEW.0 * unit).ceil() as u32, bottom.max(0.0) as u32),
            width,
            height,
        }
    }

    /// The left edge of the picture of column `column`.
    fn left(self, column: usize) -> f32 {
        (PICTURE_LEFT + ACROSS * column as f32) * self.unit
    }

    /// The pixels from `x.0` to `x.1` units right of `left` and from `y.0` to
    /// `y.1` below `top`, the rows kept to the view: `None` where nothing of
    /// them is in the frame.
    fn part(self, (left, top): (f32, f32), x: (f32, f32), y: (f32, f32)) -> Option<Rect> {
        let (x0, x1) = (left + x.0 * self.unit, left + x.1 * self.unit);
        let (y0, y1) = (top + y.0 * self.unit, top + y.1 * self.unit);
        let x0 = x0.round().max(0.0) as u32;
        let x1 = (x1.round().max(0.0) as u32).min(self.width);
        let y0 = (y0.round().max(0.0) as u32).max(self.view.0);
        let y1 = (y1.round().max(0.0) as u32).min(self.view.1);
        (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
    }
}

/// The share of `rect`'s pixels that are `of`.
fn share(image: &RgbImage, (x0, y0, x1, y1): Rect, of: fn([u8; 3]) -> bool) -> f32 {
    let count = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| of(image.get_pixel(x, y).0))
        .count();
    count as f32 / ((x1 - x0) * (y1 - y0)) as f32
}

/// The tops of the pictures whole in view (pixels), found column by column: a
/// picture stands out from the ground and its frame `PICTURE.1` tall, a grey
/// card with its frame `GREY_TALL`.
fn pictures(image: &RgbImage, grid: Grid) -> Vec<f32> {
    let mut tops: Vec<f32> = Vec::new();
    for column in 0..COLUMNS {
        let middle = grid.left(column) + PICTURE.0 * grid.unit / 2.0;
        let x0 = (middle - 0.06 * grid.unit).max(0.0) as u32;
        let x1 = ((middle + 0.06 * grid.unit) as u32).min(grid.width);
        if x0 >= x1 {
            continue;
        }
        let stands_out = (grid.view.0..grid.view.1).map(|y| {
            let lit = (x0..x1).filter(|&x| light(image.get_pixel(x, y).0)).count();
            (lit as f32) < 0.9 * (x1 - x0) as f32
        });
        let rows = grid.view.1.saturating_sub(grid.view.0) as usize;
        for (start, end) in runs(stands_out) {
            // Cut by the view's edge, its height tells nothing.
            if start == 0 || end + 1 >= rows {
                continue;
            }
            let tall = (end - start + 1) as f32 / grid.unit;
            let top = if (tall - PICTURE.1).abs() <= TALL_OFF {
                start as f32
            } else if (tall - GREY_TALL).abs() <= TALL_OFF {
                start as f32 + FRAME * grid.unit
            } else {
                continue;
            };
            tops.push(top + grid.view.0 as f32);
        }
    }
    tops
}

/// The top of the banner 未所属 (pixels), where it is whole in view: rows of
/// its navy between the columns of cards (the cards' frames and the ground
/// are light there; its letters stand between the two middle columns, which
/// are left out), `BANNER_TALL` of them.
fn banner(image: &RgbImage, grid: Grid) -> Option<f32> {
    let between: Vec<(u32, u32)> = [0, 1, 3, 4]
        .into_iter()
        .filter_map(|column| {
            let x0 = grid.left(column) + (PICTURE.0 + 0.003) * grid.unit;
            let x1 = grid.left(column + 1) - 0.003 * grid.unit;
            let (x0, x1) = (x0.round() as u32, (x1.round() as u32).min(grid.width));
            (x0 < x1).then_some((x0, x1))
        })
        .collect();
    if between.len() < 4 {
        return None;
    }
    let navy = (grid.view.0..grid.view.1).map(|y| {
        between
            .iter()
            .all(|&(x0, x1)| (x0..x1).all(|x| banner_navy(image.get_pixel(x, y).0)))
    });
    let rows = grid.view.1.saturating_sub(grid.view.0) as usize;
    runs(navy)
        .into_iter()
        .find(|&(start, end)| {
            let tall = (end - start + 1) as f32 / grid.unit;
            start > 0 && end + 1 < rows && (tall - BANNER_TALL).abs() <= BANNER_OFF
        })
        .map(|(start, _)| (start + grid.view.0 as usize) as f32)
}

/// Where the rows in view have their pictures' tops (pixels, above the frame
/// for a row cut at the top), the top row first: the held cards' rows, and
/// under the banner the rows of the cards not held, with which of the rows
/// is the first under the banner where it is in view. Found from the banner,
/// or else from the pictures whole in view; `None` where neither shows, or
/// the pictures stand elsewhere than the rows found.
fn row_tops(image: &RgbImage, grid: Grid) -> Option<(Vec<f32>, Option<usize>)> {
    let pitch = DOWN * grid.unit;
    let pictures = pictures(image, grid);
    let within = (TOGETHER.0 * grid.unit).max(TOGETHER.1);
    // How far `top` lies from the rows through `row`.
    let off = |top: f32, row: f32| (top - row - ((top - row) / pitch).round() * pitch).abs();
    // The first row any of whose cards is in view enough to tell its kind.
    let least = grid.view.0 as f32 + LEAST * grid.unit - (PICTURE.1 + FRAME) * grid.unit;
    let in_view = |top: f32| top + (LEAST - FRAME) * grid.unit < grid.view.1 as f32;
    if let Some(banner) = banner(image, grid) {
        let last_held = banner - BANNER_BELOW * grid.unit;
        let first_other = banner + BANNER_ABOVE * grid.unit;
        let placed = pictures.iter().all(|&top| {
            let row = if top < banner { last_held } else { first_other };
            off(top, row) <= within
        });
        if !placed {
            return None;
        }
        let mut tops: Vec<f32> = (0..ROWS_MOST)
            .map(|up| last_held - f32::from(up) * pitch)
            .take_while(|&top| top > least)
            .collect();
        tops.reverse();
        // The row under the banner counts even where too little of it shows.
        let under = tops.len();
        tops.push(first_other);
        tops.extend(
            (1..ROWS_MOST)
                .map(|down| first_other + f32::from(down) * pitch)
                .take_while(|&top| in_view(top)),
        );
        return Some((tops, Some(under)));
    }
    let &any = pictures.first()?;
    if pictures.iter().any(|&top| off(top, any) > within) {
        return None;
    }
    // Each beside `any`, however many rows off it lies.
    let mean = pictures
        .iter()
        .map(|&top| top - ((top - any) / pitch).round() * pitch)
        .sum::<f32>()
        / pictures.len() as f32;
    let first = mean - ((mean - least) / pitch).floor() * pitch;
    let tops = (0..ROWS_MOST)
        .map(|down| first + f32::from(down) * pitch)
        .take_while(|&top| in_view(top))
        .collect();
    Some((tops, None))
}

/// What a card's frame, left or right of its picture, is over the rows in view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// A held card's frame (or the ground, as light: a held card is told from
    /// no card by its picture).
    Pale,
    Grey,
    /// Not enough of it in view.
    Hidden,
    Other,
}

fn side(image: &RgbImage, grid: Grid, rect: Option<Rect>) -> Side {
    let Some(rect) = rect.filter(|r| (r.3 - r.1) as f32 >= LEAST * grid.unit) else {
        return Side::Hidden;
    };
    if share(image, rect, pale_frame) >= 0.9 {
        Side::Pale
    } else if share(image, rect, grey) >= 0.9 {
        Side::Grey
    } else {
        Side::Other
    }
}

/// The frame's sides are looked at in the middle of the frame, this far from
/// the picture's edge: the frame is 0.0067 wide, and a screen scaled down
/// blends its edges with the picture and the line around it. Measured on
/// 2026-11-24 at 1500 wide (the frame 5.5 pixels): white over 3 pixels.
const SIDE: (f32, f32) = (0.0019, 0.0041);

/// What kind of card stands in `column` of the row whose pictures' tops are at
/// `top`: held (a picture in a pale frame), not held (in a grey one), none
/// (the ground all over the card's place); `None` where too little of it is
/// in view, or it is none of these for sure. The pin at a held card's top
/// left covers its frame's left side there, and the card's bottom right corner
/// is cut off: the left side is looked at below the pin, the right side above
/// the corner.
fn kind(image: &RgbImage, grid: Grid, top: f32, column: usize) -> Option<Cell> {
    let at = (grid.left(column), top);
    let (wide, tall) = PICTURE;
    let sides = [
        grid.part(at, (-SIDE.1, -SIDE.0), (0.068, tall - 0.008)),
        grid.part(at, (wide + SIDE.0, wide + SIDE.1), (0.008, tall - 0.034)),
    ]
    .map(|rect| side(image, grid, rect));
    let picture = grid
        .part(at, (0.02, wide - 0.02), (0.008, tall - 0.008))
        .filter(|r| (r.3 - r.1) as f32 >= LEAST * grid.unit)?;
    let grounded = share(image, picture, ground);
    // Both sides, or the one in view.
    let all = |wanted: Side| {
        sides.contains(&wanted) && sides.iter().all(|&s| s == wanted || s == Side::Hidden)
    };
    if grounded >= EMPTY {
        // The whole of the card's place in view is the ground.
        let place = grid.part(at, (-FRAME, wide + FRAME), (-FRAME, tall + FRAME))?;
        (share(image, place, ground) >= EMPTY).then_some(Cell::Empty)
    } else if grounded >= PICTURED {
        None
    } else if all(Side::Pale) {
        Some(Cell::Held(None))
    } else if all(Side::Grey) {
        Some(Cell::Lacked)
    } else {
        None
    }
}

/// A place with no card is the ground over this share of it at least (the
/// dashes ruled on the ground count as it); a card's picture over less than
/// `PICTURED` of its middle.
const EMPTY: f32 = 0.985;
const PICTURED: f32 = 0.5;

/// The face of the card in `column` of the row whose pictures' tops are at
/// `top`: its picture's middle half across, band by band.
fn face(image: &RgbImage, grid: Grid, top: f32, column: usize) -> Face {
    let (wide, tall) = PICTURE;
    let left = grid.left(column);
    let x0 = (left + 0.25 * wide * grid.unit).max(0.0) as u32;
    let x1 = ((left + 0.75 * wide * grid.unit) as u32).min(grid.width);
    std::array::from_fn(|band| {
        let y0 = top + tall * grid.unit * band as f32 / BANDS as f32;
        let y1 = top + tall * grid.unit * (band + 1) as f32 / BANDS as f32;
        if x0 >= x1 || y0 < grid.view.0 as f32 || y1 > grid.view.1 as f32 {
            return None;
        }
        let (y0, y1) = (
            y0.round() as u32,
            (y1.round() as u32).max(y0.round() as u32 + 1),
        );
        let mut sum = [0u32; 3];
        for (x, y) in (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))) {
            let pixel = image.get_pixel(x, y).0;
            for channel in 0..3 {
                sum[channel] += u32::from(pixel[channel]);
            }
        }
        let count = (x1 - x0) * (y1 - y0);
        Some(sum.map(|s| (s / count) as u8))
    })
}

/// Parts of the letters smaller than this share of the unit squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);

/// The letters of the name under the card in `column` of the row whose
/// pictures' tops are at `top`; `None` where the name is not whole in view.
fn name_ink(image: &RgbImage, grid: Grid, top: f32, column: usize) -> Option<Ink> {
    let left = grid.left(column);
    let x0 = left - (FRAME + NAME_ASIDE) * grid.unit;
    let x1 = left + (PICTURE.0 + FRAME + NAME_ASIDE) * grid.unit;
    let (y0, y1) = (top + NAME.0 * grid.unit, top + NAME.1 * grid.unit);
    if x0 < 0.0 || x1 > grid.width as f32 || y0 < grid.view.0 as f32 || y1 > grid.view.1 as f32 {
        return None;
    }
    let least = (SPECK * grid.unit * grid.unit).round().max(1.0) as usize;
    let rect = (x0 as u32, y0 as u32, x1 as u32, y1 as u32);
    Some(Ink::parts(image, rect, dark, least))
}

/// The farthest a letter of a name under a card may lie from the name's own.
pub const WORST: f32 = 17.0;

/// One row's places as the frame shows them: its pictures' tops, what kind of
/// card each column holds, and the letters under it where they are whole in
/// view.
pub struct Row {
    pub top: f32,
    pub cells: [Option<Cell>; COLUMNS],
    pub names: [Option<Ink>; COLUMNS],
    /// Each card's face; no band of it where no card is told.
    pub faces: [Face; COLUMNS],
}

/// The rows in view, the top one first, each card's kind told and its name's
/// letters cut out (no student told yet); `None` where the frame shows no row
/// of cards where the grid has them. Under the banner 未所属 the first place
/// is a card not held, in view or not: the banner heads such cards.
pub fn rows(image: &RgbImage) -> Option<Vec<Row>> {
    let grid = Grid::of(image);
    let (tops, under_banner) = row_tops(image, grid)?;
    let rows: Vec<Row> = tops
        .iter()
        .enumerate()
        .map(|(row, &top)| {
            let mut cells: [Option<Cell>; COLUMNS] =
                std::array::from_fn(|column| kind(image, grid, top, column));
            if under_banner == Some(row) && cells[0].is_none() {
                cells[0] = Some(Cell::Lacked);
            }
            Row {
                top,
                cells,
                names: std::array::from_fn(|column| name_ink(image, grid, top, column)),
                faces: std::array::from_fn(|column| match cells[column] {
                    Some(Cell::Held(_) | Cell::Lacked) => face(image, grid, top, column),
                    _ => [None; BANDS],
                }),
            }
        })
        .collect();
    (!rows.is_empty()).then_some(rows)
}

/// What the names under the cards are told with, as the student screen's
/// plate is (see `name`): the students' names as words, a glyph bank's tiles,
/// and the letters of the names as the game draws them on the plate. Under a
/// card the same letters are darker, smaller and spaced otherwise, so they
/// are cut at the gaps between them (`Ink::letters_at_gaps`). Measured on
/// 2026-11-24 over the list walked both ways (266 students): cut where the
/// words' bounds lie, 38 of 544 names were not told (a letter's cut took a
/// sliver of its neighbour: "（" 84 from its references); cut at the gaps,
/// none, and none told wrong either way.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    pub words: &'a [Word],
    pub tiles: &'a [Tile],
    pub letters: &'a BTreeMap<char, Vec<usize>>,
    /// The names told with these before (see `Told`).
    pub told: &'a Mutex<Told>,
}

/// The student whose name `ink` writes.
pub fn named(ink: &Ink, names: Names) -> Option<i64> {
    let cut = Ink::letters_at_gaps;
    word::nearest_cut(ink, names.words, names.tiles, names.letters, WORST, cut).map(|w| w.id)
}

/// The names told lately, each by its letters' ink (as one number, see
/// `Told::key`), the oldest first out: a list at rest shows the same names
/// frame after frame, and telling a name among the students' takes most of a
/// frame's reading (measured on 2026-11-24 over the list walked top to bottom,
/// a screen 1920 wide: 46 ms a frame telling every name, 12 or 18 of them).
#[derive(Default)]
pub struct Told {
    by_ink: HashMap<u64, Option<i64>>,
    order: VecDeque<u64>,
}

/// As many names as two lists of students hold.
const TOLD_KEPT: usize = 600;

impl Told {
    /// All of `ink` as one number: two inks of one number are the same
    /// letters (but for a chance of one in 2^64).
    fn key(ink: &Ink) -> u64 {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (ink.width, ink.height).hash(&mut hasher);
        ink.on.hash(&mut hasher);
        hasher.finish()
    }

    /// The student `ink` was told as before, or as `tell` tells it now.
    fn name(&mut self, ink: &Ink, tell: impl FnOnce() -> Option<i64>) -> Option<i64> {
        let key = Self::key(ink);
        if let Some(&told) = self.by_ink.get(&key) {
            return told;
        }
        let told = tell();
        self.by_ink.insert(key, told);
        self.order.push_back(key);
        if self.order.len() > TOLD_KEPT
            && let Some(oldest) = self.order.pop_front()
        {
            self.by_ink.remove(&oldest);
        }
        told
    }
}

/// The scroll bar's track, right of the sixth column, and how far down it is
/// looked for.
const TRACK: (f32, f32) = (2288.0 / 1340.0, 2293.0 / 1340.0);
const TRACK_ROWS: (f32, f32) = (318.0 / 1340.0, 1322.0 / 1340.0);
/// The track is this long at least.
const TRACK_LONG: f32 = 0.6;
/// The thumb at the track's start lies within this of it, in units and at
/// least in pixels. Measured on 2026-11-24 over the list walked top to bottom
/// both ways, the screens drawn 1500 to 2222 wide: at the start the thumb
/// lies 0 or 1 pixel from the track's (0.0012 at most); in the nearest screen
/// without the list's first row it lies 0.0107 away.
const AT_START: (f32, f32) = (0.0022, 2.0);

/// The scroll bar's thumb (146, 166, 182) and its track (204, 214, 223), by
/// their brightness (161 and 212; the ground 249): the thumb's round ends
/// shade into the track, so the two share a bound.
fn thumb(c: [u8; 3]) -> bool {
    (135.0..187.0).contains(&luminance(c))
}

fn track(c: [u8; 3]) -> bool {
    (187.0..=230.0).contains(&luminance(c))
}

/// What stands where the scroll bar's track is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bar {
    /// The bar: how far its thumb's start lies from the track's (pixels).
    Thumb(f32),
    /// The list's ground: no bar drawn.
    None,
}

/// The scroll bar right of the sixth column, or the ground in its place;
/// `None` where something else stands there (the frame is no list laid out six
/// to a row).
pub fn bar(image: &RgbImage) -> Option<Bar> {
    let grid = Grid::of(image);
    let x0 = (TRACK.0 * grid.unit).round() as u32;
    let x1 = ((TRACK.1 * grid.unit).round() as u32).max(x0 + 1);
    let y0 = (TRACK_ROWS.0 * grid.unit) as u32;
    let y1 = ((TRACK_ROWS.1 * grid.unit) as u32).min(grid.height);
    if x1 > grid.width || y0 >= y1 {
        return None;
    }
    let all = |y: u32, of: fn([u8; 3]) -> bool| (x0..x1).all(|x| of(image.get_pixel(x, y).0));
    let thumbs = runs((y0..y1).map(|y| all(y, thumb)));
    let bars = runs((y0..y1).map(|y| all(y, |c| thumb(c) || track(c))));
    match (thumbs.as_slice(), bars.as_slice()) {
        ([], []) => {
            let column = (x0, grid.view.0, x1, grid.view.1);
            (grid.view.0 < grid.view.1 && share(image, column, ground) >= 0.97).then_some(Bar::None)
        }
        (&[(thumb_from, _)], &[(from, to)]) if (to - from) as f32 >= TRACK_LONG * grid.unit => {
            Some(Bar::Thumb((thumb_from - from) as f32))
        }
        _ => None,
    }
}

/// The header's parts, across from the right edge and down from the top: the
/// button left of the tabs (it narrows the list when on), the three tabs, the
/// search button, and the box of the filter and the sort key.
const BUTTON_ROWS: (f32, f32) = (186.0 / 1340.0, 244.0 / 1340.0);
const NARROW: (f32, f32) = (1786.0 / 1340.0, 1706.0 / 1340.0);
const ALL: (f32, f32) = (1682.0 / 1340.0, 1424.0 / 1340.0);
const STRIKER: (f32, f32) = (1397.0 / 1340.0, 1167.0 / 1340.0);
const SPECIAL: (f32, f32) = (1137.0 / 1340.0, 904.0 / 1340.0);
const SEARCH: (f32, f32) = (867.0 / 1340.0, 776.0 / 1340.0);
const SORT_BOX: (f32, f32) = (742.0 / 1340.0, 270.0 / 1340.0);
const SORT_BOX_ROWS: (f32, f32) = (175.0 / 1340.0, 253.0 / 1340.0);

/// The sort key's name and the direction button, in the cards' heights of the
/// student screen (`screen::STUDENT_UNIT`), as `screen::SORT_BUTTONS` is in the
/// lists' cards': compared with `data/student-sort-newest.png` and
/// `data/student-sort-oldest.png` (see `examples/make_templates.rs`).
pub const SORT: Region = Region {
    from_right: true,
    x: (3.86, 0.34),
    y: (1.0, 1.42),
    anchor: Anchor::Top,
};
/// The direction's arrow starts at this share of the templates' width; the
/// sort key's name and the button's three bars lie before it.
const DIRECTION_FROM: f32 = 0.875;
/// Least likeness of the sort key's name and of the arrow to a template's,
/// and the arrow's least lead over the other direction's.
const SORT_MATCH: f32 = 0.85;
const ARROW_LEAD: f32 = 0.1;

/// A selected tab's navy (22, 56, 87), the narrowing button's when on.
fn navy(c: [u8; 3]) -> bool {
    c[0] < 70 && c[1] < 100 && c[2] < 140 && c[2] > c[0]
}

/// A button's ground (244, 245, 246) or a tab's not selected (231, 242, 246).
fn pale(c: [u8; 3]) -> bool {
    c.iter().all(|&v| v >= 215)
}

/// The blue of the search button and of the filter's funnel when on
/// (22, 134, 246), and of the SPECIAL tab selected.
fn blue(c: [u8; 3]) -> bool {
    c[2] >= 200 && c[0] <= 110
}

/// The pictures of the sort key's name beside each direction button (see `SORT`).
pub struct Sorts {
    pub newest: RgbImage,
    pub oldest: RgbImage,
}

/// What the header's parts measure: how much of the button left of the tabs,
/// of each tab and of the search button is of the colour that tells it off,
/// chosen or not chosen; how much blue the sort key's box holds; and how alike
/// the sort key's name and the direction's arrow are to each template (see
/// `screen::alike`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header {
    pub narrow_pale: f32,
    pub all_navy: f32,
    pub striker_pale: f32,
    pub special_pale: f32,
    pub search_pale: f32,
    pub filter_blue: f32,
    pub newest: (f32, f32),
    pub oldest: (f32, f32),
}

/// A button off, or a tab not chosen, is pale over this share of it at least
/// (its picture or letters the rest); the chosen tab navy over `CHOSEN`; and
/// with no filter on, the sort key's box holds no more blue than `NO_FILTER`.
/// Measured on 2026-11-24 over the screens of each state drawn 1500 to 2222
/// wide (see `header_tells_the_whole_list_in_the_order_of_acceptance`).
const OFF: f32 = 0.5;
const CHOSEN: f32 = 0.6;
const NO_FILTER: f32 = 0.004;

impl Header {
    /// The header's parts as `image` shows them; `None` where the frame has no
    /// room for them.
    pub fn measure(image: &RgbImage, sorts: &Sorts) -> Option<Self> {
        let (width, height) = image.dimensions();
        let unit = screen::unit(width, height);
        let part = |x: (f32, f32), y: (f32, f32)| -> Option<Rect> {
            let (x0, x1) = (width as f32 - x.0 * unit, width as f32 - x.1 * unit);
            let (y0, y1) = (y.0 * unit, y.1 * unit);
            (x0 >= 0.0 && x0 < x1 && y1 <= height as f32).then_some((
                x0.round() as u32,
                y0.round() as u32,
                x1.round() as u32,
                y1.round() as u32,
            ))
        };
        let of = |x: (f32, f32), colour: fn([u8; 3]) -> bool| {
            part(x, BUTTON_ROWS).map(|rect| share(image, rect, colour))
        };
        let card = (unit * screen::STUDENT_UNIT).round() as u32;
        let alike =
            |template: &RgbImage| screen::alike(image, card, SORT, template, DIRECTION_FROM);
        Some(Self {
            narrow_pale: of(NARROW, pale)?,
            all_navy: of(ALL, navy)?,
            striker_pale: of(STRIKER, pale)?,
            special_pale: of(SPECIAL, pale)?,
            search_pale: of(SEARCH, pale)?,
            filter_blue: share(image, part(SORT_BOX, SORT_BOX_ROWS)?, blue),
            newest: alike(&sorts.newest),
            oldest: alike(&sorts.oldest),
        })
    }

    /// Whether the header shows the list whole and in the order of
    /// acceptance, and which way: `Some(ascending)`. Nothing told (`None`)
    /// where the button left of the tabs is on, another tab than All is
    /// chosen, the search or a filter is on (the list then leaves students
    /// out), the sort key is another, or the direction is told for neither
    /// way.
    pub fn told(&self) -> Option<bool> {
        let whole = self.narrow_pale >= OFF
            && self.all_navy >= CHOSEN
            && self.striker_pale >= OFF
            && self.special_pale >= OFF
            && self.search_pale >= OFF
            && self.filter_blue <= NO_FILTER;
        let ascending = self.oldest.1 > self.newest.1;
        let ((name, arrow), (_, other)) = if ascending {
            (self.oldest, self.newest)
        } else {
            (self.newest, self.oldest)
        };
        (whole && name >= SORT_MATCH && arrow >= SORT_MATCH && arrow - other >= ARROW_LEAD)
            .then_some(ascending)
    }
}

/// See `Header::told`.
pub fn header(image: &RgbImage, sorts: &Sorts) -> Option<bool> {
    Header::measure(image, sorts)?.told()
}

/// Reads the frame's list: `rows` with each held card's student told among
/// `names`, which way the list runs (`ascending`, from `header`), whether its
/// start is in view, and the cards' faces. `None` where the frame is not laid
/// out as the list.
pub fn read(image: &RgbImage, ascending: bool, names: Names) -> Option<Seen> {
    let grid = Grid::of(image);
    let bar = bar(image)?;
    let rows = rows(image)?;
    let mut cells = Vec::with_capacity(rows.len() * COLUMNS);
    let faces = rows.iter().flat_map(|row| row.faces).collect();
    let mut told = names.told.lock().unwrap_or_else(PoisonError::into_inner);
    for row in &rows {
        for (cell, ink) in row.cells.iter().zip(&row.names) {
            cells.push(match (cell, ink) {
                (Some(Cell::Held(_)), Some(ink)) => {
                    Some(Cell::Held(told.name(ink, || named(ink, names))))
                }
                (cell, _) => *cell,
            });
        }
    }
    drop(told);
    // The list's first row: the thumb at the track's start, and the row whose
    // pictures' tops are nearest where the first row rests.
    let at_start = (AT_START.0 * grid.unit).max(AT_START.1);
    let first = match bar {
        Bar::Thumb(from) if from <= at_start => {
            let pitch = DOWN * grid.unit;
            rows.iter()
                .position(|row| (row.top - START * grid.unit).abs() < pitch / 2.0)
                .map(|row| row * COLUMNS)
        }
        _ => None,
    };
    let sight = Sight {
        ascending,
        cells,
        first,
    };
    Some(Seen { sight, faces })
}

/// What of a screen the list is read from, for a fixture that keeps no more
/// (see `examples/mask_fixture.rs`): its parts by name, each rectangle left,
/// top, right and bottom. A fixture may keep some parts alone and take the
/// rest from another screen's (the header is the same on every screen of the
/// list in one state).
pub struct Areas {
    /// `title` (the screen's, where it is looked for), `narrow` (the button
    /// left of the tabs), `tabs`, `search`, `sort` (the box of the filter and
    /// the sort key, and where the sort buttons are looked for) and `view`
    /// (the view with a little around it, and the scroll bar).
    pub parts: Vec<(&'static str, Vec<Rect>)>,
    /// Within the view, what is not read: the insides of the cards' pictures,
    /// which only the cards' faces are taken from, and the pins at the held
    /// cards' top left.
    pub unread: Vec<Rect>,
}

/// See `Areas`.
pub fn areas(image: &RgbImage) -> Areas {
    let grid = Grid::of(image);
    let (unit, width) = (grid.unit, grid.width as f32);
    let rect = |x: (f32, f32), y: (f32, f32)| -> Rect {
        (
            x.0.max(0.0) as u32,
            y.0.max(0.0) as u32,
            (x.1.min(width)) as u32,
            (y.1.max(0.0) as u32).min(grid.height),
        )
    };
    // Each button with what a screen scaled down takes in beside it.
    let room = 0.006;
    let button = |x: (f32, f32), y: (f32, f32)| {
        rect(
            (width - (x.0 + room) * unit, width - (x.1 - room) * unit),
            ((y.0 - room) * unit, (y.1 + room) * unit),
        )
    };
    // Where a picture of `region` is looked for in the cards' heights of the
    // student screens: at the sizes and places tried (`sizes` larger or
    // smaller; 0.15 of a card's height across, 0.06 down, see `screen`).
    let card = screen::STUDENT_UNIT * unit;
    let looked_for = |region: Region, sizes: f32| {
        let across = |x: f32, by: f32, slack: f32| {
            let x = (x * by + slack) * card;
            if region.from_right { width - x } else { x }
        };
        let (near, far) = (1.0 - sizes, 1.0 + sizes);
        let x = if region.from_right {
            (
                across(region.x.0, far, 0.15),
                across(region.x.1, near, -0.15),
            )
        } else {
            (
                across(region.x.0, near, -0.15),
                across(region.x.1, far, 0.15),
            )
        };
        rect(
            x,
            (
                (region.y.0 * near - 0.06) * card,
                (region.y.1 * far + 0.06) * card,
            ),
        )
    };
    let aside = (FRAME + NAME_ASIDE + 0.008) * unit;
    let view = vec![
        // The view, and what the reading takes in beside it when a screen is
        // scaled down.
        rect(
            (grid.left(0) - aside, (TRACK.1 + 0.008) * unit),
            ((VIEW.0 - 0.02) * unit, (VIEW.1 + 0.02) * unit),
        ),
        rect(
            ((TRACK.0 - 0.008) * unit, (TRACK.1 + 0.008) * unit),
            ((TRACK_ROWS.0 - 0.008) * unit, (TRACK_ROWS.1 + 0.008) * unit),
        ),
    ];
    // A rim of each picture stays, so the frame beside it is as it was drawn.
    let rim = 0.0015 * unit;
    let mut unread = Vec::new();
    for row in rows(image).unwrap_or_default() {
        for (column, cell) in row.cells.iter().enumerate() {
            let (left, top) = (grid.left(column), row.top);
            if matches!(cell, Some(Cell::Held(_) | Cell::Lacked)) {
                unread.push(rect(
                    (left + rim, left + PICTURE.0 * unit - rim),
                    (top + rim, top + PICTURE.1 * unit - rim),
                ));
            }
            // The pin: over the card's top left corner, clear of the frame's
            // left side where it is looked at (see `kind`).
            if matches!(cell, Some(Cell::Held(_))) {
                unread.push(rect(
                    (left - 0.018 * unit, left + 0.046 * unit),
                    (top - 0.016 * unit, top + 0.048 * unit),
                ));
            }
        }
    }
    // A card cut at the view's top reaches up to the header's rows: only what
    // lies in the view is left out.
    for part in &mut unread {
        (part.1, part.3) = (part.1.max(view[0].1), part.3.min(view[0].3));
    }
    unread.retain(|r| r.0 < r.2 && r.1 < r.3);
    let parts = vec![
        ("title", vec![looked_for(screen::TITLE, 0.12)]),
        ("narrow", vec![button(NARROW, BUTTON_ROWS)]),
        (
            "tabs",
            [ALL, STRIKER, SPECIAL]
                .into_iter()
                .map(|x| button(x, BUTTON_ROWS))
                .collect(),
        ),
        ("search", vec![button(SEARCH, BUTTON_ROWS)]),
        (
            "sort",
            vec![button(SORT_BOX, SORT_BOX_ROWS), looked_for(SORT, 0.06)],
        ),
        ("view", view),
    ];
    Areas { parts, unread }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Letters of one row, `on` pixels of ink.
    fn ink(on: u32) -> Ink {
        Ink {
            width: on + 1,
            height: 1,
            on: (0..=on).map(|x| x < on).collect(),
        }
    }

    #[test]
    fn a_name_told_before_is_not_told_again() {
        let mut told = Told::default();
        assert_eq!(told.name(&ink(3), || Some(7)), Some(7));
        assert_eq!(told.name(&ink(3), || panic!("told anew")), Some(7));
        // What could not be told is kept as well.
        assert_eq!(told.name(&ink(4), || None), None);
        assert_eq!(told.name(&ink(4), || panic!("told anew")), None);
    }

    #[test]
    fn the_names_told_longest_ago_are_let_go() {
        let mut told = Told::default();
        let most = u32::try_from(TOLD_KEPT).unwrap();
        for on in 0..=most {
            told.name(&ink(on), || Some(1));
        }
        assert_eq!(told.by_ink.len(), TOLD_KEPT);
        // The first went as the last came; the one after it is still kept.
        assert_eq!(told.name(&ink(0), || Some(2)), Some(2));
        assert_eq!(told.name(&ink(2), || panic!("told anew")), Some(1));
    }

    /// The header of the list whole and in the order of acceptance, the newest
    /// first, as measured on 2026-11-24 over the screens drawn 1500 to 2222
    /// wide: each part within 0.02 of these but the sort buttons' likeness
    /// (the key's name 0.94 to 0.99, the arrow 0.92 to 0.99, the other
    /// direction's arrow 0.69 to 0.74).
    fn whole() -> Header {
        Header {
            narrow_pale: 0.62,
            all_navy: 0.90,
            striker_pale: 0.71,
            special_pale: 0.73,
            search_pale: 0.84,
            filter_blue: 0.0,
            newest: (0.94, 0.92),
            oldest: (0.93, 0.69),
        }
    }

    #[test]
    fn the_header_tells_the_whole_list_and_which_way_it_runs() {
        assert_eq!(whole().told(), Some(false));
        let oldest_first = Header {
            newest: (0.93, 0.69),
            oldest: (0.94, 0.92),
            ..whole()
        };
        assert_eq!(oldest_first.told(), Some(true));
    }

    /// Each state the list leaves students out in, or is sorted otherwise in,
    /// as measured on its screens (the farthest of each from the whole
    /// list's): none is told.
    #[test]
    fn the_header_tells_nothing_of_a_list_narrowed_or_sorted_otherwise() {
        let others = [
            // The button left of the tabs on (pale 0.20 to 0.23).
            Header {
                narrow_pale: 0.23,
                ..whole()
            },
            // The STRIKER tab (All's navy gone, its own pale 0.19 to 0.22).
            Header {
                all_navy: 0.0,
                striker_pale: 0.22,
                ..whole()
            },
            // The SPECIAL tab (pale 0.15 to 0.18).
            Header {
                all_navy: 0.0,
                special_pale: 0.18,
                ..whole()
            },
            // Searching (the button blue: pale 0.11 to 0.12).
            Header {
                search_pale: 0.12,
                ..whole()
            },
            // A filter on (the funnel and the box's edge blue: 0.084 to 0.103).
            Header {
                filter_blue: 0.084,
                ..whole()
            },
            // Sorted by level (the key's name 0.53 to 0.57 alike).
            Header {
                newest: (0.57, 0.94),
                oldest: (0.57, 0.71),
                ..whole()
            },
            // Neither arrow clearly: both alike, or none alike enough.
            Header {
                newest: (0.94, 0.90),
                oldest: (0.94, 0.86),
                ..whole()
            },
            Header {
                newest: (0.94, 0.80),
                oldest: (0.94, 0.60),
                ..whole()
            },
        ];
        for other in others {
            assert_eq!(other.told(), None, "{other:?}");
        }
    }

    /// Faces are compared over the bands both show, a band's colours by the
    /// mean of their channels' differences.
    #[test]
    fn faces_are_apart_by_the_bands_both_show() {
        let mut one: Face = [None; BANDS];
        let mut other: Face = [None; BANDS];
        (one[0], other[0]) = (Some([10, 20, 30]), Some([13, 20, 24]));
        (one[1], other[1]) = (Some([100, 100, 100]), Some([100, 100, 100]));
        // Shown by one alone: not compared.
        one[2] = Some([200, 0, 0]);
        other[3] = Some([0, 200, 0]);
        assert_eq!(apart(&one, &other), (3, 2));
        assert_eq!(apart(&one, &[None; BANDS]), (0, 0));
    }
}
