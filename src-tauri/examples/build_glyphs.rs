//! Writes the glyph bank (`data/glyphs.png` and `data/glyphs.json`, see
//! `aoi::glyph::bank`) from the fixtures labeled `"role": "tune"` and a font.
//! Evaluation fixtures are never used here.
//!
//! - The text styles of the student screen (`tests/fixtures/students.json`,
//!   `tests/fixtures/levels.json`).
//! - The digits: the credits (`tests/fixtures/credits.json`) and the coins held
//!   over the shops' cards (`tests/fixtures/shops.json`); and those of the
//!   cards' bars and prices. The fixtures do not tell which card is which
//!   label, so these are first drawn with Noto Sans JP in three weights (with
//!   the credits' digits, the same font dark on light), then the cards are
//!   read with them loosely, and the digits of a card that reads as one of its
//!   fixture's labels, but for at most one digit not yet known, are taken as
//!   the game draws them; again, while that takes more.
//! - The shops' names, as each shop's chosen menu entry draws them.
//! - The students' names, drawn with Noto Sans JP Regular, and the letters the
//!   tuning screens draw of them.
//!
//! Noto Sans JP is under the SIL Open Font License 1.1; its Regular, Medium
//! and Bold (`NotoSansJP-*.otf` from
//! <https://github.com/notofonts/noto-cjk/tree/main/Sans/SubsetOTF/JP>) are
//! read from a folder given, not kept in the repository. Measured on 2026-10-14, the game's letters are
//!   nearest Noto Sans JP's (Regular or Medium) among the free Japanese fonts
//!   tried (M PLUS 1p, Noto Sans JP in four weights).
//!
//! The cuts of a character are grouped by likeness and each group averaged, so
//! a character drawn in more than one way keeps a reference for each.
//! Run with `cargo run --example build_glyphs -- <folder of the fonts>` after
//! the fixtures or SchaleDB's students change.

use std::{collections::BTreeMap, path::Path};

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use aoi::{
    credit,
    glyph::{
        self, Style,
        bank::{Bank, Word, glyph_tile},
        word::Ink,
    },
    screen, shop,
    student::{self, Label, LevelLabel},
};
use image::{
    Rgb, RgbImage,
    imageops::{FilterType, resize},
};
use serde::Deserialize;

/// A cut farther than this from every group of its character starts a group.
/// A group's mean drifts from its cuts as more join it: at 10 a tablet's "3"
/// took a window's credits' "3" nearer to "8" (measured on 2026-10-16), and
/// the cards' small digits read fewer.
const JOIN: f32 = 5.0;
/// How loosely the digits drawn with the font read the game's: the nearest
/// digit within this distance, and within this share of the next nearest one
/// (measured on 2026-10-14: the game's digits lie 12 to 30 from the font's,
/// the letters around them 38 or farther).
const LOOSE: (f32, f32) = (34.0, 0.85);
/// Widths between those read that a bank also draws its references from: a
/// window may be any width, scaled down to the width read (see
/// `screen::normalize`), and drawn there its letters differ from the same
/// drawn at the width read; most so a card's small digits, whose counts read
/// 94 of the 105 offers at 1920 with 2040 and 2160 alone, 102 with 1980 and
/// 2100 too (measured on 2026-10-16). (The tests read others between.)
const BETWEEN: [(u32, &[u32]); 4] = [
    (1920, &[1980, 2040, 2100, 2160]),
    (1760, &[1780, 1800, 1820, 1840, 1860, 1880, 1900]),
    (1600, &[1620, 1640, 1660, 1680, 1700, 1720, 1740]),
    (1500, &[1520, 1540, 1560, 1580]),
];
/// Widths a phone's stream draws the game at, which a bank also draws its
/// student screen's references from, as such a stream shows them (see
/// `streamed`), and how much the phone scales its screen down for it: a
/// Pixel 9 sending 1920 by 854 (its 2424 columns to 1920) draws it 1646
/// wide, read at 1600, its bonds' and levels' thin digits farther from the
/// window's than they may lie; sending 2424 by 1080 it draws it 2082 wide,
/// read at 1920, a bond's "6" at 0.6 of the way to a "5" (measured on
/// 2026-10-24).
const STREAMS: [(u32, f32, &[u32]); 2] = [
    (1600, 1920.0 / 2424.0, &[1620, 1660, 1700, 1740]),
    (1920, 1.0, &[2000, 2080, 2160]),
];
/// Where a stream's pixels fall on the screen's, in the screen's pixels,
/// across and down: a character a pixel taller cuts out narrower (a bond's
/// "6" 26 from its references, measured on 2026-10-24), so each is drawn
/// at these offsets too.
const PHASES: [(f32, f32); 6] = [
    (0.0, 0.0),
    (0.0, 0.33),
    (0.0, 0.67),
    (0.5, 0.0),
    (0.5, 0.33),
    (0.5, 0.67),
];
/// A bar's letter reference lies farther than this from every digit's.
const APART: f32 = 20.0;
/// A digit of a smaller screen's card lies at most this far from the
/// window's digit at its place (see `take_scaled_card_digits`).
const ALIKE: f32 = 30.0;
/// The size the names are drawn at, in pixels.
const SIZE: f32 = 64.0;

/// A group of alike cuts: their sum and how many.
struct Group(Vec<f32>, u32);

impl Group {
    fn mean(&self) -> Vec<f32> {
        self.0.iter().map(|v| v / self.1 as f32).collect()
    }
}

/// Adds a cut to the nearest group of its character, or starts a group.
fn join(groups: &mut Vec<Group>, shape: Vec<f32>) {
    let nearest = groups
        .iter_mut()
        .map(|g| (glyph::distance(&g.mean(), &shape), g))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|(d, _)| *d <= JOIN);
    match nearest {
        Some((_, group)) => {
            group.0.iter_mut().zip(&shape).for_each(|(s, v)| *s += v);
            group.1 += 1;
        }
        None => groups.push(Group(shape, 1)),
    }
}

/// The cuts of each style's characters.
type Cuts = BTreeMap<String, BTreeMap<char, Vec<Group>>>;

/// Adds the cuts of `text` in `style`, when as many characters were cut out.
fn add(cuts: &mut Cuts, path: &str, style: &str, shapes: Vec<Vec<f32>>, text: &str) {
    if shapes.len() != text.chars().count() {
        println!(
            "{path} {style} {text:?}: {} characters cut out",
            shapes.len()
        );
        return;
    }
    for (c, shape) in text.chars().zip(shapes) {
        join(
            cuts.entry(style.to_owned())
                .or_default()
                .entry(c)
                .or_default(),
            shape,
        );
    }
}

/// A name plate labeled with its student (`tests/fixtures/names.json`).
#[derive(Deserialize)]
struct NameLabel {
    image: String,
    role: String,
    id: i64,
}

#[derive(Deserialize)]
struct CreditsLabel {
    image: String,
    role: String,
    credits: Option<u64>,
}

#[derive(Deserialize)]
struct ShopLabel {
    image: String,
    role: String,
    shop: String,
    #[serde(default)]
    held: Option<Vec<Option<u64>>>,
    offers: Vec<OfferLabel>,
}

#[derive(Deserialize)]
struct OfferLabel {
    left: u32,
    price: u32,
}

/// The digit `shape` loosely reads as among `font`'s (see `LOOSE`).
fn loosely(font: &BTreeMap<char, Vec<Group>>, shape: &[f32]) -> Option<char> {
    let mut nearest: Vec<(char, f32)> = font
        .iter()
        .map(|(c, gs)| {
            let d = gs.iter().map(|g| glyph::distance(&g.mean(), shape));
            (*c, d.fold(f32::INFINITY, f32::min))
        })
        .collect();
    nearest.sort_by(|a, b| a.1.total_cmp(&b.1));
    let ((c, d), next) = (nearest[0], nearest[1].1);
    (d <= LOOSE.0 && d <= LOOSE.1 * next).then_some(c)
}

/// Where among `shapes` the number `value` is drawn, read loosely with
/// `font`: the place whose shapes read as its digits, but for at most one
/// that reads as no digit (one `font` does not know yet), and none as another
/// digit. `None` where no place or two do.
fn place(font: &BTreeMap<char, Vec<Group>>, shapes: &[Vec<f32>], value: &str) -> Option<usize> {
    let read: Vec<Option<char>> = shapes.iter().map(|s| loosely(font, s)).collect();
    let n = value.chars().count();
    if read.len() < n {
        return None;
    }
    let fits: Vec<usize> = (0..=read.len() - n)
        .filter(|&at| {
            let pairs = || read[at..at + n].iter().zip(value.chars());
            let unread = pairs().filter(|(r, _)| r.is_none()).count();
            let wrong = pairs().any(|(r, c)| r.is_some_and(|r| r != c));
            // The digit before and after belong to no other number.
            let alone =
                (at == 0 || read[at - 1].is_none()) && read.get(at + n).is_none_or(Option::is_none);
            !wrong && unread <= 1 && unread < n && alone
        })
        .collect();
    match fits.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// `text` drawn white on black with `font`, its ink, and where each letter
/// starts and the last ends (pixels from the left).
fn draw(font: &FontVec, text: &str) -> (Ink, Vec<f32>) {
    let scale = PxScale::from(SIZE);
    let scaled = font.as_scaled(scale);
    let advance = |c: char| scaled.h_advance(font.glyph_id(c));
    let width = (text.chars().map(advance).sum::<f32>() + 2.0 * SIZE) as u32;
    let height = (SIZE * 1.5) as u32;
    let mut cover = vec![0.0f32; (width * height) as usize];
    let mut pen = SIZE;
    let mut pens = vec![pen];
    for c in text.chars() {
        let placed = font
            .glyph_id(c)
            .with_scale_and_position(scale, ab_glyph::point(pen, SIZE * 0.2 + scaled.ascent()));
        if let Some(outline) = font.outline_glyph(placed) {
            let at = outline.px_bounds().min;
            outline.draw(|x, y, v| {
                let (x, y) = (at.x as i64 + i64::from(x), at.y as i64 + i64::from(y));
                if (0..i64::from(width)).contains(&x) && (0..i64::from(height)).contains(&y) {
                    let i = (y * i64::from(width) + x) as usize;
                    cover[i] = cover[i].max(v);
                }
            });
        }
        pen += advance(c);
        pens.push(pen);
    }
    let on = cover.iter().map(|&v| v > 0.5).collect();
    (Ink { width, height, on }, pens)
}

/// The digits as `font` draws them white on navy, cut out in `style`.
fn digits(font: &FontVec, style: Style) -> Vec<Vec<f32>> {
    let (ink, _) = draw(font, "0123456789");
    let picture = RgbImage::from_fn(ink.width, ink.height, |x, y| {
        if ink.on[(y * ink.width + x) as usize] {
            Rgb([255, 255, 255])
        } else {
            Rgb([40, 64, 88])
        }
    });
    glyph::shapes(&picture, (0, 0, ink.width, ink.height), style)
}

/// The word `text` stands for as `font` draws it, its tiles added to `bank`.
fn word(bank: &mut Bank, font: &FontVec, id: i64, text: &str) -> Option<Word> {
    let (ink, pens) = draw(font, text);
    let (left, right, top, bottom) = ink.bounds()?;
    let width = (right - left) as f32;
    let bounds: Vec<u8> = pens
        .iter()
        .map(|&p| (((p - left as f32) / width).clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect();
    let mut word = Word {
        id,
        text: text.to_owned(),
        aspect: round(width / (bottom - top) as f32),
        bounds,
        tiles: Vec::new(),
    };
    word.tiles = ink
        .letters(&word)
        .into_iter()
        .map(|t| bank.add(t))
        .collect();
    Some(word)
}

/// The word of a shop's menu entry as `ink` draws it: cut into as many even
/// parts as it is wide over its height.
fn entry(bank: &mut Bank, id: i64, text: &str, ink: &Ink) -> Option<Word> {
    let (left, right, top, bottom) = ink.bounds()?;
    let aspect = round((right - left) as f32 / (bottom - top) as f32);
    let parts = aspect.round().max(1.0) as usize;
    let bounds = (0..=parts)
        .map(|i| (i * 255 / parts) as u8)
        .collect::<Vec<u8>>();
    let mut word = Word {
        id,
        text: text.to_owned(),
        aspect,
        bounds,
        tiles: Vec::new(),
    };
    word.tiles = ink
        .letters(&word)
        .into_iter()
        .map(|t| bank.add(t))
        .collect();
    Some(word)
}

/// `value` to a thousandth, as the index keeps it.
fn round(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    serde_json::from_str(&std::fs::read_to_string(path).expect("the file reads"))
        .expect("the JSON parses")
}

/// A card of a tuning shop: its fixture, the style of the digits (`"bar"` or
/// `"price"`), their shapes, and the values its fixture's labels hold.
struct Card {
    path: String,
    style: &'static str,
    shapes: Vec<Vec<f32>>,
    values: Vec<String>,
}

/// The cards of the tuning shops (see `Card`).
fn cards(shops: &[ShopLabel], open: &dyn Fn(&str) -> RgbImage) -> Vec<Card> {
    let mut cards = Vec::new();
    for label in shops.iter().filter(|l| l.role == "tune") {
        let image = open(&label.image);
        let values = |left: bool| -> Vec<String> {
            let mut v: Vec<String> = label
                .offers
                .iter()
                .map(|o| if left { o.left } else { o.price }.to_string())
                .collect();
            v.sort();
            v.dedup();
            v
        };
        for bar in shop::bars(&image) {
            if shop::card_area(&image, &bar).is_none() {
                continue;
            }
            let path = label.image.clone();
            cards.push(Card {
                path: path.clone(),
                style: "bar",
                shapes: shop::bar_shapes(&image, &bar),
                values: values(true),
            });
            cards.push(Card {
                path,
                style: "price",
                shapes: shop::price_shapes(&image, &bar).unwrap_or_default(),
                values: values(false),
            });
        }
    }
    cards
}

/// Adds the digits of the `cards` as the game draws them: each card read
/// loosely (see `place`), again while a round takes more. Where each card's
/// value was found among its shapes, `(value, at)`.
fn take_card_digits(cuts: &mut Cuts, cards: &[Card]) -> Vec<Option<(String, usize)>> {
    let mut done: Vec<Option<(String, usize)>> = vec![None; cards.len()];
    loop {
        let mut taken = Cuts::new();
        for (card, c) in cards.iter().enumerate() {
            if done[card].is_some() {
                continue;
            }
            let found: Vec<(&String, usize)> = c
                .values
                .iter()
                .filter_map(|v| place(&cuts[c.style], &c.shapes, v).map(|at| (v, at)))
                .collect();
            if let [(value, at)] = found.as_slice() {
                let n = value.chars().count();
                let shapes = c.shapes[*at..*at + n].to_vec();
                add(&mut taken, &c.path, c.style, shapes, value);
                done[card] = Some(((*value).clone(), *at));
            }
        }
        if taken.is_empty() {
            break;
        }
        for (style, chars) in taken {
            for (c, groups) in chars {
                let known = cuts.entry(style.clone()).or_default().entry(c).or_default();
                for group in groups {
                    join(known, group.mean());
                }
            }
        }
    }
    let read = done.iter().filter(|d| d.is_some()).count();
    println!("cards read for their digits: {read} of {}", cards.len());
    done
}

/// Adds the digits of the `cards` scaled down, at the places the fixtures'
/// own cards (`at_window`, with where their values were found) showed them: a
/// small screen's letters are read too loosely to place them anew. A fixture
/// whose cards cut into other shapes is left out.
fn take_scaled_card_digits(
    cuts: &mut Cuts,
    at_window: &[(Card, Option<(String, usize)>)],
    cards: &[Card],
) {
    let mut taken = 0;
    for path in cards
        .iter()
        .map(|c| &c.path)
        .collect::<std::collections::BTreeSet<_>>()
    {
        let small: Vec<&Card> = cards.iter().filter(|c| c.path == *path).collect();
        let window: Vec<&(Card, Option<(String, usize)>)> =
            at_window.iter().filter(|(c, _)| c.path == *path).collect();
        if small.len() != window.len() {
            continue;
        }
        for (card, (big, found)) in small.into_iter().zip(window) {
            let Some((value, at)) = found else {
                continue;
            };
            let n = value.chars().count();
            // At the window's place, where the shapes there look like the
            // window's digits: past them a smaller screen's letters may cut
            // into other shapes.
            let alike = card.shapes.len() >= at + n
                && (0..n)
                    .all(|i| glyph::distance(&card.shapes[at + i], &big.shapes[at + i]) <= ALIKE);
            if card.style != big.style || !alike {
                continue;
            }
            add(
                cuts,
                &card.path,
                card.style,
                card.shapes[*at..*at + n].to_vec(),
                value,
            );
            // The bar's letters, references of their own; but for the ones
            // beside the number ("と", "回", a "回" near a "0"): a digit told
            // as one of them would leave the number short.
            if card.style == "bar" {
                let letters = cuts
                    .entry(card.style.to_owned())
                    .or_default()
                    .entry(shop::BAR_LETTER)
                    .or_default();
                for (i, shape) in card.shapes.iter().enumerate() {
                    if i + 1 < *at || i > *at + n {
                        join(letters, shape.clone());
                    }
                }
            }
            taken += 1;
        }
    }
    println!(
        "cards taken at the window's places: {taken} of {}",
        cards.len()
    );
}

/// Adds the cards' first digits (see the top): drawn with the fonts, and the
/// credits' digits.
fn font_card_digits(cuts: &mut Cuts, weights: &[FontVec]) {
    for drawn in weights {
        for (style, cut) in [("bar", shop::BAR_STYLE), ("price", shop::PRICE_STYLE)] {
            add(cuts, "the font", style, digits(drawn, cut), "0123456789");
        }
    }
    // The credits' digits, the same font as the cards' though dark on light.
    let dark: Vec<(char, Vec<f32>)> = cuts["digits"]
        .iter()
        .flat_map(|(c, gs)| gs.iter().map(move |g| (*c, g.mean())))
        .collect();
    for style in ["bar", "price"] {
        for (c, mean) in &dark {
            let known = cuts
                .entry(style.to_owned())
                .or_default()
                .entry(*c)
                .or_default();
            join(known, mean.clone());
        }
    }
}

/// Adds the students' names drawn with `font`.
fn names(bank: &mut Bank, font: &FontVec) {
    // A name two ids share is one word, under the lower id.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let students = aoi::student_names(&root.join("data")).expect("data/ lists the students");
    let mut lowest: BTreeMap<&str, i64> = BTreeMap::new();
    for (id, name) in &students {
        let at = lowest.entry(name).or_insert(*id);
        *at = (*at).min(*id);
    }
    for (text, id) in lowest {
        let word = word(bank, font, id, text).expect("every student's name draws some ink");
        bank.index.names.push(word);
    }
}

/// Adds the letters of the students' names the tuning screens and name plates
/// draw.
fn letters(
    bank: &mut Bank,
    labels: &[Label],
    plates: &[NameLabel],
    open: &dyn Fn(&str) -> RgbImage,
) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let students = aoi::student_names(&root.join("data")).expect("data/ lists the students");
    let tuned = labels
        .iter()
        .filter(|l| l.role == "tune")
        .map(|l| (&l.image, l.id))
        .chain(
            plates
                .iter()
                .filter(|p| p.role == "tune")
                .map(|p| (&p.image, p.id)),
        );
    for (image, id) in tuned {
        let name = &students
            .iter()
            .find(|s| s.0 == id)
            .expect("a tune label names a student in data/")
            .1;
        let words = &bank.index.names;
        let word = words
            .iter()
            .find(|w| w.text == *name)
            .cloned()
            .expect("names() added a word for every student's name");
        let Some(ink) = student::name::plate(&open(image)) else {
            continue;
        };
        for (c, tile) in word.text.chars().zip(ink.letters(&word)) {
            let at = bank.add(tile);
            bank.index.letters.entry(c).or_default().push(at);
        }
    }
}

/// Adds the text styles of the student screen.
fn student_styles(
    cuts: &mut Cuts,
    labels: &[Label],
    levels: &[LevelLabel],
    open: &dyn Fn(&str) -> RgbImage,
) {
    for label in labels.iter().filter(|l| l.role == "tune") {
        let image = open(&label.image);
        for (style, area, text) in label.texts() {
            if let Some(shapes) = student::shapes(&image, style, area) {
                add(cuts, &label.image, style, shapes, &text);
            }
        }
    }
    for level in levels.iter().filter(|l| l.role == "tune") {
        if let Some(shapes) = student::shapes(&open(&level.image), "level", student::LEVEL) {
            let text = format!("Lv{}", level.level);
            add(cuts, &level.image, "level", shapes, &text);
        }
    }
}

/// `image` as a phone streams it `width` wide: each pixel sampled between its
/// four nearest (the phone's GPU scales it so, unlike `resize`, which
/// averages all it covers), its colour then halved (H.264's 4:2:0). Of the
/// ways tried, nearest a Pixel 9's stream of its own screens (measured on
/// 2026-10-24: 3.7 to 5.5 off per channel, scaled smoothly 4.5 to 6.0).
fn streamed(image: &RgbImage, width: u32, (dx, dy): (f32, f32)) -> RgbImage {
    let height = (image.height() as f32 * width as f32 / image.width() as f32).round() as u32;
    let (sx, sy) = (
        image.width() as f32 / width as f32,
        image.height() as f32 / height as f32,
    );
    let sampled = RgbImage::from_fn(width, height, |x, y| {
        let fx = ((x as f32 + 0.5) * sx - 0.5 + dx).max(0.0);
        let fy = ((y as f32 + 0.5) * sy - 0.5 + dy).max(0.0);
        let (x0, y0) = (
            (fx as u32).min(image.width() - 1),
            (fy as u32).min(image.height() - 1),
        );
        let (x1, y1) = (
            (x0 + 1).min(image.width() - 1),
            (y0 + 1).min(image.height() - 1),
        );
        let (ax, ay) = (fx - x0 as f32, fy - y0 as f32);
        Rgb(std::array::from_fn(|c| {
            let p = |x, y| f32::from(image.get_pixel(x, y).0[c]);
            let top = p(x0, y0) * (1.0 - ax) + p(x1, y0) * ax;
            let bottom = p(x0, y1) * (1.0 - ax) + p(x1, y1) * ax;
            (top * (1.0 - ay) + bottom * ay).round() as u8
        }))
    });
    chroma_halved(&sampled)
}

/// Adds the student screen's texts as a phone's stream shows them (see
/// `STREAMS`), for the bank of `width`.
fn stream_styles(cuts: &mut Cuts, labels: &[Label], levels: &[LevelLabel], dir: &Path, width: u32) {
    let (scale_down, streams) = STREAMS
        .iter()
        .find(|(w, _, _)| *w == width)
        .map_or((1.0, &[][..]), |s| (s.1, s.2));
    for (&at, &phase) in streams
        .iter()
        .flat_map(|at| PHASES.iter().map(move |p| (at, p)))
    {
        let open = move |path: &str| {
            let image = image::open(dir.join(path))
                .expect("the image opens")
                .to_rgb8();
            let screen = (at as f32 / scale_down).round() as u32;
            let scale = screen as f32 / screen::drawn_width(&image);
            let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
            let image = resize(
                &image,
                w.round() as u32,
                h.round() as u32,
                FilterType::Triangle,
            );
            let width = (image.width() as f32 * scale_down).round() as u32;
            let image = streamed(&image, width, phase);
            screen::normalize(&image).into_owned()
        };
        student_styles(cuts, labels, levels, &open);
    }
}

/// `image` with each 2 by 2 block's colour (Cb, Cr) averaged, its
/// brightness kept.
fn chroma_halved(image: &RgbImage) -> RgbImage {
    let (width, height) = image.dimensions();
    let ycc = |p: &Rgb<u8>| {
        let [red, green, blue] = p.0.map(f32::from);
        let luma = 0.299 * red + 0.587 * green + 0.114 * blue;
        (luma, blue - luma, red - luma)
    };
    let mut out = image.clone();
    for by in (0..height).step_by(2) {
        for bx in (0..width).step_by(2) {
            let block: Vec<(u32, u32)> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .iter()
                .map(|(dx, dy)| (bx + dx, by + dy))
                .filter(|&(x, y)| x < width && y < height)
                .collect();
            let n = block.len() as f32;
            let (cb, cr) = block.iter().fold((0.0, 0.0), |(cb, cr), &(x, y)| {
                let (_, blue, red) = ycc(image.get_pixel(x, y));
                (cb + blue / n, cr + red / n)
            });
            for &(x, y) in &block {
                let (luma, _, _) = ycc(image.get_pixel(x, y));
                let (red, blue) = (luma + cr, luma + cb);
                let green = (luma - 0.299 * red - 0.114 * blue) / 0.587;
                let rgb = [red, green, blue].map(|v| v.round().clamp(0.0, 255.0) as u8);
                out.put_pixel(x, y, Rgb(rgb));
            }
        }
    }
    out
}

/// Adds the dark digits: the credits, and the coins held over the tuning
/// shops; and the shops' names as their menu entries draw them.
fn dark_digits_and_shops(
    cuts: &mut Cuts,
    bank: &mut Bank,
    credits: &[CreditsLabel],
    shops: &[ShopLabel],
    open: &dyn Fn(&str) -> RgbImage,
) {
    for label in credits.iter().filter(|l| l.role == "tune") {
        let Some(credits) = label.credits else {
            continue;
        };
        if let Some(shapes) = credit::shapes(&open(&label.image)) {
            add(cuts, &label.image, "digits", shapes, &credits.to_string());
        }
    }
    for label in shops.iter().filter(|l| l.role == "tune") {
        let image = open(&label.image);
        let held = label.held.iter().flatten();
        for (&at, held) in shop::coins_at(label.held.as_ref().map_or(0, Vec::len))
            .iter()
            .zip(held)
        {
            let (Some(held), Some(shapes)) = (held, shop::held_shapes(&image, at)) else {
                continue;
            };
            add(cuts, &label.image, "digits", shapes, &held.to_string());
        }
        let id = shop::SHOPS
            .iter()
            .position(|s| *s == label.shop)
            .expect("a shop label names one of shop::SHOPS") as i64;
        if let Some(word) =
            shop::menu_ink(&image).and_then(|ink| entry(bank, id, &label.shop, &ink))
        {
            bank.index.shops.push(word);
        }
    }
}

/// Drops the shop bar's letters as near a digit as `APART`: no reference, as a digit
/// told as one would leave the number short.
fn drop_bar_letters_near_digits(cuts: &mut Cuts) {
    if let Some(bar) = cuts.get_mut("bar") {
        let digits: Vec<Vec<f32>> = bar
            .iter()
            .filter(|(c, _)| **c != shop::BAR_LETTER)
            .flat_map(|(_, gs)| gs.iter().map(Group::mean))
            .collect();
        if let Some(letters) = bar.get_mut(&shop::BAR_LETTER) {
            letters.retain(|g| {
                let mean = g.mean();
                digits.iter().all(|d| glyph::distance(d, &mean) > APART)
            });
        }
    }
}

fn main() {
    let fonts = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: build_glyphs <folder of the fonts>"),
    );
    let weight = |name: &str| {
        let path = fonts.join(format!("NotoSansJP-{name}.otf"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        FontVec::try_from_vec(bytes).expect("the Noto Sans JP file is a font")
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let labels: Vec<Label> = read_json(&dir.join("students.json"));
    let levels: Vec<LevelLabel> = read_json(&dir.join("levels.json"));
    let credits: Vec<CreditsLabel> = read_json(&dir.join("credits.json"));
    let plates: Vec<NameLabel> = read_json(&dir.join("names.json"));
    let shops: Vec<ShopLabel> = read_json(&dir.join("shops.json"));
    let weights = ["Regular", "Medium", "Bold"].map(weight);
    // The fixtures' cards at their own size, and where their values were
    // found among their shapes.
    let at_window: Vec<(Card, Option<(String, usize)>)> = {
        let open = |path: &str| {
            image::open(dir.join(path))
                .expect("the image opens")
                .to_rgb8()
        };
        let (mut cuts, mut bank) = (Cuts::new(), Bank::default());
        dark_digits_and_shops(&mut cuts, &mut bank, &credits, &shops, &open);
        font_card_digits(&mut cuts, &weights);
        let found_cards = cards(&shops, &open);
        let found = take_card_digits(&mut cuts, &found_cards);
        found_cards.into_iter().zip(found).collect()
    };

    // Each width the screens are read at keeps a bank of its own, drawn from
    // the fixtures scaled to it.
    for width in screen::WIDTHS {
        let mut bank = Bank::default();
        names(&mut bank, &weights[0]);
        // The fixtures as a window drawn at `at` wide shows them (smoothly,
        // as the game draws them), scaled down as Aoi reads it.
        let drawn = |at: u32| {
            let dir = &dir;
            move |path: &str| {
                let image = image::open(dir.join(path))
                    .expect("the image opens")
                    .to_rgb8();
                let scale = at as f32 / screen::drawn_width(&image);
                let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
                let image = resize(
                    &image,
                    w.round() as u32,
                    h.round() as u32,
                    FilterType::Triangle,
                );
                screen::normalize(&image).into_owned()
            }
        };
        let open = drawn(width);
        let mut cuts = Cuts::new();
        let between = BETWEEN
            .iter()
            .find(|(w, _)| *w == width)
            .map_or(&[][..], |b| b.1);
        for &at in std::iter::once(&width).chain(between) {
            let open = drawn(at);
            student_styles(&mut cuts, &labels, &levels, &open);
            // The shops' names once, as drawn at the width read.
            let mut words = Bank::default();
            let names_to = if at == width { &mut bank } else { &mut words };
            dark_digits_and_shops(&mut cuts, names_to, &credits, &shops, &open);
            take_scaled_card_digits(&mut cuts, &at_window, &cards(&shops, &open));
        }
        stream_styles(&mut cuts, &labels, &levels, &dir, width);
        font_card_digits(&mut cuts, &weights);
        drop_bar_letters_near_digits(&mut cuts);
        for (style, chars) in &cuts {
            let counts: Vec<String> = chars
                .iter()
                .map(|(c, gs)| format!("{c}:{}", gs.len()))
                .collect();
            println!("{width} {style}: {}", counts.join(" "));
            for (c, gs) in chars {
                let tiles: Vec<usize> =
                    gs.iter().map(|g| bank.add(glyph_tile(&g.mean()))).collect();
                let font = bank.index.fonts.entry(style.clone()).or_default();
                font.entry(*c).or_default().extend(tiles);
            }
        }
        letters(&mut bank, &labels, &plates, &open);
        let (png, index) = bank.encode().expect("the glyph bank encodes");
        let (picture, index_file) = glyph::bank::files(width);
        std::fs::write(root.join("data").join(picture), png).expect("the file is written");
        std::fs::write(root.join("data").join(index_file), index + "\n")
            .expect("the file is written");
        println!(
            "{width}: {} tiles: {} names, {} letters of the game, {} shops",
            bank.tiles.len(),
            bank.index.names.len(),
            bank.index.letters.len(),
            bank.index.shops.len()
        );
    }
}
