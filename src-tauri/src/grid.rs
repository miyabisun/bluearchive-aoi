//! Finds the fully visible cards of the item list.
//!
//! The list draws its cards on a flat background colour. Rows of cards are
//! separated by horizontal gaps, and columns by gaps that lean with the cards'
//! shear. Sizes are measured from each frame, so any window size works.

use image::RgbImage;

const BACKGROUND: [u8; 3] = [196, 207, 212];
const BACKGROUND_TOLERANCE: u8 = 6;
/// Horizontal shift of card edges per pixel downwards (they lean right going up).
pub const SHEAR: f32 = 0.1875;
/// A row is a gap when this share of the panel width is background.
const GAP_ROW_SHARE: f32 = 0.85;
/// An edge row shorter than the card height by more than this is cut off.
const CUT_OFF_SLACK: u32 = 3;
/// Spans between gaps shorter than this are no row of cards.
const MIN_ROW: u32 = 20;
/// A card is about as wide as tall (170 by 164 px); a span wider than this
/// share of the card height is cards whose gaps did not show (a frame too
/// small), which is no card.
const WIDEST_CARD: f32 = 1.5;
/// The padding over a list's first row is taller than this share of the card
/// height (11 and 13 px over 165 px cards); a gap between rows is lower (7 px
/// at most).
const START_PADDING: f32 = 0.055;

/// A card, measured at its vertical centre where the shear has no effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub left: f32,
    pub width: f32,
    pub top: u32,
    pub height: u32,
}

impl Card {
    pub fn center_x(&self) -> f32 {
        self.left + self.width / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.top as f32 + self.height as f32 / 2.0
    }

    /// Horizontal offset of the card's edges at row `y` relative to its centre row.
    pub fn shift_at(&self, y: f32) -> f32 {
        -SHEAR * (y - self.center_y())
    }
}

fn is_background(image: &RgbImage, x: u32, y: u32) -> bool {
    is_colour(image.get_pixel(x, y).0, BACKGROUND, BACKGROUND_TOLERANCE)
}

fn is_colour(p: [u8; 3], colour: [u8; 3], tolerance: u8) -> bool {
    p.iter()
        .zip(colour)
        .all(|(&a, b)| a.abs_diff(b) <= tolerance)
}

/// The flat colour of the remaining-time badge ("16日") time-limited items wear.
const BADGE: [u8; 3] = [42, 64, 90];
const BADGE_TOLERANCE: u8 = 8;
/// A row through a badge is mostly badge over this share of the card width.
const BADGE_ROW_SHARE: f32 = 0.25;

/// The first row below the remaining-time badge over the card's top, when it
/// wears one. The badge hides the top of the icon.
pub fn badge_bottom(image: &RgbImage, card: &Card) -> Option<u32> {
    let height = card.height as f32;
    let first = (card.top as f32 - 0.1 * height).max(0.0) as u32;
    let last = ((card.top as f32 + 0.35 * height) as u32).min(image.height());
    (first..last)
        .filter(|&y| {
            let shift = card.shift_at(y as f32);
            let from = (card.left + shift).max(0.0) as u32;
            let to = ((card.left + card.width + shift) as u32).min(image.width());
            let badge = (from..to)
                .filter(|&x| is_colour(image.get_pixel(x, y).0, BADGE, BADGE_TOLERANCE))
                .count();
            badge as f32 >= BADGE_ROW_SHARE * card.width
        })
        .max()
        .map(|y| y + 3)
}

/// Longest horizontal run of background pixels in row `y`, as `(start, len)`.
fn longest_run(image: &RgbImage, y: u32) -> (u32, u32) {
    let (mut best, mut start, mut len) = ((0, 0), 0, 0);
    for x in 0..image.width() {
        if is_background(image, x, y) {
            if len == 0 {
                start = x;
            }
            len += 1;
            if len > best.1 {
                best = (start, len);
            }
        } else {
            len = 0;
        }
    }
    best
}

/// Spans `[start, end)` of consecutive `true` values.
fn spans(flags: &[bool]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &f) in flags.iter().chain(std::iter::once(&false)).enumerate() {
        let i = u32::try_from(i).unwrap();
        match (f, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// The list panel: its background's horizontal extent, the top of its scrolling
/// viewport, the height of a card and whether the list's start shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub card_height: u32,
    pub shows_start: bool,
}

/// The panel's run of background shows in at least this many rows (its gaps
/// between the card rows are several rows high each).
const PANEL_ROWS: usize = 3;

/// The panel and its card rows `(top, bottom)`, when the frame shows the list.
fn layout(image: &RgbImage) -> Option<(Panel, Vec<(u32, u32)>)> {
    let runs: Vec<(u32, u32)> = (0..image.height()).map(|y| longest_run(image, y)).collect();
    // The panel's run, the longest that several rows share: a stray row of its
    // colour elsewhere is one row alone.
    let alike = |a: &(u32, u32), b: &(u32, u32)| a.0.abs_diff(b.0) <= 2 && a.1.abs_diff(b.1) <= 2;
    let &(panel_left, panel_width) = runs
        .iter()
        .filter(|r| runs.iter().filter(|o| alike(r, o)).count() >= PANEL_ROWS)
        .max_by_key(|r| r.1)?;
    if panel_width < image.width() / 4 {
        return None;
    }
    let panel_right = panel_left + panel_width;

    let gap_rows: Vec<bool> = (0..image.height())
        .map(|y| {
            let bg = (panel_left..panel_right)
                .filter(|&x| is_background(image, x, y))
                .count();
            bg as f32 >= GAP_ROW_SHARE * panel_width as f32
        })
        .collect();
    let gaps = spans(&gap_rows);
    // Card rows lie between two gaps; the viewport edges count as gaps too. A badge
    // spilling into a gap splits it, leaving a sliver that is no row.
    let rows: Vec<(u32, u32)> = gaps
        .windows(2)
        .map(|w| (w[0].1, w[1].0))
        .filter(|(top, bottom)| bottom - top >= MIN_ROW)
        .collect();
    let [first, .., last] = rows.as_slice() else {
        return None;
    };
    // Only the rows at the viewport's edges can be cut off. Inner rows are whole, but
    // a badge can make one look a little taller or shorter, so their middle height is
    // the card height.
    let mut inner: Vec<u32> = rows[1..rows.len() - 1].iter().map(|(a, b)| b - a).collect();
    inner.sort_unstable();
    let card_height = inner
        .get(inner.len() / 2)
        .copied()
        .unwrap_or_else(|| (last.1 - last.0).max(first.1 - first.0));
    let panel = Panel {
        left: panel_left,
        right: panel_right,
        top: gaps[0].0,
        card_height,
        // The viewport's top gap lies right over the first row and is the padding.
        shows_start: gaps[0].1 == first.0
            && (gaps[0].1 - gaps[0].0) as f32 > START_PADDING * card_height as f32,
    };
    Some((panel, rows))
}

/// The list panel, when the frame shows one.
pub fn find_panel(image: &RgbImage) -> Option<Panel> {
    layout(image).map(|(panel, _)| panel)
}

/// Fully visible cards in reading order (rows top to bottom, left to right).
pub fn find_cards(image: &RgbImage) -> Vec<Card> {
    find(image).map(|(_, cards)| cards).unwrap_or_default()
}

/// The list panel and its fully visible cards, when the frame shows the list.
pub fn find(image: &RgbImage) -> Option<(Panel, Vec<Card>)> {
    let (panel, rows) = layout(image)?;
    let height = panel.card_height;
    let mut cards = Vec::new();
    for (i, &(top, bottom)) in rows.iter().enumerate() {
        let edge = i == 0 || i == rows.len() - 1;
        if edge && bottom - top + CUT_OFF_SLACK < height {
            continue;
        }
        let top = (top + bottom).saturating_sub(height) / 2;
        cards.extend(cards_in_row(image, top, height, panel.left, panel.right));
    }
    Some((panel, cards))
}

fn cards_in_row(
    image: &RgbImage,
    top: u32,
    height: u32,
    panel_left: u32,
    panel_right: u32,
) -> Vec<Card> {
    let mid = top as f32 + height as f32 / 2.0;
    // The middle of the row avoids the rounded corners.
    let ys: Vec<u32> = (top + height / 5..top + height * 4 / 5).collect();
    let gap_columns: Vec<bool> = (panel_left..panel_right)
        .map(|x| {
            let bg = ys
                .iter()
                .filter(|&&y| {
                    let sx = x as f32 - SHEAR * (y as f32 - mid);
                    sx >= 0.0 && (sx as u32) < image.width() && is_background(image, sx as u32, y)
                })
                .count();
            bg * 10 >= ys.len() * 9
        })
        .collect();
    let card_columns: Vec<bool> = gap_columns.iter().map(|g| !g).collect();
    let spans: Vec<(f32, f32)> = spans(&card_columns)
        .into_iter()
        .filter(|(a, b)| b - a >= height / 2)
        .map(|(a, b)| ((panel_left + a) as f32, (b - a) as f32))
        .collect();
    split_spans(&spans, height as f32)
        .into_iter()
        .map(|(left, width)| Card {
            left,
            width,
            top,
            height,
        })
        .collect()
}

/// A card's width for its height, and the gap between two (180 and 4 px by
/// 165), where no card of the row shows them.
const CARD_WIDTH: f32 = 180.0 / 165.0;
const CARD_GAP: f32 = 4.0 / 165.0;
/// How far a run of cards may be from a whole number of them, of a card's width.
const RUN_SLACK: f32 = 0.1;

/// The cards `(left, width)` of a row's `spans`: one card each, as wide as the
/// row's middle one at least, or several whose gaps did not show (too narrow
/// in a small frame), cut where they would be. A span no whole number of cards
/// wide is no card.
fn split_spans(spans: &[(f32, f32)], height: f32) -> Vec<(f32, f32)> {
    let single = |w: f32| w <= height * WIDEST_CARD;
    let mut widths: Vec<f32> = spans.iter().map(|s| s.1).filter(|&w| single(w)).collect();
    widths.sort_by(f32::total_cmp);
    let card = widths
        .get(widths.len() / 2)
        .copied()
        .unwrap_or(height * CARD_WIDTH);
    let gap = height * CARD_GAP;
    let mut cards = Vec::new();
    for &(left, width) in spans {
        // The selected card's blinking frame can hide its edges.
        if width < card {
            cards.push((left - (card - width) / 2.0, card));
            continue;
        }
        if single(width) {
            cards.push((left, width));
            continue;
        }
        let count = ((width + gap) / (card + gap)).round();
        if count >= 2.0 && (count * (card + gap) - gap - width).abs() <= card * RUN_SLACK {
            let each = (width - (count - 1.0) * gap) / count;
            cards.extend((0..count as u32).map(|i| (left + i as f32 * (each + gap), each)));
        }
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> RgbImage {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        image::open(path).unwrap().to_rgb8()
    }

    fn check(name: &str, count: usize, height: u32, pitch: f32) {
        let cards = find_cards(&fixture(name));
        assert_eq!(cards.len(), count, "{name}: {cards:?}");
        for c in &cards {
            assert!(c.height.abs_diff(height) <= 2, "{name}: {c:?}");
        }
        for pair in cards.chunks(5) {
            for w in pair.windows(2) {
                let step = w[1].center_x() - w[0].center_x();
                assert!((step - pitch).abs() <= 3.0, "{name}: column pitch {step}");
            }
        }
    }

    /// The selected card's blinking frame hides its edges: on the tablet's
    /// 1920 stream it showed 147 of its 156 px. It is as wide as the others,
    /// about its middle.
    #[test]
    fn a_card_showing_narrower_is_as_wide_as_the_others() {
        let spans = [
            (1045.0, 156.0),
            (1204.0, 156.0),
            (1367.0, 147.0),
            (1521.0, 157.0),
            (1679.0, 157.0),
        ];
        let cards = split_spans(&spans, 143.0);
        assert_eq!(cards[2], (1362.5, 156.0));
        assert_eq!(cards[3], (1521.0, 157.0));
    }

    /// A row outside the panel can take the panel's colour across the screen
    /// for a pixel's height (the top bar's edge, scaled to 1500x844): the
    /// panel is the run of its colour many rows share.
    #[test]
    fn a_stray_row_of_the_panels_colour_is_no_panel() {
        use image::imageops::{FilterType, resize};
        let small = resize(
            &fixture("items-16x9-tech-notes.png"),
            1500,
            844,
            FilterType::Triangle,
        );
        assert_eq!(find_cards(&small).len(), 20);
    }

    #[test]
    fn finds_full_rows_in_16_by_9() {
        check("items-16x9-secretstone.png", 15, 164, 184.0);
    }

    #[test]
    fn finds_full_rows_in_4_by_3() {
        check("items-4x3-ooparts.png", 30, 133, 148.5);
    }

    /// A frame too small for the gaps between cards to show (scaled down to
    /// 1024 wide and back up) gives no card spanning several: reading one
    /// took minutes.
    #[test]
    fn cards_run_together_are_no_card() {
        use image::imageops::{FilterType, resize};
        let image = fixture("items-16x9-event-items.png");
        let small = resize(&image, 1024, 576, FilterType::Triangle);
        let blurred = resize(&small, 2222, 1250, FilterType::Nearest);
        for card in find_cards(&blurred) {
            assert!(card.width <= card.height as f32 * 1.5, "{card:?}");
        }
    }

    /// The smallest screen read (1500 wide, see `screen::too_small`) draws the
    /// gaps between cards narrower: every card is still found.
    #[test]
    fn a_small_frame_shows_every_card() {
        use image::imageops::{FilterType, resize};
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let fixtures: Vec<Fixture> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap())
                .unwrap();
        let mut wrong = Vec::new();
        for fixture in fixtures {
            let image = fixture_image(&dir, &fixture.image);
            let height = image.height() * 1500 / image.width();
            let small = resize(&image, 1500, height, FilterType::Triangle);
            let found = find_cards(&small).len();
            if found != fixture.cells.len() {
                wrong.push(format!(
                    "{}: {found} of {}",
                    fixture.image,
                    fixture.cells.len()
                ));
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    #[test]
    fn tells_where_the_list_starts() {
        for (name, start) in [
            ("items-16x9-list-top.png", true),
            ("equipment-16x9-top.png", true),
            ("items-16x9-secretstone.png", false),
            ("items-4x3-ooparts.png", false),
            ("items-16x9-coins-materials.png", false),
            ("items-16x9-gifts-tickets.png", false),
            ("items-16x9-event-items.png", false),
            ("equipment-16x9-middle.png", false),
            ("equipment-16x9-end.png", false),
        ] {
            let panel = find_panel(&fixture(name)).unwrap();
            assert_eq!(panel.shows_start, start, "{name}");
        }
    }

    #[derive(serde::Deserialize)]
    struct Fixture {
        image: String,
        cells: Vec<Label>,
    }

    #[derive(serde::Deserialize)]
    struct Label {
        #[serde(default)]
        badge: bool,
    }

    #[test]
    fn finds_the_remaining_time_badges() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let fixtures: Vec<Fixture> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json")).unwrap())
                .unwrap();
        let mut wrong = Vec::new();
        for fixture in fixtures {
            let image = fixture_image(&dir, &fixture.image);
            for (i, (card, label)) in find_cards(&image).iter().zip(&fixture.cells).enumerate() {
                let found = badge_bottom(&image, card).is_some();
                if found != label.badge {
                    wrong.push(format!("{} cell {i}: badge {found}", fixture.image));
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    fn fixture_image(dir: &Path, name: &str) -> RgbImage {
        image::open(dir.join(name)).unwrap().to_rgb8()
    }
}
