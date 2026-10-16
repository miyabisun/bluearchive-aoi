//! Reads a student's training from the student detail screen (the 基本情報 tab).
//!
//! The screen keeps its layout at any 16:9 window size, so each field sits at
//! a fixed place measured in window heights from the left or the right edge.
//! The name is told among the students' names (`name`); the short texts
//! ("Lv.65", "T10", "MAX") are read character by character against references
//! from labeled screenshots (`glyph`). A field that cannot be read for sure is
//! left unknown.

pub mod name;
pub mod roster;

use image::RgbImage;
use serde::{Deserialize, Serialize};

use crate::{
    glyph::{self, Fonts, Ground, Ink, Rect, Style},
    screen::{self, Anchor},
};

/// A student's training as the 基本情報 tab shows it; `None` where a field
/// could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Training {
    pub level: Option<u8>,
    /// Rarity, 1 to 5 stars.
    pub rarity: Option<u8>,
    pub bond: Option<u8>,
    /// EX (up to 5), normal, passive and sub skills (up to 10).
    pub skills: [Option<u8>; 4],
    /// The unique weapon; `Some(None)` while it is locked (under 5 stars).
    /// Left out of the window's data when unknown, so it stays apart from
    /// locked (`null`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub weapon: Option<Option<Weapon>>,
    /// The three equipment slots.
    pub equipment: [Option<Gear>; 3],
    /// The favourite item's tier, 0 when none is equipped.
    pub gear: Option<u8>,
    /// Ability release (能力解放) of HP, attack and healing, 0 when not released.
    pub ability: [Option<u8>; 3],
}

/// A field that is there, `null` or not: `Some` of what it holds (a missing
/// field is `None` by its default).
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weapon {
    pub level: u8,
    pub stars: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gear {
    pub tier: u8,
    pub level: u8,
}

/// A part of the screen in units (window heights at 16:9, see `screen::unit`):
/// across from `x.0` to `x.1` away from the left (or the right) edge, down from
/// `y.0` to `y.1`, kept to `anchor`.
#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub from_right: bool,
    pub x: (f32, f32),
    pub y: (f32, f32),
    pub anchor: Anchor,
}

/// A part of the name plate, at the bottom left.
const fn on_plate(x: (f32, f32), y: (f32, f32)) -> Area {
    Area {
        from_right: false,
        x,
        y,
        anchor: Anchor::Bottom,
    }
}

/// A part of the panel, at the middle right.
const fn on_panel(x: (f32, f32), y: (f32, f32)) -> Area {
    Area {
        from_right: true,
        x,
        y,
        anchor: Anchor::Middle,
    }
}

/// The name plate's row: the bond heart, the name and the rarity stars (the
/// name's own columns are `name::COLUMNS`).
pub const NAME: Area = on_plate((0.04, 0.48), (0.76, 0.83));
const BOND: Area = on_plate((0.0624, 0.0944), (0.786, 0.807));
const RARITY: Area = on_plate((0.364, 0.474), (0.789, 0.818));
/// The level on the name plate, which every tab of the student screen shows.
pub const LEVEL: Area = on_plate((0.048, 0.116), (0.827, 0.856));
/// HP and attack on the first row, healing on the second, each with its
/// ability release ("Lv.25") after the value.
/// Below the "MAX" tag that tops a fully released badge.
const ABILITY: [Area; 3] = [
    on_panel((0.834, 0.498), (0.324, 0.350)),
    on_panel((0.498, 0.258), (0.324, 0.350)),
    on_panel((0.498, 0.258), (0.373, 0.398)),
];
const SKILLS: [Area; 4] = [
    on_panel((0.815, 0.727), (0.558, 0.590)),
    on_panel((0.666, 0.578), (0.558, 0.590)),
    on_panel((0.519, 0.431), (0.558, 0.590)),
    on_panel((0.372, 0.284), (0.558, 0.590)),
];
const SKILL_MAX: [u8; 4] = [5, 10, 10, 10];
const WEAPON_LEVEL: Area = on_panel((0.698, 0.614), (0.629, 0.659));
/// Room for four stars, which stand right-aligned.
const WEAPON_STARS: Area = on_panel((0.400, 0.258), (0.706, 0.740));
/// Up to the "E" badge of the equipped piece, and down past the digits' foot
/// with a few pixels to spare: cut at it (0.797), a smaller screen's "0" lost
/// its foot to the edge, taken for the ground, and "Lv70" read "Lv7".
const EQUIPMENT_LEVEL: [Area; 3] = [
    on_panel((0.822, 0.738), (0.774, 0.802)),
    on_panel((0.690, 0.606), (0.774, 0.802)),
    on_panel((0.558, 0.474), (0.774, 0.802)),
];
/// The tier badges at the bottom left of the pieces, whole with a few pixels
/// to spare (measured on 2026-10-12: the gear's badge spans 0.444 to 0.406
/// from the right, 0.854 to 0.879 down); a badge cut at its edge reads no
/// longer once the capture moves a pixel.
const EQUIPMENT_TIER: [Area; 3] = [
    on_panel((0.834, 0.770), (0.851, 0.883)),
    on_panel((0.705, 0.638), (0.851, 0.883)),
    on_panel((0.576, 0.506), (0.851, 0.883)),
];
/// The favorite item's badge, "T1" or "T2"; its picture, blue in places,
/// stands right of and above it.
const GEAR_TIER: Area = on_panel((0.447, 0.402), (0.851, 0.883));
/// An ability release's slate badge covers at least this share of its `ABILITY`
/// area (measured on 2026-10-15 over the student fixtures, down to 1280 wide:
/// none without one; 0.105 or more with one).
const ABILITY_BADGE: f32 = 0.03;
/// The badge's blue covers at least this share of `GEAR_TIER` where there is a
/// favorite item (measured on 2026-10-14 over the student fixtures: none
/// without one; 0.19 to 0.27 with one, 0.19 on a phone's H.264 video).
const GEAR_BADGE: f32 = 0.05;

/// Every area the screen is read in.
pub fn areas() -> Vec<Area> {
    let mut all = vec![
        NAME,
        BOND,
        RARITY,
        LEVEL,
        WEAPON_LEVEL,
        WEAPON_STARS,
        GEAR_TIER,
    ];
    all.extend(ABILITY);
    all.extend(SKILLS);
    all.extend(EQUIPMENT_LEVEL);
    all.extend(EQUIPMENT_TIER);
    all
}

/// The pixel rectangle of `area` in `image`.
pub fn rect(image: &RgbImage, area: Area) -> Option<Rect> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    let unit = screen::unit(image.width(), image.height());
    let (x0, x1) = if area.from_right {
        (w - area.x.0 * unit, w - area.x.1 * unit)
    } else {
        (area.x.0 * unit, area.x.1 * unit)
    };
    let origin = area.anchor.origin(image.height(), unit);
    let (y0, y1) = (origin + area.y.0 * unit, origin + area.y.1 * unit);
    (x0 >= 0.0 && x1 <= w && y0 >= 0.0 && y1 <= h && x0 < x1)
        .then_some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
}

fn luminance(c: [u8; 3]) -> f32 {
    0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2])
}

/// The light text on the dark name plate ("Lv.65"). Its soft edges are left
/// out, or neighbours of the italic font touch more ("21", "22"). Measured on
/// 2026-10-12 over the level samples: above 150 "21" reads as nothing, above
/// 220 no character is cut out at all; 200 reads every sample.
const LIGHT: Ink = |c| c.iter().all(|&v| v > 200);
/// The dark digits in the bond heart.
const DARK: Ink = |c| luminance(c) < 100.0;
/// The navy skill levels ("Lv.7").
const NAVY: Ink = |c| luminance(c) < 130.0 && c[2] >= c[0];
/// The white fill of the italic levels of the weapon and the equipment, whose
/// outlines touch, by its brightness: enclosed by the outline
/// (`Style::enclosed`), the light card around it is left out, which a video
/// dims as far as the fill. Measured on 2026-10-15 over the student fixtures
/// scaled to 1920 wide and a tablet's stream: 225 reads the most; at 200 the
/// fill joins the card through the outline's light edge, at 235 the stream's
/// dimmed fill is lost.
const LIT: Ink = |c| luminance(c) >= 225.0;
/// The equipment level's fill alone, where `LIT` joins it to the card
/// through gaps in a stream's outline and cuts out nothing: brighter than the
/// card (240 and more against 225 to 240 on a tablet's stream 1920 wide,
/// 2026-10-16). It cuts the fill thinner, which breaks apart: its characters
/// are read only within `CRISP` of their references.
const BRIGHT: Ink = |c| luminance(c) >= 240.0;
/// How near the characters `BRIGHT` cuts lie to their references, at most:
/// within 4.9 where the level is whole, while a piece of a "70" whose "0" was
/// lost came 17.0 near "1" (a stream at 1830 wide, 2026-10-16).
const CRISP: f32 = 8.0;
/// The blue of the tier badges, by which the favourite item's is told.
const BLUE: Ink = |c| i32::from(c[2]) - i32::from(c[0]) > 90 && c[2] > 150;
/// The tier ("T10") and the ring around it, darker than the white badge: by
/// its brightness, which a video keeps where it halves the colour (on a
/// tablet's stream 1920 wide, 100 to 145 against 245 and more).
const TIER: Ink = |c| luminance(c) < 190.0;
/// The slate blue badge of an ability release; the navy of the stats beside
/// it is darker and greyer.
const SLATE: Ink =
    |c| i32::from(c[2]) - i32::from(c[0]) > 60 && (80.0..120.0).contains(&luminance(c));
/// The ability release's yellow text ("Lv.25") by its brightness on the badge
/// (on a tablet's stream 1920 wide, 180 to 227 against the badge's 96).
const ON_SLATE: Ink = |c| luminance(c) > 160.0;
/// The gold "MAX" of a skill at its highest level.
const GOLD: Ink = |c| c[0] > 200 && c[1] > 150 && c[2] < 120;
/// The rarity stars.
const STAR: Ink = |c| c[0] > 220 && c[1] > 150 && c[2] < 100;
/// The unique weapon's stars.
const WEAPON_STAR: Ink = |c| c[2] > 220 && c[1] > 180 && c[0] < 180;
const fn style(
    ink: Ink,
    tall: f32,
    width: f32,
    widest: f32,
    framed: bool,
    ground: Ground,
    skew: f32,
) -> Style {
    Style {
        ink,
        tall,
        framed,
        ground,
        widest,
        width,
        skew,
        entire: false,
        within: None,
        line: true,
    }
}

/// Characters never wider than this share of their height are left whole.
const WHOLE: f32 = 1.2;

/// The text styles by name (`Fonts` holds references under it). The italic
/// font of the level draws neighbours touching ("Lv", "90"): it is stood
/// upright first (a skew of 0.15, measured on 2026-10-12 over the level
/// samples: the evaluation characters lie within 5.0 of their own references
/// and at most 0.19 of the way to the next one, where 0.12 or 0.2 leave 0.8
/// and 0.52, and 0.25 misreads "56"), then cut apart by width. The white
/// italic levels of the weapon and the equipment are read by their fill,
/// which their outlines keep apart.
pub const STYLES: &[(&str, Style)] = &[
    (
        "level",
        style(LIGHT, 0.5, 0.62, f32::INFINITY, false, Ground::Free, 0.15),
    ),
    (
        "bond",
        style(DARK, 0.5, WHOLE, 1.3, false, Ground::Free, 0.0),
    ),
    (
        "skill",
        style(NAVY, 0.5, WHOLE, 1.3, false, Ground::Free, 0.0),
    ),
    (
        "weapon",
        Style {
            ..style(LIT, 0.5, WHOLE, 1.3, false, Ground::Around, 0.0)
        },
    ),
    // The equipment's levels break into pieces on a smaller screen.
    (
        "equipment",
        Style {
            entire: true,
            ..style(LIT, 0.5, WHOLE, 1.3, false, Ground::Around, 0.0)
        },
    ),
    ("tier", style(TIER, 0.5, 0.62, 1.6, true, Ground::Free, 0.0)),
    // The ability's digits touch on a stream ("25" one part): kept, and cut
    // by width.
    (
        "ability",
        Style {
            within: Some(SLATE),
            ..style(ON_SLATE, 0.5, 0.62, 2.6, false, Ground::Around, 0.0)
        },
    ),
];

fn style_of(name: &str) -> Style {
    STYLES
        .iter()
        .find(|(s, _)| *s == name)
        .map(|&(_, s)| s)
        .unwrap()
}

/// The text of `style` in `area`.
fn text(image: &RgbImage, fonts: &Fonts, name: &str, area: Area) -> Option<String> {
    let shapes = glyph::shapes(image, rect(image, area)?, style_of(name));
    if shapes.is_empty() {
        return Some(String::new());
    }
    fonts.read(name, &shapes)
}

/// The number after `prefix` in the text of `style` in `area`.
fn number(image: &RgbImage, fonts: &Fonts, style: &str, prefix: &str, area: Area) -> Option<u8> {
    let rect = rect(image, area)?;
    let shapes = glyph::shapes(image, rect, style_of(style));
    let text = if shapes.is_empty() && style == "equipment" {
        let bright = Style {
            ink: BRIGHT,
            ..style_of(style)
        };
        after_prefix(
            fonts,
            style,
            &glyph::shapes(image, rect, bright),
            prefix,
            CRISP,
        )
    } else {
        after_prefix(fonts, style, &shapes, prefix, f32::INFINITY)
    };
    text?.parse().ok()
}

/// The text after `prefix` that `shapes` spell in `style`: the prefix's last
/// letter nearest to what it is, at its place, anchoring the rest, which is
/// told strictly (see `Fonts::read`); the letters before it need only be
/// there. A stream blurs a level's "L" past what a digit may lie from its
/// references (23 in the middle equipment slot on a tablet's stream 1920
/// wide), or breaks its foot off, while the digits keep to theirs. The last
/// letter is not looked for farther on (a piece of a digit may lie nearest
/// it). The characters lie within `farthest` of their references as well.
fn after_prefix(
    fonts: &Fonts,
    style: &str,
    shapes: &[Vec<f32>],
    prefix: &str,
    farthest: f32,
) -> Option<String> {
    let (head, rest) = shapes.split_at_checked(prefix.chars().count())?;
    let anchored = match (head.last(), prefix.chars().last()) {
        (Some(shape), Some(c)) => fonts.nearest(style, shape) == Some(c),
        _ => true,
    };
    anchored.then(|| fonts.read_within(style, rest, farthest))?
}

/// How many pixels of `ink` `area` holds.
fn count(image: &RgbImage, area: Area, ink: Ink) -> Option<usize> {
    let (x0, y0, x1, y1) = rect(image, area)?;
    Some(
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| ink(image.get_pixel(x, y).0))
            .count(),
    )
}

/// How many stars of `ink` stand in a row in `area`, `pitch` window heights
/// apart: counted from the right one, a place at a time, while the place holds
/// a star's worth of ink. A picture beside the row (Eimi's gun hangs a tag
/// left of her weapon's stars) is never taken for one.
fn stars(image: &RgbImage, area: Area, ink: Ink, pitch: f32) -> Option<u8> {
    let (x0, y0, x1, y1) = rect(image, area)?;
    let inked = |x: u32| (y0..y1).filter(|&y| ink(image.get_pixel(x, y).0)).count();
    let Some(right) = (x0..x1).rev().find(|&x| inked(x) > 0) else {
        return Some(0);
    };
    let step = pitch * screen::unit(image.width(), image.height());
    let star = |k: u8| {
        let end = right as f32 + 1.0 - f32::from(k) * step;
        let start = end - step;
        if start < x0 as f32 {
            return false;
        }
        let ink: usize = (start as u32..end as u32).map(inked).sum();
        ink as f32 >= STAR_SHARE * step * (y1 - y0) as f32
    };
    Some((0..=u8::MAX).take_while(|&k| star(k)).count() as u8)
}

/// Rarity stars stand this far apart (window heights). Measured on
/// 2026-10-12 at 1250 high: 3, 4 and 5 stars span 77, 101 and 124 pixels.
const RARITY_PITCH: f32 = 0.0200;
/// The unique weapon's stars stand this far apart (window heights): 1 to 4
/// stars span 31, 59, 88 and 116 pixels at 1250 high.
const WEAPON_PITCH: f32 = 0.0228;
/// A place in a row of stars holds a star when its ink covers this share of
/// it. Measured on 2026-10-12: a weapon's star covers 0.28 of its place, an
/// empty place nothing, the tag beside Eimi's stars nothing of theirs.
const STAR_SHARE: f32 = 0.12;
/// "MAX" in gold covers at least this share of a skill label. Measured on
/// 2026-10-12: about 0.010 for MAX, 0 for every "Lv.N".
const GOLD_SHARE: f32 = 0.004;

/// Reads the training of the student on screen.
pub fn read(image: &RgbImage, fonts: &Fonts) -> Training {
    let skill = |i: usize| -> Option<u8> {
        let area = SKILLS[i];
        let (x0, y0, x1, y1) = rect(image, area)?;
        let gold = count(image, area, GOLD)? as f32 / ((x1 - x0) * (y1 - y0)) as f32;
        if gold >= GOLD_SHARE {
            return Some(SKILL_MAX[i]);
        }
        number(image, fonts, "skill", "Lv", area).filter(|&n| (1..SKILL_MAX[i]).contains(&n))
    };
    let weapon_level = number(image, fonts, "weapon", "Lv", WEAPON_LEVEL);
    let weapon_stars = stars(image, WEAPON_STARS, WEAPON_STAR, WEAPON_PITCH);
    let weapon = match (weapon_level, weapon_stars) {
        (None, Some(0)) => Some(None),
        // Four stars so far; five is said to be the most there will be.
        (Some(level), Some(stars @ 1..=5)) => Some(Some(Weapon { level, stars })),
        _ => None,
    };
    let equipment = |i: usize| -> Option<Gear> {
        let tier = number(image, fonts, "tier", "T", EQUIPMENT_TIER[i])?;
        let level = number(image, fonts, "equipment", "Lv", EQUIPMENT_LEVEL[i])?;
        Some(Gear { tier, level })
    };
    // Not released where the badge's slate is missing; a badge whose text
    // does not read is unknown.
    let ability = |i: usize| -> Option<u8> {
        let (x0, y0, x1, y1) = rect(image, ABILITY[i])?;
        let slate = count(image, ABILITY[i], SLATE)?;
        if (slate as f32) < ABILITY_BADGE * ((x1 - x0) * (y1 - y0)) as f32 {
            return Some(0);
        }
        text(image, fonts, "ability", ABILITY[i])?
            .strip_prefix("Lv")?
            .parse()
            .ok()
    };
    // No favorite item where its badge's blue is missing; a badge whose tier
    // does not read (a video's blur) is unknown, not missing.
    let badge = rect(image, GEAR_TIER).zip(count(image, GEAR_TIER, BLUE));
    let gear = match badge {
        Some(((x0, y0, x1, y1), blue))
            if (blue as f32) < GEAR_BADGE * ((x1 - x0) * (y1 - y0)) as f32 =>
        {
            Some(0)
        }
        Some(_) => {
            text(image, fonts, "tier", GEAR_TIER).and_then(|t| t.strip_prefix('T')?.parse().ok())
        }
        None => None,
    };
    Training {
        skills: [skill(0), skill(1), skill(2), skill(3)],
        weapon,
        equipment: [equipment(0), equipment(1), equipment(2)],
        gear,
        ability: [ability(0), ability(1), ability(2)],
        ..plate(image, fonts)
    }
}

/// What the name plate shows, which every tab of the student screen has: the
/// level, the rarity and the bond, the rest unknown.
pub fn plate(image: &RgbImage, fonts: &Fonts) -> Training {
    Training {
        level: level(image, fonts),
        rarity: stars(image, RARITY, STAR, RARITY_PITCH).filter(|n| (1..=5).contains(n)),
        bond: number(image, fonts, "bond", "", BOND),
        skills: [None; 4],
        weapon: None,
        equipment: [None; 3],
        gear: None,
        ability: [None; 3],
    }
}

/// The level on the name plate.
pub fn level(image: &RgbImage, fonts: &Fonts) -> Option<u8> {
    number(image, fonts, "level", "Lv", LEVEL)
}

/// A screen of the student screen, on any tab, whose level is labeled by eye
/// (`tests/fixtures/levels.json`): samples of the level's digits.
#[derive(Debug, Deserialize)]
pub struct LevelLabel {
    pub image: String,
    pub role: String,
    pub level: u8,
}

/// A student screen labeled by eye (`tests/fixtures/students.json`).
#[derive(Debug, Deserialize)]
pub struct Label {
    pub image: String,
    pub role: String,
    pub id: i64,
    pub level: u8,
    pub rarity: u8,
    pub bond: u8,
    pub skills: [u8; 4],
    /// `None` while the unique weapon is locked.
    pub weapon: Option<Weapon>,
    /// Tier and level of each equipment slot; `None` where the slot holds
    /// nothing (nothing equipped, or locked below the level that opens it),
    /// which the screen is not read for yet.
    pub equipment: [Option<[u8; 2]>; 3],
    pub gear: u8,
    pub ability: [u8; 3],
}

impl Label {
    pub fn training(&self) -> Training {
        Training {
            level: Some(self.level),
            rarity: Some(self.rarity),
            bond: Some(self.bond),
            skills: self.skills.map(Some),
            weapon: Some(self.weapon),
            equipment: self
                .equipment
                .map(|slot| slot.map(|[tier, level]| Gear { tier, level })),
            gear: Some(self.gear),
            ability: self.ability.map(Some),
        }
    }

    /// The texts the screen shows in each style, where, for building references:
    /// the tall characters only (the "v" and "." of "Lv." are left out).
    pub fn texts(&self) -> Vec<(&'static str, Area, String)> {
        let mut out = vec![
            ("level", LEVEL, format!("Lv{}", self.level)),
            ("bond", BOND, self.bond.to_string()),
        ];
        for (i, &level) in self.skills.iter().enumerate() {
            if level < SKILL_MAX[i] {
                out.push(("skill", SKILLS[i], format!("Lv{level}")));
            }
        }
        if let Some(weapon) = self.weapon {
            out.push(("weapon", WEAPON_LEVEL, format!("Lv{}", weapon.level)));
        }
        for (i, [tier, level]) in self
            .equipment
            .iter()
            .enumerate()
            .filter_map(|(i, s)| Some((i, (*s)?)))
        {
            out.push(("tier", EQUIPMENT_TIER[i], format!("T{tier}")));
            out.push(("equipment", EQUIPMENT_LEVEL[i], format!("Lv{level}")));
        }
        if self.gear > 0 {
            out.push(("tier", GEAR_TIER, format!("T{}", self.gear)));
        }
        for (i, &level) in self.ability.iter().enumerate() {
            if level > 0 {
                out.push(("ability", ABILITY[i], format!("Lv{level}")));
            }
        }
        out
    }
}

/// The characters of `style` in `area`, normalized, for building references.
pub fn shapes(image: &RgbImage, name: &str, area: Area) -> Option<Vec<Vec<f32>>> {
    Some(glyph::shapes(image, rect(image, area)?, style_of(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shape inked in rows `from..to` of the glyph, at `ink`.
    fn rows(from: usize, to: usize, ink: f32) -> Vec<f32> {
        let (w, h) = glyph::GLYPH;
        (0..w * h)
            .map(|i| {
                if (from * w..to * w).contains(&i) {
                    ink
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// A number's prefix needs only its last letter nearest to what it is,
    /// the letters before it there at all: a stream blurs the "L" of "Lv70"
    /// past what a digit may lie from its references, or breaks its foot off,
    /// while the digits keep to theirs. A prefix letter lost, something else
    /// where its last one stands (a speck, a piece of a digit), or a digit
    /// blurred leaves the number unread.
    #[test]
    fn a_prefix_is_told_by_its_nearest_letters_and_the_digits_strictly() {
        let refs = [('L', 0), ('v', 4), ('7', 8), ('0', 12)]
            .map(|(c, from)| (c, vec![rows(from, from + 4, 1.0)]));
        let fonts = Fonts([("equipment".to_owned(), refs.into_iter().collect())].into());
        let [l, v, seven, zero] = [0, 4, 8, 12].map(|from| rows(from, from + 4, 1.0));
        let blurred = |from| rows(from, from + 4, 0.4);
        let read =
            |shapes: &[Vec<f32>]| after_prefix(&fonts, "equipment", shapes, "Lv", f32::INFINITY);
        let lv70 = [blurred(0), v.clone(), seven.clone(), zero.clone()];
        assert_eq!(fonts.read("equipment", &lv70), None);
        assert_eq!(read(&lv70), Some("70".to_owned()));
        let footless = [seven.clone(), v.clone(), seven.clone(), zero.clone()];
        assert_eq!(read(&footless), Some("70".to_owned()));
        let speckled = [
            l.clone(),
            blurred(12),
            v.clone(),
            seven.clone(),
            zero.clone(),
        ];
        assert_eq!(read(&speckled), None);
        assert_eq!(
            read(&[l.clone(), seven.clone(), v.clone(), zero.clone()]),
            None
        );
        assert_eq!(read(&[v.clone(), seven.clone(), zero.clone()]), None);
        assert_eq!(read(&[l, v, blurred(8), zero]), None);
    }
}
