//! Which list the screen shows, and whether it is sorted the default way,
//! ascending ("デフォルト ≡↑").
//!
//! The list's order tells shared icons apart, so lists are only read while the
//! sort buttons show the default order. The buttons sit at a fixed distance from
//! the window's top right corner, and the screen's title ("アイテム", "装備") at
//! one from the top left, measured in card heights at any window size. They are
//! compared with `data/sort-default.png` and `data/title-*.png`, written by
//! `cargo run --example make_templates`.

use std::borrow::Cow;

use image::{
    Rgb, RgbImage,
    imageops::{self, FilterType},
};

/// How large the game draws a screen: the height of a 16:9 window as wide as
/// the screen. A phone's screen is wider, a tablet's taller, and the game's
/// parts keep the size the width gives them, each kept to the top, the middle
/// or the bottom (see `Anchor`): the parts measured in window heights at 16:9
/// are measured in this. A window within a pixel of 16:9 keeps its height.
pub fn unit(width: u32, height: u32) -> f32 {
    let wide = width as f32 * 9.0 / 16.0;
    if (wide - height as f32).abs() < 1.0 {
        height as f32
    } else {
        wide
    }
}

/// Which edge a part of the screen keeps to where the screen is not 16:9 and
/// so shorter or taller than `unit` (measured on 2026-10-14 on a Pixel 9 against
/// a 16:9 window): the top bar and the lists to the top, the student screen's
/// panel to the middle, its name plate to the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Top,
    Middle,
    Bottom,
}

impl Anchor {
    /// Where a 16:9 window's top lies for the parts kept to this edge, in a
    /// screen `height` tall drawn at `unit`.
    pub fn origin(self, height: u32, unit: f32) -> f32 {
        let spare = height as f32 - unit;
        match self {
            Anchor::Top => 0.0,
            Anchor::Middle => spare / 2.0,
            Anchor::Bottom => spare,
        }
    }
}

/// A part of the screen measured in card heights from a top corner: across
/// from `x.0` to `x.1` away from the left (or right) edge, down from `y.0` to
/// `y.1`, kept to `anchor`.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub from_right: bool,
    pub x: (f32, f32),
    pub y: (f32, f32),
    pub anchor: Anchor,
}

pub const SORT_BUTTONS: Region = Region {
    from_right: true,
    x: (3.2, 0.4),
    y: (0.95, 1.42),
    anchor: Anchor::Top,
};
/// The title; below it the 4:3 screen draws a yellow underline (0.41 and down).
pub const TITLE: Region = Region {
    from_right: false,
    x: (1.0, 2.35),
    y: (0.06, 0.4),
    anchor: Anchor::Top,
};
/// The selected "基本情報" tab of the student screen, which has no cards: there
/// a card height is `STUDENT_UNIT` units (what it is on the lists).
pub const STUDENT_TAB: Region = Region {
    from_right: true,
    x: (6.47, 4.59),
    y: (1.24, 1.76),
    anchor: Anchor::Middle,
};
pub const STUDENT_UNIT: f32 = 165.0 / 1250.0;
/// The buttons may sit this far off (card heights), across and down.
const SLACK: (f32, f32) = (0.15, 0.06);
/// Height of the compared picture of the sort buttons, in pixels.
pub const HEIGHT: u32 = 24;
/// Height of the compared picture of a title: telling "アイテム" from "装備"
/// needs less detail, and the search over sizes and positions costs less.
pub const TITLE_HEIGHT: u32 = 12;
/// Least similarity of the buttons to the default order's.
const MATCH: f32 = 0.85;
/// The direction button ("≡↑") starts at this share of the template's width.
const DIRECTION_FROM: f32 = 0.77;
/// Card heights off by these factors are tried too.
const SIZES: [f32; 5] = [0.94, 0.97, 1.0, 1.03, 1.06];
/// The title scales a little differently from the cards between window shapes
/// (measured 10% between 16:9 and 4:3).
const TITLE_SIZES: [f32; 5] = [0.88, 0.94, 1.0, 1.06, 1.12];
/// Least similarity of a title, and its least lead over the other titles.
/// Measured on 2026-10-12: a list's own title 0.92 to 0.99, the other's 0.27 to 0.43.
const TITLE_MATCH: f32 = 0.8;
const TITLE_LEAD: f32 = 0.1;

/// The `region`, resampled to `height` pixels tall, with `slack` card heights
/// around it.
pub fn crop(
    image: &RgbImage,
    card_height: u32,
    region: Region,
    slack: (f32, f32),
    height: u32,
) -> Option<RgbImage> {
    let unit = card_height as f32;
    let scale = height as f32 / ((region.y.1 - region.y.0) * unit);
    let width = image.width() as f32;
    let (x0, x1) = if region.from_right {
        (
            width - (region.x.0 + slack.0) * unit,
            width - (region.x.1 - slack.0) * unit,
        )
    } else {
        ((region.x.0 - slack.0) * unit, (region.x.1 + slack.0) * unit)
    };
    let origin = region
        .anchor
        .origin(image.height(), self::unit(image.width(), image.height()));
    let y0 = origin + (region.y.0 - slack.1) * unit;
    let y1 = origin + (region.y.1 + slack.1) * unit;
    if x0 < 0.0 || y0 < 0.0 || x1 > width || y1 > image.height() as f32 {
        return None;
    }
    let crop = image::imageops::crop_imm(
        image,
        x0 as u32,
        y0 as u32,
        (x1 - x0) as u32,
        (y1 - y0) as u32,
    )
    .to_image();
    Some(image::imageops::resize(
        &crop,
        ((x1 - x0) * scale).round() as u32,
        ((y1 - y0) * scale).round() as u32,
        FilterType::Triangle,
    ))
}

/// The top bar's white starts this share of the game's width in (right of
/// the back button) and runs to its right end, at `BAR_ROW` window heights
/// down, over the bar's icons.
const BAR_START: f32 = 163.0 / 2222.0;
const BAR_ROW: f32 = 0.003;
/// The bar's white, and the least share of the window it spans.
const BAR_WHITE: u32 = 235;
const BAR_SPAN: f32 = 0.5;
/// Insets narrower than this share of the window are a pixel of the bar's
/// soft ends, not a phone's.
const INSET_SLACK: f32 = 0.01;

/// The columns `(left, right)` the game draws its screens in: all of a window,
/// a phone's safe area short of its rounded corners and camera, as far in on
/// either side. The top bar's white starts a set share of the safe area's
/// width in from its left end, which tells the inset. (The bar runs past the
/// safe area's right end: measured on 2026-10-14 on a Pixel 9, the student
/// panel's parts lie as at 16:9 in 2082 columns from 170 or so, the bar's
/// white runs to 2263.) A screen without the bar keeps it all, and so does one
/// no wider than 16:9, which the game fills.
pub fn safe_area(image: &RgbImage) -> (u32, u32) {
    let (width, height) = image.dimensions();
    if u64::from(width) * 9 <= u64::from(height) * 16 {
        return (0, width);
    }
    let row = (BAR_ROW * height as f32).round() as u32;
    let Some((start, _)) = bar((0..width).map(|x| image.get_pixel(x, row))) else {
        return (0, width);
    };
    let span = width as f32;
    let inset = (start as f32 - BAR_START * span) / (1.0 - 2.0 * BAR_START);
    if inset <= INSET_SLACK * span {
        return (0, width);
    }
    let inset = inset.round() as u32;
    (inset, width - inset)
}

/// The top bar's white along `line`, where it runs over `BAR_SPAN` of it: its
/// longest run, `(start, end)`.
fn bar<'a>(line: impl ExactSizeIterator<Item = &'a Rgb<u8>>) -> Option<(u32, u32)> {
    let length = line.len();
    let white = |p: &Rgb<u8>| {
        let [r, g, b] = p.0;
        (299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) / 1000 > BAR_WHITE
    };
    let mut longest: Option<(u32, u32)> = None;
    let mut from = None;
    for (at, lit) in line.map(white).chain([false]).enumerate() {
        let at = at as u32;
        match (lit, from) {
            (true, None) => from = Some(at),
            (false, Some(start)) => {
                if longest.is_none_or(|(a, b)| at - start > b - a) {
                    longest = Some((start, at));
                }
                from = None;
            }
            _ => {}
        }
    }
    longest.filter(|(a, b)| (b - a) as f32 >= BAR_SPAN * length as f32)
}

/// Which way to turn a frame that came upright back to the game's landscape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Clockwise,
    Counterclockwise,
}

impl Turn {
    pub fn apply(self, image: &RgbImage) -> RgbImage {
        match self {
            Self::Clockwise => imageops::rotate90(image),
            Self::Counterclockwise => imageops::rotate270(image),
        }
    }
}

/// How to turn a frame that came upright (a phone held sideways may send its
/// screen so), the top bar along one side; `None` for one in landscape or with
/// no bar to tell.
pub fn turn(image: &RgbImage) -> Option<Turn> {
    let (width, height) = image.dimensions();
    if width >= height {
        return None;
    }
    let side = (BAR_ROW * width as f32).round() as u32;
    let column = |x: u32| bar((0..height).map(|y| image.get_pixel(x, y))).is_some();
    if column(side) {
        Some(Turn::Clockwise)
    } else if column(width - 1 - side) {
        Some(Turn::Counterclockwise)
    } else {
        None
    }
}

/// The brightest a border's pixels may be to be cut off (black, with the
/// codec's noise over it).
const BORDER_DARK: u8 = 24;

/// The frame without the black borders a sender boxed it in; one mostly black
/// (a loading screen) as it is.
pub fn trim(image: &RgbImage) -> Cow<'_, RgbImage> {
    let (width, height) = image.dimensions();
    let dark = |x: u32, y: u32| image.get_pixel(x, y).0.iter().all(|&c| c <= BORDER_DARK);
    let row = |y: u32| (0..width).all(|x| dark(x, y));
    let column = |x: u32| (0..height).all(|y| dark(x, y));
    let Some(top) = (0..height).find(|&y| !row(y)) else {
        return Cow::Borrowed(image);
    };
    let bottom = (0..height)
        .rev()
        .find(|&y| !row(y))
        .map_or(height, |y| y + 1);
    let left = (0..width).find(|&x| !column(x)).unwrap_or(0);
    let right = (0..width)
        .rev()
        .find(|&x| !column(x))
        .map_or(width, |x| x + 1);
    let (kept_width, kept_height) = (right - left, bottom - top);
    // A screen sent upright into a wide stream keeps a quarter of it or so.
    if u64::from(kept_width) * u64::from(kept_height) * 8 < u64::from(width) * u64::from(height)
        || (kept_width, kept_height) == (width, height)
    {
        return Cow::Borrowed(image);
    }
    Cow::Owned(imageops::crop_imm(image, left, top, kept_width, kept_height).to_image())
}

/// The widths the texts are read at, with references of their own: the
/// 2222x1250 window the references were first drawn from, and smaller screens
/// (a Full HD window, a phone's, or a stream of one, at 1080 lines or 720),
/// whose letters lose their shapes too far for the window's references
/// (measured on 2026-10-15: the student screen's names and stars, the credits
/// and the shops' bars read wrong or not at all at 1600 wide with the
/// window's; the equipment levels not at all at 1920).
pub const WIDTHS: [u32; 4] = [2222, 1920, 1600, 1280];

/// The width of a 16:9 window drawing the game's parts as large as `image`.
pub fn drawn_width(image: &RgbImage) -> f32 {
    unit(image.width(), image.height()) * 16.0 / 9.0
}

/// The least width a screen is read at: a little short of the smallest of
/// `WIDTHS`, where reading was measured right or not at all (on 2026-10-15:
/// scaled to about 1150 wide, a bond of "25" read "5").
const SMALLEST: f32 = 1240.0;

/// Whether `image` draws the game's parts too small to read. An upright
/// screen is no game's (the game is drawn sideways only), small or not.
pub fn too_small(image: &RgbImage) -> bool {
    image.width() > image.height() && drawn_width(image) < SMALLEST
}

/// The width among `WIDTHS` nearest to `width`.
pub fn nearest_width(width: f32) -> u32 {
    WIDTHS
        .into_iter()
        .min_by(|a, b| {
            (*a as f32 - width)
                .abs()
                .total_cmp(&(*b as f32 - width).abs())
        })
        .unwrap_or(WIDTHS[0])
}

/// The frame as the game lays it out: a phone's safe area (see `safe_area`)
/// scaled to the nearest of `WIDTHS` (a phone draws its letters smaller than
/// a window as wide); a window's frame as it is.
pub fn normalize(image: &RgbImage) -> Cow<'_, RgbImage> {
    let (left, right) = safe_area(image);
    if (left, right) == (0, image.width()) {
        return Cow::Borrowed(image);
    }
    let area = imageops::crop_imm(image, left, 0, right - left, image.height());
    let width = nearest_width((right - left) as f32);
    let height = (image.height() as f32 * width as f32 / (right - left) as f32).round();
    Cow::Owned(imageops::resize(
        &*area,
        width,
        height as u32,
        FilterType::Nearest,
    ))
}

/// Normalized cross-correlation of the template's columns `columns` with the
/// image placed at `(ox, oy)`.
fn ncc(
    template: &RgbImage,
    columns: std::ops::Range<u32>,
    image: &RgbImage,
    ox: u32,
    oy: u32,
) -> f32 {
    let (mut n, mut st, mut si, mut stt, mut sii, mut sti) = (0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0);
    for y in 0..template.height() {
        for x in columns.clone() {
            let (wanted, seen) = (template.get_pixel(x, y), image.get_pixel(ox + x, oy + y));
            for c in 0..3 {
                let (a, b) = (f32::from(wanted.0[c]), f32::from(seen.0[c]));
                n += 1.0;
                st += a;
                si += b;
                stt += a * a;
                sii += b * b;
                sti += a * b;
            }
        }
    }
    let var_t = stt - st * st / n;
    let var_i = sii - si * si / n;
    if var_t <= 0.0 || var_i <= 0.0 {
        return -1.0;
    }
    (sti - st * si / n) / (var_t * var_i).sqrt()
}

/// Whether the frame's sort buttons show the default order, ascending. The
/// order's name and the direction button must both match: the name alone
/// would pass a descending list. Window sizes and game versions shift the
/// buttons a little, so a few sizes and positions are tried.
pub fn sorted_by_default(image: &RgbImage, card_height: u32, template: &RgbImage) -> bool {
    let split = (template.width() as f32 * DIRECTION_FROM) as u32;
    let best = best_match(
        image,
        card_height,
        (SORT_BUTTONS, HEIGHT),
        &SIZES,
        template,
        |area, ox, oy| {
            let name = ncc(template, 0..split, area, ox, oy);
            let direction = ncc(template, split..template.width(), area, ox, oy);
            name.min(direction)
        },
    );
    best >= MATCH
}

/// Which of `titles` (pictures of `TITLE`) the screen shows, if one clearly does.
pub fn title(image: &RgbImage, card_height: u32, titles: &[RgbImage]) -> Option<usize> {
    let scores: Vec<f32> = titles
        .iter()
        .map(|t| {
            best_match(
                image,
                card_height,
                (TITLE, TITLE_HEIGHT),
                &TITLE_SIZES,
                t,
                |area, ox, oy| ncc(t, 0..t.width(), area, ox, oy),
            )
        })
        .collect();
    let (best, score) = scores
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    let lead = scores
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != best)
        .all(|(_, s)| score - s >= TITLE_LEAD);
    (score >= TITLE_MATCH && lead).then_some(best)
}

/// How alike the student screen's "基本情報" tab is to `template` (the tab
/// selected), at the unit `STUDENT_UNIT` gives.
pub fn student_tab(image: &RgbImage, template: &RgbImage) -> bool {
    let unit = (self::unit(image.width(), image.height()) * STUDENT_UNIT).round() as u32;
    best_match(
        image,
        unit,
        (STUDENT_TAB, HEIGHT),
        &SIZES,
        template,
        |area, ox, oy| ncc(template, 0..template.width(), area, ox, oy),
    ) >= MATCH
}

/// The best `score` of `template` over the positions and `sizes` tried in
/// `region` (pictured `height` pixels tall).
fn best_match(
    image: &RgbImage,
    card_height: u32,
    (region, height): (Region, u32),
    sizes: &[f32],
    template: &RgbImage,
    score: impl Fn(&RgbImage, u32, u32) -> f32,
) -> f32 {
    let mut best = f32::MIN;
    for size in sizes {
        let unit = (card_height as f32 * size).round() as u32;
        let Some(area) = crop(image, unit, region, SLACK, height) else {
            continue;
        };
        if area.width() < template.width() || area.height() < template.height() {
            continue;
        }
        for oy in 0..=area.height() - template.height() {
            for ox in 0..=area.width() - template.width() {
                best = best.max(score(&area, ox, oy));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use image::RgbImage;

    use super::{
        Turn, drawn_width, nearest_width, normalize, safe_area, sorted_by_default, too_small, trim,
        turn,
    };
    use crate::grid::find_panel;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn template() -> RgbImage {
        image::open(root().join("data/sort-default.png"))
            .unwrap()
            .to_rgb8()
    }

    fn shows_default(image: &RgbImage) -> bool {
        find_panel(image).is_some_and(|p| sorted_by_default(image, p.card_height, &template()))
    }

    fn fixture(name: &str) -> RgbImage {
        image::open(root().join("tests/fixtures").join(name))
            .unwrap()
            .to_rgb8()
    }

    #[test]
    fn every_item_list_fixture_is_in_the_default_order() {
        for name in [
            "items-16x9-secretstone.png",
            "items-16x9-list-top.png",
            "items-16x9-coins-materials.png",
            "items-16x9-gifts-tickets.png",
            "items-16x9-event-items.png",
            "items-4x3-ooparts.png",
        ] {
            assert!(shows_default(&fixture(name)), "{name}");
        }
    }

    #[test]
    fn other_screens_are_not_the_item_list() {
        for name in [
            "other/home.png",
            "other/momotalk.png",
            "other/event-stages.png",
        ] {
            assert!(!shows_default(&fixture(name)), "{name}");
        }
    }

    /// The game sizes its parts by the screen's width, whatever its shape
    /// (measured on 2026-10-14: a card is 165 pixels high in a 2222x1250
    /// window, 155 on a phone's 2080-wide safe area, 226 on a 3040x1904 tablet).
    #[test]
    fn the_unit_follows_the_width() {
        for (width, height, unit) in [
            (2222, 1250, 1250.0),
            (2080, 1080, 1170.0),
            (3040, 1904, 1710.0),
        ] {
            assert!(
                (super::unit(width, height) - unit).abs() < 0.01,
                "{width}x{height}"
            );
        }
    }

    /// Each screen is read with the references of the width nearest to the
    /// one it draws the game's parts at, a 16:9 window's.
    #[test]
    fn a_screen_is_read_at_the_nearest_width() {
        assert_eq!(nearest_width(2222.0), 2222);
        assert_eq!(nearest_width(3040.0), 2222);
        assert_eq!(nearest_width(1920.0), 1920);
        assert_eq!(nearest_width(2100.0), 2222);
        assert_eq!(nearest_width(1700.0), 1600);
        assert_eq!(nearest_width(1400.0), 1280);
        assert_eq!(nearest_width(800.0), 1280);
        assert!((drawn_width(&RgbImage::new(1600, 900)) - 1600.0).abs() < 1.0);
        // A 4:3 window draws its parts as large as a 16:9 window as wide.
        assert!((drawn_width(&RgbImage::new(1744, 1308)) - 1744.0).abs() < 1.0);
        // A smaller phone's screen is scaled to the nearest width, not up to
        // the largest.
        let phone = fixture("android/top-bar.png");
        let small =
            image::imageops::resize(&phone, 1600, 713, image::imageops::FilterType::Triangle);
        assert_eq!(normalize(&small).width(), 1280);
        assert_eq!(normalize(&phone).width(), 2222);
        // Smaller than the smallest width read, a screen is not read at all.
        assert!(!too_small(&RgbImage::new(1280, 720)));
        assert!(!too_small(&normalize(&small)));
        assert!(too_small(&RgbImage::new(1100, 619)));
        // Upright, it is no game (it is drawn sideways only), small or not.
        assert!(!too_small(&RgbImage::new(1202, 1920)));
    }

    /// A phone keeps the game in from its rounded corners and its camera, as
    /// far on either side: the top bar shows how far (measured on 2026-10-14
    /// on a Pixel 9 at 2424x1080: the student panel's parts lie as at 16:9 in
    /// 2082 columns from 170 or so, the back button and the bar 172). A
    /// window keeps it all.
    #[test]
    fn the_safe_area_is_as_far_in_as_the_top_bar_starts() {
        let (left, right) = safe_area(&fixture("android/top-bar.png"));
        assert!(
            left.abs_diff(171) <= 2 && left + right == 2424,
            "{left}..{right}"
        );
        // Scaled to the window the references were drawn from.
        let phone = fixture("android/top-bar.png");
        let normal = normalize(&phone);
        let height = 1080.0 * 2222.0 / f64::from(right - left);
        assert_eq!(normal.width(), 2222);
        assert!((f64::from(normal.height()) - height).abs() < 1.0);
        for name in [
            "items-16x9-list-top.png",
            "items-4x3-ooparts.png",
            "credits/credit-02.png",
        ] {
            let image = fixture(name);
            assert_eq!(safe_area(&image), (0, image.width()), "{name}");
            assert!(
                matches!(normalize(&image), std::borrow::Cow::Borrowed(_)),
                "{name}"
            );
        }
    }

    /// A frame sent letterboxed comes back as the game drew it; one mostly
    /// black (a loading screen) stays as it is.
    #[test]
    fn black_borders_are_cut_off() {
        let game = RgbImage::from_fn(640, 300, |x, y| [x as u8 | 64, y as u8 | 64, 128].into());
        let mut boxed = RgbImage::new(700, 500);
        image::imageops::replace(&mut boxed, &game, 30, 100);
        assert_eq!(*trim(&boxed), game);
        assert_eq!(*trim(&game), game);
        // A screen sent upright into a wide stream.
        let upright = RgbImage::from_fn(160, 500, |x, y| [x as u8 | 64, y as u8 | 64, 128].into());
        let mut wide = RgbImage::new(900, 500);
        image::imageops::replace(&mut wide, &upright, 370, 0);
        assert_eq!(*trim(&wide), upright);
        let mut dark = RgbImage::new(400, 300);
        image::imageops::replace(
            &mut dark,
            &RgbImage::from_pixel(100, 100, [255; 3].into()),
            150,
            100,
        );
        assert_eq!(*trim(&dark), dark);
    }

    /// A phone held sideways may send its screen upright: the top bar on one
    /// side tells which way to turn it back.
    #[test]
    fn a_frame_sent_upright_is_turned_back() {
        let phone = fixture("android/top-bar.png");
        let left = image::imageops::rotate270(&phone);
        let right = image::imageops::rotate90(&phone);
        assert_eq!(turn(&left), Some(Turn::Clockwise));
        assert_eq!(turn(&right), Some(Turn::Counterclockwise));
        assert_eq!(Turn::Clockwise.apply(&left), phone);
        assert_eq!(Turn::Counterclockwise.apply(&right), phone);
        assert_eq!(turn(&phone), None);
        assert_eq!(turn(&RgbImage::new(300, 600)), None);
    }

    /// No descending screenshot yet: the "≡↑" button turned upside down stands in.
    #[test]
    fn a_descending_arrow_is_not_the_default_order() {
        let mut image = fixture("items-16x9-secretstone.png");
        let (x0, y0, x1, y1) = (2050, 158, 2170, 238);
        let button = image::imageops::crop_imm(&image, x0, y0, x1 - x0, y1 - y0).to_image();
        let flipped = image::imageops::flip_vertical(&button);
        image::imageops::replace(&mut image, &flipped, i64::from(x0), i64::from(y0));
        assert!(!shows_default(&image));
    }
}
