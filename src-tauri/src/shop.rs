//! The shop screen: which shop the menu has chosen and, card by card, what it
//! offers, the coin it is paid with and how many more times it can be bought,
//! read by the OCR. What each costs is the shops' terms (`price`).

use std::collections::{BTreeMap, HashMap};

use image::{Rgb, RgbImage, RgbaImage, imageops};
use serde::{Deserialize, Serialize};

use crate::{
    Item,
    grid::Card,
    icon::{IconMatcher, Mask},
    ocr::Ocr,
    price::{Price, Prices},
    student::Area,
    tally::CONFIRM,
};

/// The shops whose offers are kept, as the menu names them (the user's choice
/// of 2026-10-12; 青輝石, カケラ(その他) and 戦術対抗戦 are left out).
pub const SHOPS: [&str; 6] = [
    "総力戦",
    "大決戦",
    "カケラ(文字)",
    "合同火力演習",
    "指名手配",
    "熟達証書",
];

/// Where the parts of the screen lie, in window heights (measured on
/// 2026-10-12 at 2222x1250 over the six shops). The menu down the left, its
/// chosen entry painted navy, the entry's name right of its icon and short of
/// the entry's slanting end (read up to either end, see `Goods::read`):
const MENU_START: f32 = 0.072;
const MENU_ENDS: [f32; 2] = [0.275, 0.30];
const MENU_ROWS: Span = (0.13, 0.80);
/// The cards scroll right of this (from the right edge), between these rows.
const PANEL: f32 = 0.92;
const VIEW: Span = (0.172, 0.84);
/// Each card's navy bar ("あとN回購入可能") is at least this wide; nothing
/// else in the panel is navy that wide.
const BAR_WIDTH: f32 = 0.16;
/// The card's name, from the bar's top.
const NAME: Span = (-0.215, -0.145);
/// The price under the bar, from its bottom, and its coin's icon from the bar's
/// left end.
const PRICE: Span = (0.012, 0.048);
const COIN: Span = (0.02, 0.065);
/// The shop's coins over the cards: the centres of their icons from the right
/// edge, left one first (a shop with one coin shows only the right one). In a
/// shop with two, a card's coin is told by its icon's colour.
const COINS: [f32; 2] = [0.829, 0.552];
const COINS_ROW: f32 = 0.12;
const COIN_SIZE: f32 = 0.04;
/// The icon slot over the bar, drawn like a card of the item list: its centre
/// from the bar's top and its height (fitted on 26 cards whose names read).
const SLOT_CENTRE: f32 = -0.0575;
const SLOT_HEIGHT: f32 = 0.13;
/// The bar covers the slot's bottom, the "xN" count with it.
const SLOT_MASK: Mask = Mask {
    left: 0.0,
    top: 0.8,
};
/// The bar's navy on a card still on sale. A card bought up is darkened
/// whole, its bar to about two thirds of this.
const BAR: [f32; 3] = [40.0, 64.0, 88.0];
const DARKENED: f32 = 0.85;
/// The chosen menu entry's italics lean about this far right per row up (stood
/// upright by either), and its letters are lighter than this on the navy.
const ITALIC: [f32; 2] = [0.22, 0.15];
const LIGHT: u8 = 160;

type Span = (f32, f32);

/// An offer as read: the item, which of the shop's coins pays for it (0 the
/// left or only one, 1 the right) and how many more times it can be bought.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: i64,
    pub coin: u8,
    pub left: u32,
}

/// What a frame of the shop screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub shop: &'static str,
    pub offers: Vec<Offer>,
}

/// Where the shop screen is read besides its cards (`card_area`): the menu and
/// the coins over the cards. A fixture keeps these alone.
pub fn areas() -> Vec<Area> {
    let coins = COIN_SIZE / 2.0;
    vec![
        Area {
            from_right: false,
            x: (0.0, MENU_ENDS[1]),
            y: MENU_ROWS,
        },
        Area {
            from_right: true,
            x: (COINS[0] + coins, COINS[1] - coins),
            y: (COINS_ROW - coins, COINS_ROW + coins),
        },
    ]
}

/// The navy of the chosen menu entry and of the bars, and the darker navy of
/// the bars of cards bought up.
fn navy(c: [u8; 3]) -> bool {
    let [r, g, b] = c;
    r < 60 && g < 85 && (40..110).contains(&b) && b > r + 25
}

/// What tells the goods of a shop apart: their icons, drawn as on the item
/// list's cards, and their names.
pub struct Goods {
    matcher: IconMatcher,
    /// The items each icon draws.
    drawn: Vec<Vec<i64>>,
    names: Names,
}

impl Goods {
    /// The goods among `items`, whose icons `picture` gives by name.
    pub fn new(
        items: &[Item],
        picture: impl Fn(&str) -> Result<RgbaImage, String>,
    ) -> Result<Self, String> {
        let mut icons: Vec<&str> = items.iter().map(|i| i.icon.as_str()).collect();
        icons.sort_unstable();
        icons.dedup();
        let pictures = icons
            .iter()
            .map(|name| picture(name))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            matcher: IconMatcher::new(&pictures, SLOT_MASK),
            drawn: icons
                .iter()
                .map(|name| {
                    items
                        .iter()
                        .filter(|i| i.icon == *name)
                        .map(|i| i.id)
                        .collect()
                })
                .collect(),
            names: Names::new(items),
        })
    }

    /// Reads the shop screen: the chosen shop, when it is one kept, and the
    /// offers of the cards shown whole; `prices` tell how many coins it takes.
    /// Any other screen gives `None`.
    pub fn read(&self, image: &RgbImage, ocr: &Ocr, prices: &Prices) -> Option<Seen> {
        let h = image.height() as f32;
        let (top, bottom) = menu_entry(image)?;
        let bars = bars(image);
        if bars.is_empty() {
            return None;
        }
        // The OCR reads the name in one of a few takes, and a take reads as a
        // kept shop's name or as nothing alike.
        let shop = MENU_ENDS
            .iter()
            .flat_map(|&end| ITALIC.iter().map(move |&lean| (end, lean)))
            .find_map(|(end, lean)| {
                let entry = crop(image, MENU_START * h, top as f32, end * h, bottom as f32)?;
                let menu = normalize(&ocr.read(&upright(&entry, lean)).ok()?);
                SHOPS.into_iter().find(|s| normalize(s) == menu)
            })?;
        let coins = if prices.coins(shop).len() > 1 {
            coin_colours(image)
        } else {
            Vec::new()
        };
        let offers = bars
            .iter()
            .filter_map(|bar| self.offer(image, ocr, bar, &coins))
            .collect();
        Some(Seen { shop, offers })
    }

    /// The offer of the card over `bar`, when the card is shown whole, the item
    /// is told and the times it can still be bought read.
    fn offer(&self, image: &RgbImage, ocr: &Ocr, bar: &Bar, coins: &[[f32; 3]]) -> Option<Offer> {
        let h = image.height() as f32;
        let (left, name_top, right, _) = card_area(image, bar)?;
        let (top, bottom) = (bar.top as f32, bar.bottom as f32);
        let name = crop(image, left, name_top, right, top + NAME.1 * h)?;
        let count = crop(image, left, top - 2.0, right, bottom + 3.0)?;
        let text = ocr
            .read(&stack(&[letters(&name, false), letters(&count, true)]))
            .ok()?;
        let lines: Vec<&str> = text.split('/').collect();
        let at = lines.iter().position(|l| parse_left(l).is_some())?;
        let id = decide(
            self.icon(image, bar),
            self.names.find_among(&lines[..at]),
            &self.drawn,
        )?;
        let coin_at = (
            left + f32::midpoint(COIN.0, COIN.1) * h,
            bottom + f32::midpoint(PRICE.0, PRICE.1) * h,
        );
        Some(Offer {
            id,
            coin: if coins.is_empty() {
                0
            } else {
                nearest(&mean(image, coin_at.0, coin_at.1, COIN.1 - COIN.0)?, coins)?
            },
            left: parse_left(lines[at])?,
        })
    }

    /// The icon of the card over `bar` (see `IconMatcher::identify`), the card
    /// brightened back first when it is bought up.
    fn icon(&self, image: &RgbImage, bar: &Bar) -> Result<usize, Vec<usize>> {
        let h = image.height() as f32;
        let all: Vec<usize> = (0..self.drawn.len()).collect();
        let (cx, cy) = (
            (bar.left + bar.right) as f32 / 2.0,
            bar.top as f32 + SLOT_CENTRE * h,
        );
        // The slot with room for the matcher's position search.
        let reach = SLOT_HEIGHT * h;
        let Some(mut around) = crop(image, cx - reach, cy - reach, cx + reach, cy + reach) else {
            return Err(Vec::new());
        };
        let darkening = darkening(image, bar);
        if darkening < DARKENED {
            for p in around.pixels_mut() {
                p.0 = p.0.map(|v| (f32::from(v) / darkening).min(255.0) as u8);
            }
        }
        let height = (SLOT_HEIGHT * h) as u32;
        let card = Card {
            left: reach - height as f32 / 2.0,
            width: height as f32,
            top: (reach - height as f32 / 2.0) as u32,
            height,
        };
        self.matcher.identify(&around, &card, None, &all)
    }
}

/// What the card over `bar` is read from, `(x0, y0, x1, y1)`: from its name to
/// its price, between the bar's ends. `None` where the viewport cuts it.
pub fn card_area(image: &RgbImage, bar: &Bar) -> Option<(f32, f32, f32, f32)> {
    let h = image.height() as f32;
    let top = bar.top as f32 + NAME.0 * h;
    let bottom = bar.bottom as f32 + PRICE.1 * h;
    (top >= VIEW.0 * h && bottom <= VIEW.1 * h).then_some((
        bar.left as f32,
        top,
        bar.right as f32,
        bottom,
    ))
}

/// How much darker than on sale the bar is drawn (1: not at all).
fn darkening(image: &RgbImage, bar: &Bar) -> f32 {
    let navy_pixels: Vec<[u8; 3]> = (bar.top..=bar.bottom)
        .flat_map(|y| (bar.left..bar.right).map(move |x| image.get_pixel(x, y).0))
        .filter(|&c| navy(c))
        .collect();
    let n = navy_pixels.len().max(1) as f32;
    (0..3)
        .map(|c| navy_pixels.iter().map(|p| f32::from(p[c])).sum::<f32>() / n / BAR[c])
        .sum::<f32>()
        / 3.0
}

/// The rows `(top, bottom)` of the chosen menu entry.
fn menu_entry(image: &RgbImage) -> Option<(u32, u32)> {
    let h = image.height() as f32;
    let x1 = (MENU_ENDS[0] * h) as u32;
    // Mostly navy: the icon and the name cover the rest.
    let filled =
        |y: u32| (0..x1).filter(|&x| navy(image.get_pixel(x, y).0)).count() * 2 >= x1 as usize;
    let rows: Vec<u32> = ((MENU_ROWS.0 * h) as u32..(MENU_ROWS.1 * h) as u32)
        .filter(|&y| filled(y))
        .collect();
    let (&top, &bottom) = (rows.first()?, rows.last()?);
    // A single entry, about a twelfth of the window tall.
    ((0.06..0.12).contains(&((bottom - top) as f32 / h))).then_some((top, bottom))
}

/// A card's bar: its left and right ends and its top and bottom rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

/// The cards' bars, row by row. The white letters cut the navy, so a bar is
/// navy runs bridging short gaps, its rows joined across the letters.
pub fn bars(image: &RgbImage) -> Vec<Bar> {
    let (w, h) = (image.width(), image.height() as f32);
    let from = w.saturating_sub((PANEL * h) as u32);
    let width = (BAR_WIDTH * h) as u32;
    let mut out: Vec<Bar> = Vec::new();
    for y in (VIEW.0 * h) as u32..(VIEW.1 * h) as u32 {
        let mut runs: Vec<(u32, u32)> = Vec::new();
        for x in from..w {
            if navy(image.get_pixel(x, y).0) {
                match runs.last_mut() {
                    Some(run) if x - run.1 <= 8 => run.1 = x + 1,
                    _ => runs.push((x, x + 1)),
                }
            }
        }
        for (left, right) in runs.into_iter().filter(|r| r.1 - r.0 >= width) {
            match out
                .iter_mut()
                .find(|b| b.bottom + (0.02 * h) as u32 >= y && b.left.abs_diff(left) < 8)
            {
                Some(bar) => {
                    bar.bottom = y;
                    bar.right = bar.right.max(right);
                }
                None => out.push(Bar {
                    left,
                    right,
                    top: y,
                    bottom: y,
                }),
            }
        }
    }
    out.retain(|b| (b.bottom - b.top) as f32 >= 0.012 * h);
    out
}

fn crop(image: &RgbImage, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<RgbImage> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    (x0 >= 0.0 && y0 >= 0.0 && x1 <= w && y1 <= h && x0 < x1 && y0 < y1).then(|| {
        imageops::crop_imm(
            image,
            x0 as u32,
            y0 as u32,
            (x1 - x0) as u32,
            (y1 - y0) as u32,
        )
        .to_image()
    })
}

fn luminance(c: [u8; 3]) -> u8 {
    ((299 * u32::from(c[0]) + 587 * u32::from(c[1]) + 114 * u32::from(c[2])) / 1000) as u8
}

/// The luminance that best splits `values` in two (Otsu's method): values
/// above it are the light class.
pub fn otsu(values: &[u8]) -> u8 {
    let mut histogram = [0u64; 256];
    for &v in values {
        histogram[usize::from(v)] += 1;
    }
    let total = values.len() as f64;
    let sum: f64 = histogram
        .iter()
        .enumerate()
        .map(|(v, &n)| v as f64 * n as f64)
        .sum();
    let (mut below, mut below_sum, mut best, mut split) = (0.0, 0.0, -1.0, 0u8);
    for (v, &n) in histogram.iter().enumerate() {
        below += n as f64;
        below_sum += v as f64 * n as f64;
        let above = total - below;
        if below == 0.0 || above == 0.0 {
            continue;
        }
        let between = below * above * (below_sum / below - (sum - below_sum) / above).powi(2);
        if between > best {
            best = between;
            split = v as u8;
        }
    }
    split
}

/// The letters of `picture` black on white for the OCR, twice the size, with a
/// margin: the `light` side of its Otsu split, or the dark letters of a name.
/// A name's letters are only its darker half of the dark side: whole, the
/// strokes of the condensed font run together ("術教" read "燉"); measured on
/// 2026-10-12 over 234 names, 125 read as an item this way, 56 with the split.
fn letters(picture: &RgbImage, light: bool) -> RgbImage {
    const MARGIN: u32 = 20;
    let values: Vec<u8> = picture.pixels().map(|p| luminance(p.0)).collect();
    let split = otsu(&values);
    let dark: Vec<f32> = values
        .iter()
        .filter(|&&v| v <= split)
        .map(|&v| f32::from(v))
        .collect();
    let dark_mean = dark.iter().sum::<f32>() / dark.len().max(1) as f32;
    let threshold = if light {
        split
    } else {
        f32::midpoint(dark_mean, f32::from(split)) as u8
    };
    let (w, h) = picture.dimensions();
    let ink = RgbImage::from_fn(w, h, |x, y| {
        let v = luminance(picture.get_pixel(x, y).0);
        if (light && v > threshold) || (!light && v < threshold) {
            Rgb([0, 0, 0])
        } else {
            Rgb([255, 255, 255])
        }
    });
    let big = imageops::resize(&ink, w * 2, h * 2, imageops::FilterType::Triangle);
    let mut out =
        RgbImage::from_pixel(w * 2 + 2 * MARGIN, h * 2 + 2 * MARGIN, Rgb([255, 255, 255]));
    imageops::overlay(&mut out, &big, i64::from(MARGIN), i64::from(MARGIN));
    out
}

/// The chosen menu entry's name, light italics on navy, black on white for the
/// OCR: stood upright, twice the size, with a margin. Leaning, "大決戦" and
/// "熟達証書" read as nothing (2026-10-12).
fn upright(entry: &RgbImage, italic: f32) -> RgbImage {
    const MARGIN: u32 = 30;
    let (w, h) = entry.dimensions();
    let lean = (2.0 * h as f32 * italic) as u32;
    let mut out = RgbImage::from_pixel(
        2 * w + 2 * MARGIN + lean,
        2 * h + 2 * MARGIN,
        Rgb([255, 255, 255]),
    );
    for y in 0..2 * h {
        // Rows higher up lean further right.
        let shift = lean - ((2 * h - y) as f32 * italic) as u32;
        for x in 0..2 * w {
            if luminance(entry.get_pixel(x / 2, y / 2).0) > LIGHT {
                out.put_pixel(x + MARGIN + shift, y + MARGIN, Rgb([0, 0, 0]));
            }
        }
    }
    out
}

/// Pictures one under another, on white.
fn stack(pictures: &[RgbImage]) -> RgbImage {
    let w = pictures.iter().map(RgbImage::width).max().unwrap_or(0);
    let h = pictures.iter().map(RgbImage::height).sum();
    let mut out = RgbImage::from_pixel(w, h, Rgb([255, 255, 255]));
    let mut y = 0;
    for p in pictures {
        imageops::overlay(&mut out, p, 0, i64::from(y));
        y += p.height();
    }
    out
}

/// What lies around a coin's icon: the navy of the price, the light blue of
/// the button, and the white of the coin boxes over the cards.
fn around_coin(c: [u8; 3]) -> bool {
    let [r, g, b] = c;
    navy(c) || (r < 150 && g > 150 && b > 200) || c.iter().all(|&v| v > 200)
}

/// The mean colour of a coin's icon in a square `size` window heights wide
/// centred at `(x, y)`, what lies around it left out (a blue coin and a silver
/// one look alike with it).
fn mean(image: &RgbImage, x: f32, y: f32, size: f32) -> Option<[f32; 3]> {
    let half = size * image.height() as f32 / 2.0;
    let picture = crop(image, x - half, y - half, x + half, y + half)?;
    let coin: Vec<[u8; 3]> = picture
        .pixels()
        .map(|p| p.0)
        .filter(|&c| !around_coin(c))
        .collect();
    let n = coin.len().max(1) as f32;
    Some([0, 1, 2].map(|i| coin.iter().map(|c| f32::from(c[i])).sum::<f32>() / n))
}

/// The mean colours of the shop's coin icons, left one first.
fn coin_colours(image: &RgbImage) -> Vec<[f32; 3]> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    COINS
        .iter()
        .filter_map(|x| mean(image, w - x * h, COINS_ROW * h, COIN_SIZE))
        .collect()
}

/// The index of the colour in `among` nearest to `colour`.
fn nearest(colour: &[f32; 3], among: &[[f32; 3]]) -> Option<u8> {
    let distance = |c: &[f32; 3]| {
        c.iter()
            .zip(colour)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
    };
    among
        .iter()
        .enumerate()
        .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)))
        .and_then(|(i, _)| u8::try_from(i).ok())
}

/// The shop whose offers last; the others' reset as the month changes.
pub const LASTING: &str = "カケラ(文字)";

/// A shop's offers as taken, in the order first seen, and when one was last
/// taken (local time, "2026-10-12 15:30:00").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelf {
    pub read_at: String,
    pub offers: Vec<Offer>,
}

/// Keeps the offers of every shop seen. An offer is taken once the same times
/// left have been read in `CONFIRM` frames in a row.
#[derive(Default)]
pub struct Shelves {
    shelves: BTreeMap<String, Shelf>,
    /// The times left last read for each shop, item and coin, and in how many
    /// frames in a row.
    streaks: HashMap<(&'static str, i64, u8), (u32, u8)>,
}

impl Shelves {
    /// Shelves starting from those kept from before.
    pub fn seeded(shelves: BTreeMap<String, Shelf>) -> Self {
        Self {
            shelves,
            streaks: HashMap::new(),
        }
    }

    /// Takes one frame's offers, seen at local time `now`; returns whether a
    /// shelf changed. A shop's lineup is the month's: the first offer taken in
    /// a new month clears what an earlier month left, but in the `LASTING` shop.
    pub fn observe(&mut self, seen: &Seen, now: &str) -> bool {
        let mut changed = false;
        for offer in &seen.offers {
            let streak = self
                .streaks
                .entry((seen.shop, offer.id, offer.coin))
                .or_insert((offer.left, 0));
            if streak.0 == offer.left {
                streak.1 = streak.1.saturating_add(1);
            } else {
                *streak = (offer.left, 1);
            }
            if streak.1 < CONFIRM {
                continue;
            }
            let stock = self.shelves.entry(seen.shop.to_owned()).or_default();
            if seen.shop != LASTING && month(&stock.read_at) != month(now) {
                stock.offers.clear();
            }
            let known = stock
                .offers
                .iter()
                .position(|o| o.id == offer.id && o.coin == offer.coin);
            match known {
                Some(i) if stock.offers[i] == *offer => continue,
                Some(i) => stock.offers[i] = *offer,
                None => stock.offers.push(*offer),
            }
            now.clone_into(&mut stock.read_at);
            changed = true;
        }
        changed
    }

    /// The shelves by shop.
    pub fn shelves(&self) -> &BTreeMap<String, Shelf> {
        &self.shelves
    }
}

/// A shop's shelf as the window and the export show it: its coins by name
/// (left first), whether its offers reset as the month changes, and its goods
/// in id order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShopRow {
    pub name: String,
    pub read_at: String,
    pub resets: bool,
    pub coins: Vec<String>,
    pub goods: Vec<GoodRow>,
}

/// A good of a shop, named, with what each coin buys of it (`None`: that
/// coin does not).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoodRow {
    pub id: i64,
    pub name: String,
    pub offers: Vec<Option<OfferRow>>,
}

/// How many more times a coin buys a good, and on what terms (`None`: none
/// in the shops' terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OfferRow {
    pub left: u32,
    pub price: Option<Price>,
}

impl ShopRow {
    /// The shelf the row shows.
    pub fn shelf(&self) -> Shelf {
        Shelf {
            read_at: self.read_at.clone(),
            offers: self
                .goods
                .iter()
                .flat_map(|g| {
                    g.offers.iter().zip(0..).filter_map(|(o, coin)| {
                        o.map(|o| Offer {
                            id: g.id,
                            coin,
                            left: o.left,
                        })
                    })
                })
                .collect(),
        }
    }
}

/// The goods on `offers`, in id order, each with the offer of every coin; there
/// are `coins` coins or as many as the offers name.
pub fn goods(offers: &[Offer], coins: usize) -> Vec<(i64, Vec<Option<Offer>>)> {
    let width = offers
        .iter()
        .map(|o| usize::from(o.coin) + 1)
        .max()
        .unwrap_or(0)
        .max(coins);
    let mut by_id: BTreeMap<i64, Vec<Option<Offer>>> = BTreeMap::new();
    for offer in offers {
        by_id.entry(offer.id).or_insert_with(|| vec![None; width])[usize::from(offer.coin)] =
            Some(*offer);
    }
    by_id.into_iter().collect()
}

/// The year and month of a local time ("2026-10").
fn month(at: &str) -> &str {
    at.get(..7).unwrap_or(at)
}

/// The item a card shows, from its icon (`Ok` the icon, `Err` the icons it
/// could be) and the item its name reads as; icons index `drawn`, the items
/// each draws. When the two disagree, or neither tells one item, none.
pub fn decide(
    icon: Result<usize, Vec<usize>>,
    named: Option<i64>,
    drawn: &[Vec<i64>],
) -> Option<i64> {
    match (icon, named) {
        (Ok(icon), Some(id)) => drawn[icon].contains(&id).then_some(id),
        (Ok(icon), None) => match drawn[icon].as_slice() {
            [only] => Some(*only),
            _ => None,
        },
        (Err(icons), Some(id)) if icons.is_empty() => Some(id),
        (Err(icons), Some(id)) => icons.iter().any(|&i| drawn[i].contains(&id)).then_some(id),
        (Err(_), None) => None,
    }
}

/// Spelling the OCR cannot keep apart, made one: the voiced marks it drops
/// ("メダル" read "メタル"), small kana it reads large and the other way, the
/// widths of brackets, letters and digits, and spaces.
pub fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(plain)
        .collect()
}

/// Voiced kana, small kana and the kana each stands for.
const KANA: [(&str, &str); 2] = [
    (
        "ガギグゲゴザジズゼゾダヂヅデドバビブベボパピプペポヴがぎぐげござじずぜぞだぢづでどばびぶべぼぱぴぷぺぽ",
        "カキクケコサシスセソタチツテトハヒフヘホハヒフヘホウかきくけこさしすせそたちつてとはひふへほはひふへほ",
    ),
    (
        "ァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎ",
        "アイウエオツヤユヨワカケあいうえおつやゆよわ",
    ),
];

fn plain(c: char) -> char {
    match c {
        // Full-width letters, digits and brackets are the ASCII ones shifted.
        '！'..='～' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
        _ => KANA
            .iter()
            .find_map(|(from, to)| from.chars().zip(to.chars()).find(|&(f, _)| f == c))
            .map_or(c, |(_, t)| t),
    }
}

/// "あと15回購入可能": how many more times the offer can be bought.
pub fn parse_left(text: &str) -> Option<u32> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let digits = text.strip_prefix("あと")?.strip_suffix("回購入可能")?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// Item ids by their normalized names; a name two items share tells none.
pub struct Names(HashMap<String, Option<i64>>);

impl Names {
    pub fn new(items: &[Item]) -> Self {
        let mut names: HashMap<String, Option<i64>> = HashMap::new();
        for item in items {
            names
                .entry(normalize(&item.name))
                .and_modify(|id| {
                    if *id != Some(item.id) {
                        *id = None;
                    }
                })
                .or_insert(Some(item.id));
        }
        Self(names)
    }

    /// The item the OCR's `text` names, its lines run together.
    pub fn find(&self, text: &str) -> Option<i64> {
        self.0
            .get(&normalize(&text.replace('/', "")))
            .copied()
            .flatten()
    }

    /// The item named by the OCR's `lines` run together, or by them without a
    /// stray first or last line (a speck read as "1").
    pub fn find_among(&self, lines: &[&str]) -> Option<i64> {
        let n = lines.len();
        [(0, n), (1, n), (0, n.saturating_sub(1))]
            .into_iter()
            .filter(|(a, b)| a < b)
            .find_map(|(a, b)| self.find(&lines[a..b].concat()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i64, name: &str) -> Item {
        Item {
            id,
            name: name.into(),
            icon: String::new(),
            category: "Material".into(),
            tier: 0,
        }
    }

    #[test]
    fn spellings_the_ocr_confuses_are_one() {
        assert_eq!(
            normalize("破損した古代文明のメダル"),
            normalize("破 損 し た 古 代 文/明 の メ タ ル").replace('/', "")
        );
        assert_eq!(
            normalize("ユズ（メイド）の神名文字"),
            normalize("ュス(メイト)の神名文字")
        );
        assert_eq!(
            normalize("初級戦術教育BD（百鬼夜行）"),
            normalize("初級戦術教育ＢＤ(百鬼夜行)")
        );
        assert_ne!(normalize("黄金の糸"), normalize("黄金の巻糸"));
    }

    const NOW: &str = "2026-10-12 22:00:00";

    fn seen(shop: &'static str, offers: &[Offer]) -> Seen {
        Seen {
            shop,
            offers: offers.to_vec(),
        }
    }

    fn offer(id: i64, coin: u8, left: u32) -> Offer {
        Offer { id, coin, left }
    }

    #[test]
    fn an_offer_is_taken_once_frames_agree() {
        let mut shelves = Shelves::default();
        let book = offer(1, 1, 40);
        for _ in 1..CONFIRM {
            assert!(!shelves.observe(&seen("総力戦", &[book]), NOW));
        }
        assert!(shelves.observe(&seen("総力戦", &[book]), NOW));
        assert_eq!(shelves.shelves()["総力戦"].offers, [book]);
        assert_eq!(shelves.shelves()["総力戦"].read_at, NOW);
        // A misread frame changes nothing; the same coin's twin is its own offer.
        assert!(!shelves.observe(&seen("総力戦", &[offer(1, 1, 4)]), NOW));
        let twin = offer(1, 0, 5);
        for _ in 0..CONFIRM {
            shelves.observe(&seen("総力戦", &[book, twin]), NOW);
        }
        assert_eq!(shelves.shelves()["総力戦"].offers, [book, twin]);
    }

    #[test]
    fn goods_go_in_id_order_with_an_offer_for_each_coin() {
        let offers = [offer(4000, 1, 40), offer(3000, 0, 12), offer(4000, 0, 16)];
        assert_eq!(
            goods(&offers, 2),
            [
                (3000, vec![Some(offer(3000, 0, 12)), None]),
                (
                    4000,
                    vec![Some(offer(4000, 0, 16)), Some(offer(4000, 1, 40))]
                ),
            ]
        );
        // A shop without terms has as many coins as its offers name.
        assert_eq!(
            goods(&[offer(1, 0, 5)], 0),
            [(1, vec![Some(offer(1, 0, 5))])]
        );
    }

    #[test]
    fn a_new_month_clears_the_shops_that_reset() {
        let last_month = |offers: Vec<Offer>| Shelf {
            read_at: "2026-09-30 23:00:00".to_owned(),
            offers,
        };
        let mut shelves = Shelves::seeded(BTreeMap::from([
            (
                "総力戦".to_owned(),
                last_month(vec![offer(1, 0, 0), offer(2, 0, 0)]),
            ),
            (
                "カケラ(文字)".to_owned(),
                last_month(vec![offer(3, 0, 0), offer(4, 0, 1)]),
            ),
        ]));
        for _ in 0..CONFIRM {
            shelves.observe(&seen("総力戦", &[offer(1, 0, 0)]), NOW);
            shelves.observe(&seen("カケラ(文字)", &[offer(3, 0, 0)]), NOW);
        }
        let shelf = |shop: &str| shelves.shelves()[shop].clone();
        assert_eq!(shelf("総力戦").offers, [offer(1, 0, 0)]);
        assert_eq!(shelf("総力戦").read_at, NOW);
        assert_eq!(
            shelf("カケラ(文字)").offers,
            [offer(3, 0, 0), offer(4, 0, 1)]
        );
    }

    #[test]
    fn the_icon_tells_the_item_and_the_name_settles_look_alikes() {
        // Icon 0 draws item 1, icon 1 item 2, icon 2 both items 3 and 4.
        let drawn = [vec![1], vec![2], vec![3, 4]];
        assert_eq!(decide(Ok(0), None, &drawn), Some(1));
        assert_eq!(decide(Ok(0), Some(1), &drawn), Some(1));
        assert_eq!(decide(Ok(0), Some(2), &drawn), None);
        assert_eq!(decide(Ok(2), None, &drawn), None);
        assert_eq!(decide(Ok(2), Some(4), &drawn), Some(4));
        assert_eq!(decide(Err(vec![0, 1]), Some(2), &drawn), Some(2));
        assert_eq!(decide(Err(vec![0, 1]), Some(3), &drawn), None);
        assert_eq!(decide(Err(vec![0, 1]), None, &drawn), None);
        // An icon SchaleDB draws otherwise matches nothing; the name alone tells.
        assert_eq!(decide(Err(vec![]), Some(3), &drawn), Some(3));
    }

    #[test]
    fn the_times_left_read_only_from_the_whole_phrase() {
        assert_eq!(parse_left("あ と 15 回 購 入 可 能"), Some(15));
        assert_eq!(parse_left("あと0回購入可能"), Some(0));
        assert_eq!(parse_left("あと回購入可能"), None);
        assert_eq!(parse_left("あと1 5回購入"), None);
        assert_eq!(parse_left("15"), None);
    }

    #[test]
    fn a_name_is_found_whole_and_only_when_one_item_bears_it() {
        let names = Names::new(&[
            item(1, "秘伝ノート"),
            item(2, "秘伝ノートの断片"),
            item(3, "ヒビキ（応援団）の神名文字"),
            item(4, "同じ名前"),
            item(5, "同じ名前"),
        ]);
        assert_eq!(names.find("秘 伝 ノ ー ト"), Some(1));
        assert_eq!(names.find("ヒ ヒ キ ( 応 援/団 ) の 神 名 文 字"), Some(3));
        assert_eq!(names.find("伝ノート"), None);
        assert_eq!(names.find("同じ名前"), None);
    }
}
