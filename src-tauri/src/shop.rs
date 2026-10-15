//! The shop screen: which shop the menu has chosen and, card by card, what it
//! offers (told by its icon), the coin it is paid with and how many more times
//! it can be bought. What each costs is the shops' terms (`price`).

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Mutex, PoisonError},
};

use image::{RgbImage, RgbaImage, imageops};
use serde::{Deserialize, Serialize};

use crate::{
    Item, catalog,
    credit::DARK_DIGITS,
    glyph::{
        self, Fonts, Ground, Style,
        bank::Bank,
        word::{self, Ink},
    },
    grid::Card,
    icon::{IconMatcher, Mask},
    price::{Price, Prices},
    quantity::Count,
    screen::{self, Anchor},
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

/// Where the parts of the screen lie, in units (window heights at 16:9, see
/// `screen::unit`; measured on 2026-10-12 at 2222x1250 over the six shops).
/// Rows count from the top, but for the ends of the menu and of the cards'
/// view, which keep to the bottom (see `rows`). The menu down the left, its
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
/// The price under the bar, from its bottom, and its coin's icon and its
/// digits from the card's left and right ends.
const PRICE: Span = (0.012, 0.048);
const COIN: Span = (0.02, 0.065);
const DIGITS: Span = (0.085, -0.045);
/// The shop's coins over the cards: the centres of their icons from the right
/// edge, left one first (a shop with one coin shows only the right one). In a
/// shop with two, a card's coin is told by its icon's colour.
const COINS: [f32; 2] = [0.829, 0.552];
const COINS_ROW: f32 = 0.12;
const COIN_SIZE: f32 = 0.04;
/// How many of each coin are held, written right of its icon: from the icon's
/// centre, and the rows around `COINS_ROW` (measured on 2026-10-13: "530"
/// spans 0.031 to 0.09 right of its icon, the box ends 0.22 right of it).
const HELD: Span = (0.03, 0.2);
const HELD_ROWS: f32 = 0.025;
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
/// The chosen menu entry's letters are lighter than this on the navy.
const LIGHT: u8 = 160;
/// The farthest a part of the chosen menu entry may lie from the shop's name
/// as the tuning fixtures draw it (see `glyph::word`). Measured on 2026-10-14
/// over the shop fixtures: an entry lies within 4 of its own shop's name in a
/// window (a capture the names were not cut from), within 15 on a phone,
/// scaled up to the references (see `screen::normalize`), and 54 or farther
/// from any other's.
const MENU_WORST: f32 = 24.0;
/// Parts of the menu's letters smaller than this share of the window height
/// squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// The letters of a bar lie farther than this from every digit. Measured on
/// 2026-10-14 over the shop fixtures at their own size: 27 or farther; the
/// digits read lie within 10 of their references, one left unread at 11. On
/// 2026-10-15 over them scaled to 1600 wide: the letters 29 or farther, a "0"
/// read as none of the digits at 22 (taken for a letter, "30" read "3", at
/// 20); scaled to 2100 and down to 1920, "589" read "9" at 20.
const LETTER: f32 = 27.0;
/// The light letters of a card's bar ("あと6回購入可能"), on a bar brightened
/// back first when its card is bought up. Only their lightest pixels: with
/// their soft edges the small digits touch ("57" one part; measured on
/// 2026-10-14, above 150 a few touched, above 190 none).
pub const BAR_STYLE: Style = Style {
    ink: |c| luminance(c) > 190,
    tall: 0.5,
    framed: false,
    ground: Ground::Free,
    widest: 1.3,
    width: 1.2,
    skew: 0.0,
    entire: false,
    within: None,
};
/// The white digits of a card's price ("1,800") on the navy, apart from the
/// light blue around; the comma is too short to count.
pub const PRICE_STYLE: Style = Style {
    ink: |c| c.iter().all(|&v| v > 200),
    tall: 0.6,
    framed: false,
    ground: Ground::Free,
    widest: 1.3,
    width: 1.2,
    skew: 0.0,
    entire: false,
    within: None,
};

type Span = (f32, f32);

/// An offer as read: the item, which of the shop's coins pays for it (0 the
/// left or only one, 1 the right) and how many more times it can be bought.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: i64,
    pub coin: u8,
    pub left: u32,
    /// The price on the card, read where the shop's terms leave it to the card.
    #[serde(default)]
    pub price: Option<u32>,
}

/// What a frame of the shop screen shows: the shop, its offers, and how many
/// of each of its coins are held (left one first; `None` not read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub shop: &'static str,
    pub offers: Vec<Offer>,
    pub held: Vec<Option<u64>>,
}

/// Where the shop screen is read besides its cards (`card_area`): the menu and
/// the coins over the cards with their counts. A fixture keeps these alone.
pub fn areas() -> Vec<Area> {
    let coins = COIN_SIZE / 2.0;
    // The menu's end keeps to the bottom (see `rows`): on a screen taller
    // than 16:9 it lies below where its start's edge puts it.
    let menu = |anchor| Area {
        from_right: false,
        x: (0.0, MENU_ENDS[1]),
        y: MENU_ROWS,
        anchor,
    };
    vec![
        menu(Anchor::Top),
        menu(Anchor::Bottom),
        Area {
            from_right: true,
            x: (COINS[0] + coins, COINS[1] - HELD.1),
            y: (COINS_ROW - HELD_ROWS, COINS_ROW + HELD_ROWS),
            anchor: Anchor::Top,
        },
    ]
}

/// The unit the screen is drawn at (see `screen::unit`).
fn unit(image: &RgbImage) -> f32 {
    screen::unit(image.width(), image.height())
}

/// The rows `span` covers, its start kept to the top and its end to the bottom.
fn rows(image: &RgbImage, span: Span) -> (f32, f32) {
    let unit = unit(image);
    (
        span.0 * unit,
        Anchor::Bottom.origin(image.height(), unit) + span.1 * unit,
    )
}

/// The digits of how many are held of the coin whose icon is centred `at`
/// units from the right edge.
pub fn held_shapes(image: &RgbImage, at: f32) -> Option<Vec<Vec<f32>>> {
    let (w, u) = (image.width() as f32, unit(image));
    let x = w - at * u;
    let rect = (
        x + HELD.0 * u,
        (COINS_ROW - HELD_ROWS) * u,
        x + HELD.1 * u,
        (COINS_ROW + HELD_ROWS) * u,
    );
    let inside = rect.0 >= 0.0 && rect.2 <= w && rect.0 < rect.2;
    inside.then(|| {
        let rect = (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32);
        glyph::shapes(image, rect, DARK_DIGITS)
    })
}

/// How many are held of the coin whose icon is centred `at` units from the
/// right edge.
fn held(image: &RgbImage, fonts: &Fonts, at: f32) -> Option<u64> {
    number(&fonts.read("digits", &held_shapes(image, at)?)?)
}

/// The number `digits` spell, when there are some.
fn number(digits: &str) -> Option<u64> {
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

/// Where the shop's coins are shown, from the right edge (see `COINS`), for a
/// shop of `coins` coins.
pub fn coins_at(coins: usize) -> &'static [f32] {
    &COINS[COINS.len() - coins.min(COINS.len())..]
}

/// The navy of the chosen menu entry and of the bars, and the darker navy of
/// the bars of cards bought up.
fn navy(c: [u8; 3]) -> bool {
    let [r, g, b] = c;
    r < 60 && g < 85 && (40..110).contains(&b) && b > r + 25
}

/// What tells the goods of a shop apart: their icons, drawn as on the item
/// list's cards.
pub struct Goods {
    matcher: IconMatcher,
    /// The items each icon draws.
    drawn: Vec<Vec<i64>>,
    /// The last frame's card panel (see `panel`) and what it read: a still
    /// screen's panel stays the same to the pixel, and so does its reading.
    last: Mutex<Option<(Vec<u8>, Option<Seen>)>>,
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
            // A shop's icon slot is drawn as a card of the item list, its
            // colour the item's rarity.
            matcher: IconMatcher::new(&pictures, catalog::rarities(items, &icons), SLOT_MASK),
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
            last: Mutex::new(None),
        })
    }

    /// Reads the shop screen: the chosen shop, when it is one kept, and the
    /// offers of the cards shown whole; `prices` tell how many coins it takes,
    /// the `glyphs` and the `fonts` the shops' names and the digits.
    /// Any other screen gives `None`.
    /// A frame whose card panel is the last one's reads as it did.
    pub fn read(
        &self,
        image: &RgbImage,
        glyphs: &Bank,
        fonts: &Fonts,
        prices: &Prices,
    ) -> Option<Seen> {
        let entry = menu_entry(image)?;
        let bars = bars(image);
        if bars.is_empty() {
            return None;
        }
        let panel = panel(image);
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen_panel, seen)) = last.as_ref()
            && *seen_panel == panel
        {
            return seen.clone();
        }
        let seen = self.read_afresh(image, (glyphs, fonts), prices, entry, &bars);
        *last = Some((panel, seen.clone()));
        seen
    }

    fn read_afresh(
        &self,
        image: &RgbImage,
        (glyphs, fonts): (&Bank, &Fonts),
        prices: &Prices,
        entry: (u32, u32),
        bars: &[Bar],
    ) -> Option<Seen> {
        let ink = entry_ink(image, entry)?;
        let none = BTreeMap::new();
        let named = word::nearest(&ink, &glyphs.index.shops, &glyphs.tiles, &none, MENU_WORST)?;
        let shop = SHOPS.get(usize::try_from(named.id).ok()?).copied()?;
        let coins = if prices.coins(shop).len() > 1 {
            coin_colours(image)
        } else {
            Vec::new()
        };
        let offers = bars
            .iter()
            .filter_map(|bar| self.offer(image, fonts, bar, &coins, (shop, prices)))
            .collect();
        // A shop with one coin shows it where a shop with two shows the right one.
        let held = coins_at(prices.coins(shop).len())
            .iter()
            .map(|&at| held(image, fonts, at))
            .collect();
        Some(Seen { shop, offers, held })
    }

    /// The offer of the card over `bar` in `shop`, when the card is shown
    /// whole, the item is told and the times it can still be bought read; where
    /// the shop's terms leave the price to the card, the price on it too.
    fn offer(
        &self,
        image: &RgbImage,
        fonts: &Fonts,
        bar: &Bar,
        coins: &[[f32; 3]],
        (shop, prices): (&str, &Prices),
    ) -> Option<Offer> {
        let u = unit(image);
        let (left, _, _, _) = card_area(image, bar)?;
        let bottom = bar.bottom as f32;
        let times = times_left(fonts, &bar_shapes(image, bar), LETTER)?;
        let shown =
            || u32::try_from(number(&fonts.read("price", &price_shapes(image, bar)?)?)?).ok();
        let price = if prices.reads_price(shop) {
            Some(shown()?)
        } else {
            None
        };
        let coin_at = (
            left + f32::midpoint(COIN.0, COIN.1) * u,
            bottom + f32::midpoint(PRICE.0, PRICE.1) * u,
        );
        let coin = if coins.is_empty() {
            0
        } else {
            nearest(&mean(image, coin_at.0, coin_at.1, COIN.1 - COIN.0)?, coins)?
        };
        let shown = price.or_else(shown);
        let sold_at =
            |id| shown.is_some() && prices.find(shop, id, coin, None).map(|p| p.price) == shown;
        let id = decide(self.icon(image, bar), &self.drawn, sold_at)?;
        Some(Offer {
            id,
            coin,
            left: times,
            price,
        })
    }

    /// The icon of the card over `bar` (see `IconMatcher::identify`), the card
    /// brightened back first when it is bought up.
    fn icon(&self, image: &RgbImage, bar: &Bar) -> Result<usize, Vec<usize>> {
        let u = unit(image);
        let all: Vec<usize> = (0..self.drawn.len()).collect();
        let (cx, cy) = (
            (bar.left + bar.right) as f32 / 2.0,
            bar.top as f32 + SLOT_CENTRE * u,
        );
        // The slot with room for the matcher's position search.
        let reach = SLOT_HEIGHT * u;
        let Some(mut around) = crop(image, cx - reach, cy - reach, cx + reach, cy + reach) else {
            return Err(Vec::new());
        };
        let darkening = darkening(image, bar);
        if darkening < DARKENED {
            for p in around.pixels_mut() {
                p.0 = p.0.map(|v| (f32::from(v) / darkening).min(255.0) as u8);
            }
        }
        let height = (SLOT_HEIGHT * u) as u32;
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
    let u = unit(image);
    let top = bar.top as f32 + NAME.0 * u;
    let bottom = bar.bottom as f32 + PRICE.1 * u;
    let (first, last) = rows(image, VIEW);
    (top >= first && bottom <= last).then_some((bar.left as f32, top, bar.right as f32, bottom))
}

/// The letters of the chosen menu entry, when the menu shows one.
pub fn menu_ink(image: &RgbImage) -> Option<Ink> {
    entry_ink(image, menu_entry(image)?)
}

/// The letters of the chosen menu entry, whose rows are `(top, bottom)`.
fn entry_ink(image: &RgbImage, (top, bottom): (u32, u32)) -> Option<Ink> {
    let u = unit(image);
    let (left, right) = ((MENU_START * u) as u32, (MENU_ENDS[1] * u) as u32);
    if right > image.width() {
        return None;
    }
    let least = (SPECK * u * u).round().max(1.0) as usize;
    Some(Ink::parts(
        image,
        (left, top, right, bottom + 1),
        |c| luminance(c) > LIGHT,
        least,
    ))
}

/// The letters of the bar of the card over `bar` ("あと6回購入可能"),
/// brightened back first when the card is bought up.
pub fn bar_shapes(image: &RgbImage, bar: &Bar) -> Vec<Vec<f32>> {
    let mut picture = imageops::crop_imm(
        image,
        bar.left,
        bar.top,
        bar.right - bar.left,
        bar.bottom - bar.top + 1,
    )
    .to_image();
    let darkening = darkening(image, bar);
    if darkening < DARKENED {
        for p in picture.pixels_mut() {
            p.0 = p.0.map(|v| (f32::from(v) / darkening).min(255.0) as u8);
        }
    }
    let (w, h) = picture.dimensions();
    glyph::shapes(&picture, (0, 0, w, h), BAR_STYLE)
}

/// The references of a bar's letters ("あと…回購入可能", in pieces) are kept
/// under this character among its digits'.
pub const BAR_LETTER: char = '#';

/// How many more times the card's bar says it can be bought: the digits among
/// its letters (told as `BAR_LETTER`). A part told as neither but within
/// `letter` of a digit leaves it unread, as a number read short would be
/// wrong; so does a digit apart from the number's run of them (a letter told
/// as one).
pub fn times_left(fonts: &Fonts, shapes: &[Vec<f32>], letter: f32) -> Option<u32> {
    let near_digit = |shape: &[f32]| {
        fonts
            .0
            .get("bar")
            .into_iter()
            .flatten()
            .filter(|(c, _)| **c != BAR_LETTER)
            .flat_map(|(_, refs)| refs)
            .any(|r| glyph::distance_within(r, shape, letter) <= letter)
    };
    let mut digits = String::new();
    let mut ended = false;
    for shape in shapes {
        match fonts.character("bar", shape) {
            Some(BAR_LETTER) => ended = !digits.is_empty(),
            Some(_) if ended => return None,
            Some(c) => digits.push(c),
            None if near_digit(shape) => return None,
            None => ended = !digits.is_empty(),
        }
    }
    u32::try_from(number(&digits)?).ok()
}

/// The digits of the price under the bar of the card over `bar`.
pub fn price_shapes(image: &RgbImage, bar: &Bar) -> Option<Vec<Vec<f32>>> {
    let u = unit(image);
    let (left, _, right, price_bottom) = card_area(image, bar)?;
    let rect = (
        left + DIGITS.0 * u,
        bar.bottom as f32 + PRICE.0 * u,
        right + DIGITS.1 * u,
        price_bottom,
    );
    (rect.0 < rect.2).then(|| {
        let rect = (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32);
        glyph::shapes(image, rect, PRICE_STYLE)
    })
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
    let u = unit(image);
    let x1 = (MENU_ENDS[0] * u) as u32;
    // Mostly navy: the icon and the name cover the rest.
    let filled =
        |y: u32| (0..x1).filter(|&x| navy(image.get_pixel(x, y).0)).count() * 2 >= x1 as usize;
    let (first, last) = rows(image, MENU_ROWS);
    let navy_rows: Vec<u32> = (first as u32..last as u32).filter(|&y| filled(y)).collect();
    let (&top, &bottom) = (navy_rows.first()?, navy_rows.last()?);
    // A single entry, about a twelfth of a unit tall.
    ((0.06..0.12).contains(&((bottom - top) as f32 / u))).then_some((top, bottom))
}

/// The card panel with the coins over it, row by row: all a shop is read from
/// but the menu, which moves on a still screen. Another shop chosen shows
/// other cards and coins.
fn panel(image: &RgbImage) -> Vec<u8> {
    let w = image.width() as usize;
    let from = w.saturating_sub((PANEL * unit(image)) as usize) * 3;
    image
        .as_raw()
        .chunks_exact(w * 3)
        .take(rows(image, VIEW).1 as usize)
        .flat_map(|row| &row[from..])
        .copied()
        .collect()
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
    let (w, u) = (image.width(), unit(image));
    let from = w.saturating_sub((PANEL * u) as u32);
    let width = (BAR_WIDTH * u) as u32;
    let (first, last) = rows(image, VIEW);
    let mut out: Vec<Bar> = Vec::new();
    for y in first as u32..last as u32 {
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
                .find(|b| b.bottom + (0.02 * u) as u32 >= y && b.left.abs_diff(left) < 8)
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
    // A bar's rows of letters are navy for the most part: a bar found too
    // short (a margin row a smaller screen blurred short) reaches through
    // them.
    let navy_share = |bar: &Bar, y: u32| {
        let n = (bar.left..bar.right)
            .filter(|&x| navy(image.get_pixel(x, y).0))
            .count();
        n as f32 / (bar.right - bar.left) as f32
    };
    let short = |b: &Bar| ((b.bottom - b.top) as f32) < 0.012 * u;
    for bar in out.iter_mut().filter(|b| short(b)) {
        while bar.top > 0 && navy_share(bar, bar.top - 1) >= BAR_ROW {
            bar.top -= 1;
        }
        while bar.bottom + 1 < image.height() && navy_share(bar, bar.bottom + 1) >= BAR_ROW {
            bar.bottom += 1;
        }
    }
    out.retain(|b| !short(b));
    out
}

/// A row of a bar's letters is at least this share navy (measured on
/// 2026-10-15 over the shop fixtures drawn 1760 wide and scaled down to 1600:
/// 0.41 or more, the rows around the bar 0.05 or less).
const BAR_ROW: f32 = 0.3;

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

/// What lies around a coin's icon: the navy of the price, the light blue of
/// the button, and the white of the coin boxes over the cards.
fn around_coin(c: [u8; 3]) -> bool {
    let [r, g, b] = c;
    navy(c) || (r < 150 && g > 150 && b > 200) || c.iter().all(|&v| v > 200)
}

/// The mean colour of a coin's icon in a square `size` units wide
/// centred at `(x, y)`, what lies around it left out (a blue coin and a silver
/// one look alike with it).
fn mean(image: &RgbImage, x: f32, y: f32, size: f32) -> Option<[f32; 3]> {
    let half = size * unit(image) / 2.0;
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
    let (w, u) = (image.width() as f32, unit(image));
    COINS
        .iter()
        .filter_map(|x| mean(image, w - x * u, COINS_ROW * u, COIN_SIZE))
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

/// The ids the shops' goods the item list lacks take, past the items': an
/// entry of the equipment list's (チタン撃針) or a currency's (クレジットポイント)
/// own id added to these.
pub const EQUIPMENT_GOODS: i64 = 1_000_000_000;
pub const CURRENCY_GOODS: i64 = 2_000_000_000;

/// The shop whose offers last; the others' reset as the month changes.
pub const LASTING: &str = "カケラ(文字)";

/// A shop's offers as taken, in the order first seen, and when one was last
/// taken (local time, "2026-10-12 15:30:00").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelf {
    pub read_at: String,
    pub offers: Vec<Offer>,
    /// How many of each coin are held (as `Seen::held`), taken as offers are.
    #[serde(default)]
    pub held: Vec<Option<u64>>,
}

/// Keeps the offers of every shop seen. An offer is taken once the same times
/// left have been read in `CONFIRM` frames in a row.
#[derive(Default)]
pub struct Shelves {
    by_shop: BTreeMap<String, Shelf>,
    /// The times left last read for each shop, item and coin, and in how many
    /// frames in a row.
    streaks: HashMap<(&'static str, i64, u8), (u32, u8)>,
    /// The same for the coins held, by shop and coin.
    held: HashMap<(&'static str, usize), (u64, u8)>,
    /// The coins held taken since last asked (see `fresh_held`).
    fresh: Vec<(&'static str, usize, u64)>,
}

impl Shelves {
    /// Shelves starting from those kept from before.
    pub fn seeded(shelves: BTreeMap<String, Shelf>) -> Self {
        Self {
            by_shop: shelves,
            streaks: HashMap::new(),
            held: HashMap::new(),
            fresh: Vec::new(),
        }
    }

    /// Takes one frame's offers, seen at local time `now`; returns whether a
    /// shelf changed. A shop's lineup is the month's: the first offer taken in
    /// a new month clears what an earlier month left, but in the `LASTING` shop.
    pub fn observe(&mut self, seen: &Seen, now: &str) -> bool {
        let mut changed = false;
        for (coin, &count) in seen.held.iter().enumerate() {
            let Some(count) = count else { continue };
            let streak = self.held.entry((seen.shop, coin)).or_insert((count, 0));
            if streak.0 == count {
                streak.1 = streak.1.saturating_add(1);
            } else {
                *streak = (count, 1);
            }
            if streak.1 == CONFIRM {
                self.fresh.push((seen.shop, coin, count));
            }
            let stock = self.by_shop.entry(seen.shop.to_owned()).or_default();
            if streak.1 >= CONFIRM && stock.held.get(coin) != Some(&Some(count)) {
                stock
                    .held
                    .resize(seen.held.len().max(stock.held.len()), None);
                stock.held[coin] = Some(count);
                changed = true;
            }
        }
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
            let stock = self.by_shop.entry(seen.shop.to_owned()).or_default();
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

    /// The coins held taken since this was last asked, each once as its
    /// reading is confirmed: the shop, the coin (as `Seen::held`) and the count.
    pub fn fresh_held(&mut self) -> Vec<(&'static str, usize, u64)> {
        std::mem::take(&mut self.fresh)
    }

    /// The shelves by shop.
    pub fn shelves(&self) -> &BTreeMap<String, Shelf> {
        &self.by_shop
    }
}

/// A shop's shelf as the window and the export show it: its coins in id
/// order (the plain coin before the rare, which the screen shows left),
/// whether its offers reset as the month changes, and its goods in id order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShopRow {
    pub name: String,
    pub read_at: String,
    pub resets: bool,
    pub coins: Vec<CoinRow>,
    pub goods: Vec<GoodRow>,
}

/// A coin of a shop: its name, its icon's (empty where SchaleDB has none) and
/// how many are held, the item list's count where the coin is an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoinRow {
    pub name: String,
    pub icon: String,
    pub held: Option<Count>,
    /// The coin as the screen tells it (see `Offer::coin`).
    #[serde(skip)]
    pub at: u8,
}

/// A good of a shop, named, with its icon, what each coin buys of it
/// (`None`: that coin does not) and whether every coin bought it up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoodRow {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub offers: Vec<Option<OfferRow>>,
    pub sold: bool,
}

/// How many more times a coin buys a good, and on what terms (`None`: none
/// in the shops' terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OfferRow {
    pub left: u32,
    pub price: Option<Price>,
    /// The price read on the card (see `Offer::price`), kept with the shelf.
    #[serde(skip)]
    pub shown: Option<u32>,
    /// The coin as the screen tells it (see `Offer::coin`).
    #[serde(skip)]
    pub coin: u8,
}

impl ShopRow {
    /// The shelf the row shows.
    pub fn shelf(&self) -> Shelf {
        let mut held = vec![None; self.coins.len()];
        for coin in &self.coins {
            held[usize::from(coin.at)] = match coin.held {
                Some(Count::Exact(n)) => Some(u64::from(n)),
                _ => None,
            };
        }
        Shelf {
            held,
            read_at: self.read_at.clone(),
            offers: self
                .goods
                .iter()
                .flat_map(|g| {
                    g.offers.iter().flatten().map(|o| Offer {
                        id: g.id,
                        coin: o.coin,
                        left: o.left,
                        price: o.shown,
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
/// could be); icons index `drawn`, the items each draws. Measured on
/// 2026-10-14 over the shop fixtures, the icon alone tells every offer that
/// the names, read by Windows' OCR then, told. An icon not told from others
/// alike (the tiers' blueprint boxes, measured on 2026-10-16) tells the one
/// item among theirs that the shop sells at the card's price (`sold_at`).
pub fn decide(
    icon: Result<usize, Vec<usize>>,
    drawn: &[Vec<i64>],
    sold_at: impl Fn(i64) -> bool,
) -> Option<i64> {
    let could: Vec<i64> = match icon {
        Ok(icon) => match drawn[icon].as_slice() {
            [only] => return Some(*only),
            items => items.to_vec(),
        },
        Err(icons) => icons
            .iter()
            .flat_map(|&i| drawn[i].iter().copied())
            .collect(),
    };
    match could
        .into_iter()
        .filter(|&id| sold_at(id))
        .collect::<Vec<_>>()[..]
    {
        [only] => Some(only),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-12 22:00:00";

    /// The number of a bar is one run of digits: a letter of the bar told as
    /// a digit after it ("あと30回購入可能" read "30" and then "2" of a kanji)
    /// leaves it unread, where it would read "302".
    #[test]
    fn times_left_are_one_run_of_digits() {
        let size = glyph::GLYPH.0 * glyph::GLYPH.1;
        let block = |from: usize| -> Vec<f32> {
            (0..size)
                .map(|i| {
                    if (from..from + 48).contains(&i) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect()
        };
        let mut digits = std::collections::BTreeMap::new();
        for (c, from) in [('3', 0), ('0', 48), ('2', 96)] {
            digits.insert(c, vec![block(from)]);
        }
        let fonts = Fonts([("bar".to_owned(), digits)].into_iter().collect());
        let letter = block(144);
        let (three, zero, two) = (block(0), block(48), block(96));
        let bar = [letter.clone(), three.clone(), zero.clone(), letter.clone()];
        assert_eq!(times_left(&fonts, &bar, 27.0), Some(30));
        let bar = [letter.clone(), three, zero, letter.clone(), two, letter];
        assert_eq!(times_left(&fonts, &bar, 27.0), None);
    }

    /// The bar's letters have references of their own (`BAR_LETTER`): one
    /// told as a letter is no digit, and leaves the number read however near
    /// a digit it lies.
    #[test]
    fn a_bars_letters_told_as_such_leave_its_number() {
        let size = glyph::GLYPH.0 * glyph::GLYPH.1;
        let block = |from: usize, to: usize| -> Vec<f32> {
            (0..size)
                .map(|i| if (from..to).contains(&i) { 1.0 } else { 0.0 })
                .collect()
        };
        let mut refs = std::collections::BTreeMap::new();
        refs.insert('3', vec![block(0, 48)]);
        refs.insert('0', vec![block(48, 96)]);
        // A letter as near the "3" as to be taken for a broken digit without
        // a reference of its own.
        refs.insert(BAR_LETTER, vec![block(10, 48)]);
        let fonts = Fonts([("bar".to_owned(), refs)].into_iter().collect());
        let bar = [block(10, 48), block(0, 48), block(48, 96), block(10, 48)];
        assert_eq!(times_left(&fonts, &bar, 27.0), Some(30));
    }

    fn seen(shop: &'static str, offers: &[Offer]) -> Seen {
        Seen {
            shop,
            offers: offers.to_vec(),
            held: Vec::new(),
        }
    }

    #[test]
    fn the_coins_held_are_taken_once_read_alike_in_a_row() {
        let mut shelves = Shelves::default();
        let held = |counts: &[Option<u64>]| Seen {
            shop: "総力戦",
            offers: Vec::new(),
            held: counts.to_vec(),
        };
        // A misread frame between restarts the count.
        for counts in [[Some(530), None], [Some(580), None], [Some(530), None]] {
            assert!(!shelves.observe(&held(&counts), NOW));
        }
        for _ in 1..CONFIRM - 1 {
            assert!(!shelves.observe(&held(&[Some(530), None]), NOW));
        }
        assert!(shelves.observe(&held(&[Some(530), Some(5940)]), NOW));
        assert_eq!(shelves.shelves()["総力戦"].held, [Some(530), None]);
        // Each count is given out once, as it is taken.
        assert_eq!(shelves.fresh_held(), [("総力戦", 0, 530)]);
        shelves.observe(&held(&[Some(530), Some(5940)]), NOW);
        assert_eq!(shelves.fresh_held(), []);
    }

    fn offer(id: i64, coin: u8, left: u32) -> Offer {
        Offer {
            id,
            coin,
            left,
            price: None,
        }
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
            held: Vec::new(),
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
    fn the_icon_tells_the_item_when_it_draws_one() {
        // Icon 0 draws item 1, icon 2 both items 3 and 4.
        let drawn = [vec![1], vec![2], vec![3, 4]];
        let none = |_: i64| false;
        assert_eq!(decide(Ok(0), &drawn, none), Some(1));
        assert_eq!(decide(Ok(2), &drawn, none), None);
        assert_eq!(decide(Err(vec![0, 1]), &drawn, none), None);
    }

    /// An icon not told from others alike (the tiers' blueprint boxes) tells
    /// the one item among theirs the shop sells at the price the card shows;
    /// none or two such leave it untold.
    #[test]
    fn an_icon_like_others_tells_the_item_sold_at_the_card_price() {
        // Icon 0 draws item 1, icon 1 item 2, icon 2 both items 3 and 4.
        let drawn = [vec![1], vec![2], vec![3, 4]];
        let terms = [(1, 300), (2, 480), (3, 840), (4, 840)];
        let at = |price: u32| move |id: i64| terms.contains(&(id, price));
        assert_eq!(decide(Err(vec![0, 1]), &drawn, at(480)), Some(2));
        assert_eq!(decide(Err(vec![0, 1]), &drawn, at(999)), None);
        assert_eq!(decide(Err(vec![0, 2]), &drawn, at(840)), None);
        assert_eq!(decide(Ok(2), &drawn, at(840)), None);
        // An item of that price the icon cannot be.
        assert_eq!(decide(Err(vec![1, 2]), &drawn, at(300)), None);
    }
}
