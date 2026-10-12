//! Tells which SchaleDB icon a card shows.
//!
//! The game draws the whole SchaleDB icon canvas upright, scaled to
//! `ICON_HEIGHT_RATIO` of the card height and centred on the card. Cards are
//! resampled to a fixed size, so templates are prepared once for any window
//! size. A cheap comparison at low resolution shortlists icons, then the
//! shortlist is compared at full template size with a small position search.

use image::{RgbImage, RgbaImage, imageops::FilterType};
use serde::Deserialize;

use crate::grid::Card;

/// Icon canvas height relative to the card height.
const ICON_HEIGHT_RATIO: f32 = 0.933;
/// SchaleDB icon canvas size.
const CANVAS: (u32, u32) = (146, 116);
const FINE_HEIGHT: u32 = 58;
const COARSE_HEIGHT: u32 = 15;
/// Position search in fine pixels around the expected placement.
const SEARCH: u32 = 2;
const SHORTLIST: usize = 12;
/// Template pixels below this alpha are background and ignored.
const OPAQUE: u8 = 230;
/// The part of the icon canvas that other drawings cover and the comparison
/// leaves out: from `left` to the right edge and from `top` to the bottom
/// (fractions of the canvas width and height).
#[derive(Debug, Clone, Copy)]
pub struct Mask {
    pub left: f32,
    pub top: f32,
}
/// How rare an item is (SchaleDB's `rarity`), which the game shows in the
/// colour of the card behind its icon: grey, blue, gold, purple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Rarity {
    N,
    R,
    SR,
    SSR,
}

/// The chromaticity (each channel's share of the brightness) of the card
/// behind the icon for each rarity: the means over the fixtures on 2026-10-13
/// (110 N, 25 R, 28 SR and 38 SSR cards, each within 0.03 of its own).
const BACKGROUNDS: [(Rarity, Rgb); 4] = [
    (Rarity::N, [0.321, 0.335, 0.344]),
    (Rarity::R, [0.300, 0.334, 0.366]),
    (Rarity::SR, [0.375, 0.345, 0.280]),
    (Rarity::SSR, [0.327, 0.300, 0.373]),
];

/// A background names a rarity when it is at most this share of the distance
/// to the next nearest.
const BACKGROUND_LEAD: f32 = 0.6;

impl Rarity {
    /// The rarity whose card colour `chroma` clearly is.
    pub fn of_background(chroma: Rgb) -> Option<Self> {
        let far = |c: Rgb| (0..3).map(|i| (c[i] - chroma[i]).abs()).sum::<f32>();
        let mut by: Vec<(f32, Self)> = BACKGROUNDS.iter().map(|&(r, c)| (far(c), r)).collect();
        by.sort_by(|a, b| a.0.total_cmp(&b.0));
        (by[0].0 <= BACKGROUND_LEAD * by[1].0).then_some(by[0].1)
    }
}

/// A match is accepted when it is this close and leads the runner-up by `CLOSE.1`,
const CLOSE: (f32, f32) = (0.93, 0.05);
/// or, down to `CLEAR.0`, by a lead that grows on the line from `CLOSE` to
/// `CLEAR` (0.3 at 0.8). Look-alikes (tiers of one material, students' elephs
/// in the same frame) score up to about 0.9, while faint icons such as a
/// written page may score only 0.83 and a titanium spring 0.898 (lead 0.2).
const CLEAR: (f32, f32) = (0.8, 0.3);
/// Icons that differ only in a detail (a gift box with a "?" or a tick tag, the
/// universal blueprints of two categories) tie overall (lead under `CLOSE.1`);
/// then the winner needs at least `DETAIL.0` overall and, on the pixels where
/// the two icons differ, at least `DETAIL.1` (a gloves universal blueprint
/// measures 0.898) and a lead of `DETAIL.2` over the runner-up.
const DETAIL: (f32, f32, f32) = (0.9, 0.88, 0.3);
/// Most difference in average tint (`Template::tint`) for a match. Matches measure
/// up to about 0.03; a green ticket against the blue one 0.05 and more.
const MAX_TINT: f32 = 0.04;
/// Most difference in average brightness (`Template::level`) for a candidate.
/// Matches measure up to 0.036 (180 matches of 0.85 or more, 2026-10-12); a
/// chrome spring against the titanium one, which NCC and tint cannot tell
/// apart, 0.10 and more.
const MAX_LEVEL: f32 = 0.07;
/// Two icons differ at a pixel when their channels differ by this much in sum.
const DIFFERENT: f32 = 90.0;

pub type Rgb = [f32; 3];

/// A candidate icon as compared: its index, its score, and the sample and the
/// position it scored at.
type Ranked<'a> = (usize, f32, &'a RgbImage, (u32, u32));

/// How far from the icon's drawing a pixel of the card behind it is taken, in
/// template pixels (the drawing's edge blurs, and the match may be a pixel off).
const BACKGROUND_ROOM: i64 = 2;

struct Template {
    width: u32,
    /// Colour of each pixel taking part in the comparison, row by row.
    grid: Vec<Option<Rgb>>,
    /// Where the card shows behind the icon, `BACKGROUND_ROOM` from the drawing.
    clear: Vec<bool>,
}

impl Template {
    fn new(icon: &RgbaImage, height: u32, mask: Mask) -> Self {
        let width = scaled_width(height);
        let small = image::imageops::resize(icon, width, height, FilterType::Triangle);
        let covered = |x: u32, y: u32| {
            x as f32 >= mask.left * width as f32 && y as f32 >= mask.top * height as f32
        };
        let grid = small
            .enumerate_pixels()
            .map(|(x, y, p)| {
                (p.0[3] >= OPAQUE && !covered(x, y))
                    .then(|| [f32::from(p.0[0]), f32::from(p.0[1]), f32::from(p.0[2])])
            })
            .collect();
        let bare = |x: i64, y: i64| {
            let inside = (0..i64::from(width)).contains(&x) && (0..i64::from(height)).contains(&y);
            inside && {
                let (x, y) = (x as u32, y as u32);
                small.get_pixel(x, y).0[3] == 0 && !covered(x, y)
            }
        };
        let room = -BACKGROUND_ROOM..=BACKGROUND_ROOM;
        let clear = small
            .enumerate_pixels()
            .map(|(x, y, _)| {
                let (x, y) = (i64::from(x), i64::from(y));
                room.clone()
                    .all(|dy| room.clone().all(|dx| bare(x + dx, y + dy)))
            })
            .collect();
        Self { width, grid, clear }
    }

    /// The chromaticity of the card behind the icon placed at `(ox, oy)`, from
    /// template row `from` down; none where too little of it shows.
    fn background(&self, image: &RgbImage, ox: u32, oy: u32, from: u32) -> Option<Rgb> {
        let (mut sum, mut n) = ([0.0f32; 3], 0u32);
        for (i, _) in self
            .clear
            .iter()
            .enumerate()
            .skip((from * self.width) as usize)
            .filter(|(_, c)| **c)
        {
            let (x, y) = (i as u32 % self.width, i as u32 / self.width);
            let px = image.get_pixel(ox + x, oy + y).0;
            (0..3).for_each(|c| sum[c] += f32::from(px[c]));
            n += 1;
        }
        let total: f32 = sum.iter().sum();
        (n >= 20 && total > 0.0).then(|| sum.map(|c| c / total))
    }

    /// The pixels taking part, from row `from` down (rows above it are hidden).
    fn pixels(&self, from: u32) -> impl Iterator<Item = (u32, u32, Rgb)> + '_ {
        (0u32..)
            .zip(&self.grid)
            .skip((from * self.width) as usize)
            .filter_map(|(i, p)| p.map(|rgb| (i % self.width, i / self.width, rgb)))
    }

    fn at(&self, x: u32, y: u32) -> Option<Rgb> {
        self.grid[(y * self.width + x) as usize]
    }

    /// How far the average hue of `image` under the icon is from the icon's own:
    /// the summed difference of their chromaticities (each channel's share of the
    /// brightness), 0 for the same tint.
    fn tint(&self, image: &RgbImage, ox: u32, oy: u32, from: u32) -> f32 {
        let (mut t, mut p) = ([0.0f32; 3], [0.0f32; 3]);
        for (x, y, rgb) in self.pixels(from) {
            let px = image.get_pixel(ox + x, oy + y).0;
            for c in 0..3 {
                t[c] += rgb[c];
                p[c] += f32::from(px[c]);
            }
        }
        let share = |v: [f32; 3]| {
            let sum = v.iter().sum::<f32>().max(1.0);
            v.map(|c| c / sum)
        };
        let (t, p) = (share(t), share(p));
        (0..3).map(|c| (t[c] - p[c]).abs()).sum()
    }

    /// How far the average brightness of `image` under the icon is from the
    /// icon's own, as a share of full brightness.
    fn level(&self, image: &RgbImage, ox: u32, oy: u32, from: u32) -> f32 {
        let (mut t, mut p, mut n) = (0.0f32, 0.0f32, 0.0f32);
        for (x, y, rgb) in self.pixels(from) {
            let px = image.get_pixel(ox + x, oy + y).0;
            t += rgb.iter().sum::<f32>();
            p += px.iter().map(|&v| f32::from(v)).sum::<f32>();
            n += 3.0;
        }
        ((t - p) / n.max(1.0) / 255.0).abs()
    }

    /// Similarity with `image` placed at `(ox, oy)`, from template row `from` down.
    fn score(&self, image: &RgbImage, ox: u32, oy: u32, from: u32) -> f32 {
        ncc(self
            .pixels(from)
            .map(|(x, y, t)| (t, image.get_pixel(ox + x, oy + y).0)))
    }
}

/// Normalized cross-correlation of paired template and image colours.
fn ncc(pairs: impl Iterator<Item = (Rgb, [u8; 3])>) -> f32 {
    let (mut n, mut st, mut si, mut stt, mut sii, mut sti) = (0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (t, p) in pairs {
        for c in 0..3 {
            let (a, b) = (t[c], f32::from(p[c]));
            n += 1.0;
            st += a;
            si += b;
            stt += a * a;
            sii += b * b;
            sti += a * b;
        }
    }
    let var_t = stt - st * st / n;
    let var_i = sii - si * si / n;
    if n < 30.0 || var_t <= 0.0 || var_i <= 0.0 {
        return -1.0;
    }
    (sti - st * si / n) / (var_t * var_i).sqrt()
}

/// The lead a match of `score` needs over the runner-up: `CLOSE.1` from
/// `CLOSE.0` up, growing on a line to `CLEAR.1` at `CLEAR.0`.
fn needed_lead(score: f32) -> f32 {
    let t = ((CLOSE.0 - score) / (CLOSE.0 - CLEAR.0)).clamp(0.0, 1.0);
    CLOSE.1 + t * (CLEAR.1 - CLOSE.1)
}

fn scaled_width(height: u32) -> u32 {
    (height * CANVAS.0 + CANVAS.1 / 2) / CANVAS.1
}

pub struct IconMatcher {
    coarse: Vec<Template>,
    fine: Vec<Template>,
    /// The rarity of each icon's items, where they agree.
    rarity: Vec<Option<Rarity>>,
}

impl IconMatcher {
    /// Templates of `icons` and their `rarity`, leaving out the canvas part
    /// under `mask`.
    pub fn new(icons: &[RgbaImage], rarity: Vec<Option<Rarity>>, mask: Mask) -> Self {
        Self {
            rarity,
            coarse: icons
                .iter()
                .map(|i| Template::new(i, COARSE_HEIGHT, mask))
                .collect(),
            fine: icons
                .iter()
                .map(|i| Template::new(i, FINE_HEIGHT, mask))
                .collect(),
        }
    }

    /// The card's icon area at SchaleDB's canvas size, for making the icon of an
    /// item SchaleDB lacks.
    pub fn canvas(image: &RgbImage, card: &Card) -> RgbImage {
        Self::sample(image, card, CANVAS.1, 0)
    }

    /// The card area where the icon sits, resampled so the icon is `height` tall,
    /// with `margin` extra pixels on every side for the position search.
    fn sample(image: &RgbImage, card: &Card, height: u32, margin: u32) -> RgbImage {
        let width = scaled_width(height);
        let scale = card.height as f32 * ICON_HEIGHT_RATIO / height as f32;
        let (out_w, out_h) = (width + 2 * margin, height + 2 * margin);
        let x0 = card.center_x() - out_w as f32 * scale / 2.0;
        let y0 = card.center_y() - out_h as f32 * scale / 2.0;
        let (cx, cy) = (x0.round().max(0.0) as u32, y0.round().max(0.0) as u32);
        let crop_w = ((out_w as f32 * scale).round() as u32).min(image.width() - cx);
        let crop_h = ((out_h as f32 * scale).round() as u32).min(image.height() - cy);
        let crop = image::imageops::crop_imm(image, cx, cy, crop_w, crop_h).to_image();
        image::imageops::resize(&crop, out_w, out_h, FilterType::Triangle)
    }

    /// The first template row (of a template `height` tall) below image row
    /// `hidden`, plus `margin` rows for the position search.
    fn first_visible_row(card: &Card, hidden: Option<u32>, height: u32, margin: u32) -> u32 {
        let Some(hidden) = hidden else { return 0 };
        let canvas = card.height as f32 * ICON_HEIGHT_RATIO;
        let canvas_top = card.center_y() - canvas / 2.0;
        let rows = ((hidden as f32 - canvas_top) / canvas * height as f32)
            .ceil()
            .max(0.0);
        (rows as u32 + margin).min(height)
    }

    /// The icon among `candidates` that the card clearly shows, or else the icons it
    /// could be (none when nothing is close). Image rows above `hidden` (a badge
    /// over the icon) are left out.
    pub fn identify(
        &self,
        image: &RgbImage,
        card: &Card,
        hidden: Option<u32>,
        candidates: &[usize],
    ) -> Result<usize, Vec<usize>> {
        let coarse_from = Self::first_visible_row(card, hidden, COARSE_HEIGHT, 1);
        let from = Self::first_visible_row(card, hidden, FINE_HEIGHT, SEARCH);
        let coarse = Self::sample(image, card, COARSE_HEIGHT, 0);
        let shortlist: Vec<(usize, f32)> = candidates
            .iter()
            .map(|&i| (i, self.coarse[i].score(&coarse, 0, 0, coarse_from)))
            .collect();
        // Thin icons (a barrel, a firing pin) leave too few pixels to score at low
        // resolution once the count is masked; they always go on to the fine
        // comparison.
        let (unscored, mut shortlist): (Vec<_>, Vec<_>) =
            shortlist.into_iter().partition(|s| s.1 <= -1.0);
        shortlist.sort_by(|a, b| b.1.total_cmp(&a.1));
        shortlist.truncate(SHORTLIST);
        shortlist.extend(unscored);

        // Fine detail (a page of writing) suffers from a pixel's error in the measured
        // card height, so the card is also sampled a pixel smaller and larger.
        let samples: Vec<RgbImage> = [-1i32, 0, 1]
            .iter()
            .map(|&d| {
                let card = Card {
                    height: card.height.saturating_add_signed(d),
                    ..*card
                };
                Self::sample(image, &card, FINE_HEIGHT, SEARCH)
            })
            .collect();
        let mut ranked: Vec<Ranked> = shortlist
            .into_iter()
            .map(|(i, _)| {
                let t = &self.fine[i];
                debug_assert_eq!(t.width + 2 * SEARCH, samples[1].width());
                // The whole position search at the measured size, then the other sizes
                // only around the best position.
                let mut best = (f32::MIN, &samples[1], (0, 0));
                for oy in 0..=2 * SEARCH {
                    for ox in 0..=2 * SEARCH {
                        let score = t.score(&samples[1], ox, oy, from);
                        if score > best.0 {
                            best = (score, &samples[1], (ox, oy));
                        }
                    }
                }
                let (bx, by) = best.2;
                for fine in [&samples[0], &samples[2]] {
                    for oy in by.saturating_sub(1)..=(by + 1).min(2 * SEARCH) {
                        for ox in bx.saturating_sub(1)..=(bx + 1).min(2 * SEARCH) {
                            let score = t.score(fine, ox, oy, from);
                            if score > best.0 {
                                best = (score, fine, (ox, oy));
                            }
                        }
                    }
                }
                (i, best.0, best.1, best.2)
            })
            .collect();
        // Icons as bright as the card's (NCC and tint ignore brightness).
        ranked
            .retain(|&(i, _, fine, (ox, oy))| self.fine[i].level(fine, ox, oy, from) <= MAX_LEVEL);
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));

        let Some(&(best, score, fine, (ox, oy))) = ranked.first() else {
            return Err(Vec::new());
        };
        // A recoloured twin (a green ticket for a blue one) matches in shape only.
        if self.fine[best].tint(fine, ox, oy, from) > MAX_TINT {
            return Err(Vec::new());
        }
        // The icons the card could be when no single one stands out.
        let tied = || -> Vec<usize> {
            ranked
                .iter()
                .take_while(|r| r.1 >= CLEAR.0 && score - r.1 < CLOSE.1)
                .map(|r| r.0)
                .collect()
        };
        let Some(&(runner_up, next, _, _)) = ranked.get(1) else {
            return if score >= CLEAR.0 {
                Ok(best)
            } else {
                Err(Vec::new())
            };
        };
        let lead = score - next;
        if score >= CLEAR.0 && lead >= needed_lead(score) {
            return Ok(best);
        }
        if score < DETAIL.0 || lead >= CLOSE.1 {
            return Err(tied());
        }
        if let Some(found) = self.by_background(&ranked, from) {
            return Ok(found);
        }
        // Compare both icons where they differ, at the winner's position.
        let (a, b) = (&self.fine[best], &self.fine[runner_up]);
        let differing: Vec<(Rgb, Rgb, [u8; 3])> = a
            .pixels(from)
            .filter_map(|(x, y, ta)| {
                let tb = b.at(x, y)?;
                let diff: f32 = (0..3).map(|c| (ta[c] - tb[c]).abs()).sum();
                (diff >= DIFFERENT).then(|| (ta, tb, fine.get_pixel(ox + x, oy + y).0))
            })
            .collect();
        let score_a = ncc(differing.iter().map(|&(ta, _, p)| (ta, p)));
        let score_b = ncc(differing.iter().map(|&(_, tb, p)| (tb, p)));
        if score_a >= DETAIL.1 && score_a - score_b >= DETAIL.2 {
            Ok(best)
        } else {
            Err(tied())
        }
    }

    /// Twins drawn alike for two rarities (the grey and the blue grade of a
    /// school's tech note share its logo) are told by the card behind them,
    /// which has the colour of its rarity; then the drawing tells the kind
    /// among the icons of that rarity (`ranked` best first).
    fn by_background(&self, ranked: &[Ranked], from: u32) -> Option<usize> {
        let (&(best, _, fine, (ox, oy)), &(runner_up, ..)) = (ranked.first()?, ranked.get(1)?);
        if self.rarity[best] == self.rarity[runner_up] {
            return None;
        }
        let rarity = Rarity::of_background(self.fine[best].background(fine, ox, oy, from)?)?;
        let mut alike = ranked.iter().filter(|r| self.rarity[r.0] == Some(rarity));
        let &(first, score, ..) = alike.next()?;
        let lead = alike.next().map_or(f32::INFINITY, |r| score - r.1);
        (score >= CLEAR.0 && lead >= needed_lead(score)).then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_names_the_rarity_of_its_colour() {
        // Measured on the fixtures (2026-10-13), each rarity's extremes.
        let cases = [
            ([0.313, 0.334, 0.350], Some(Rarity::N)),
            ([0.340, 0.338, 0.322], Some(Rarity::N)),
            ([0.304, 0.334, 0.362], Some(Rarity::R)),
            ([0.297, 0.333, 0.369], Some(Rarity::R)),
            ([0.369, 0.343, 0.288], Some(Rarity::SR)),
            ([0.331, 0.292, 0.377], Some(Rarity::SSR)),
            // Halfway between grey and blue, a background names neither.
            ([0.3105, 0.3345, 0.355], None),
        ];
        for (chroma, rarity) in cases {
            assert_eq!(Rarity::of_background(chroma), rarity, "{chroma:?}");
        }
    }
}
