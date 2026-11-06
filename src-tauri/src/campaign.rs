//! The campaign the 任務 screen tells left of its middle: the label
//! "キャンペーン中" and the text after it ("任務Normalで獲得できる報酬量が2倍
//! (+100%)になっています。"), each read as a word, as the titles are (see
//! `title`). The text is read up to its rate, which is not fixed: 2倍, 3倍 at
//! the anniversaries (the user's word, 2026-11-06).

use image::RgbImage;

use crate::{
    glyph::{
        bank::Bank,
        word::{self, Ink},
    },
    screen::{self, Anchor},
    student::{self, Area},
};

/// The label a campaign shows.
pub const LABEL: &str = "キャンペーン中";

/// A campaign told by its text's start: the screen whose rewards it raises.
pub struct Text {
    pub screen: &'static str,
    pub start: &'static str,
    /// Where the text is read up to, across, in units: within the rate's
    /// digit, which is left out as it touches the edge.
    pub to: f32,
}

/// The campaigns told. Measured on 2026-11-06 at 2370x1333 (Area 14, 2倍):
/// the text from 0.334 across, "が" ending at 0.712, the rate's "2" from 0.715
/// to 0.727.
pub const TEXTS: [Text; 1] = [Text {
    screen: "任務 Normal",
    start: "任務Normalで獲得できる報酬量が",
    to: 0.721,
}];

/// Where the label is, in units, kept to the top (same screen: its letters
/// from 0.160 to 0.309 across and 0.343 to 0.364 down in a pink box, its
/// dark edge from 0.150 to 0.319 across and 0.337 to 0.374 down). Short of
/// the box's sides, its top and bottom edges cross the area and are left out.
pub const LABEL_AREA: Area = Area {
    from_right: false,
    x: (0.155, 0.314),
    y: (0.333, 0.378),
    anchor: Anchor::Top,
};

/// Where the text's first line is, up to `to` (same screen: its letters from
/// 0.343 to 0.366 down, the second line's from 0.376; the label's box ends at
/// 0.320).
pub const fn text_area(to: f32) -> Area {
    Area {
        from_right: false,
        x: (0.33, to),
        y: (0.338, 0.372),
        anchor: Anchor::Top,
    }
}

/// Parts of the letters smaller than this share of the unit squared are specks.
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// No letter of the label lies farther than this from the word's. Measured on
/// 2026-11-06 over the 任務 fixture drawn from 1500 to 2370 wide by 10: within
/// 16.7 of its own; no other fixture's label area compares at all (its ink as
/// wide as no label's).
const WORST: f32 = 24.0;
/// The text's letters lie within this of the word's on the mean. Its 17
/// letters are cut at equal widths, and as the width changes a cut may take a
/// letter's edge one way and the gap before it the other: on the same sweep
/// one letter came to 33.2 while the mean stayed within 11.4. With a letter or
/// more after "任務" taken out (as "任務Hard" is shorter than "任務Normal"),
/// the mean came to 55 or more.
const TEXT_MEAN: f32 = 24.0;

/// The label's dark pink letters on its light pink box.
fn label_ink(c: [u8; 3]) -> bool {
    student::luminance(c) < 100.0 && c[0] > c[2]
}

/// The text's white letters on the navy band.
fn text_ink(c: [u8; 3]) -> bool {
    student::luminance(c) > 170.0
}

fn ink(image: &RgbImage, area: Area, ink: fn([u8; 3]) -> bool) -> Option<Ink> {
    let u = screen::unit(image.width(), image.height());
    let speck = (SPECK * u * u).round().max(1.0) as usize;
    let ink = Ink::parts(image, student::rect(image, area)?, ink, speck);
    ink.bounds().map(|_| ink)
}

/// The label's letters.
pub fn label(image: &RgbImage) -> Option<Ink> {
    ink(image, LABEL_AREA, label_ink)
}

/// The text's letters up to `to` (see `Text`).
pub fn text(image: &RgbImage, to: f32) -> Option<Ink> {
    ink(image, text_area(to), text_ink)
}

/// The screen whose campaign the 任務 screen shows, among `TEXTS`: the label
/// there and the text's start one of theirs.
pub fn read(image: &RgbImage, glyphs: &Bank) -> Option<&'static str> {
    let none = std::collections::BTreeMap::new();
    let words = &glyphs.index.campaign;
    let labels = words.iter().filter(|w| w.text == LABEL);
    word::nearest(&label(image)?, labels, &glyphs.tiles, &none, WORST)?;
    TEXTS
        .iter()
        .filter_map(|t| {
            let ink = text(image, t.to)?;
            let word = words.iter().find(|w| w.text == t.start)?;
            let (mean, _) = word::compare(&ink, word, &glyphs.tiles, &none)?;
            (mean <= TEXT_MEAN).then_some((mean, t.screen))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, screen)| screen)
}
