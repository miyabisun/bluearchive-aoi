//! Tells which SchaleDB icon a card shows.
//!
//! The game draws the whole SchaleDB icon canvas upright, scaled to
//! `ICON_HEIGHT_RATIO` of the card height and centred on the card. Cards are
//! resampled to a fixed size, so templates are prepared once for any window
//! size. A cheap comparison at low resolution shortlists icons, then the
//! shortlist is compared at full template size with a small position search.

use image::{RgbImage, RgbaImage, imageops::FilterType};

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
/// The "xN" count covers the bottom right of the icon canvas; that part is
/// left out of the comparison (fractions of the canvas width and height).
const COUNT_AREA: (f32, f32) = (0.25, 0.8);
/// A match is accepted when it is this close and leads the runner-up by `CLOSE.1`,
const CLOSE: (f32, f32) = (0.93, 0.05);
/// or when it is this close and leads the runner-up by `CLEAR.1`. Look-alikes
/// (tiers of one material, students' elephs in the same frame) score up to
/// about 0.9, while faint icons such as a written page may score only 0.83.
const CLEAR: (f32, f32) = (0.8, 0.3);
/// Icons that differ only in a detail (a gift box with a "?" or a tick tag)
/// tie overall (lead under `CLOSE.1`); then the winner needs at least `DETAIL.0` overall and,
/// on the pixels where the two icons differ, at least `DETAIL.1` and a lead of
/// `DETAIL.2` over the runner-up.
const DETAIL: (f32, f32, f32) = (0.9, 0.9, 0.3);
/// Most difference in average tint (`Template::tint`) for a match. Matches measure
/// up to about 0.03; a green ticket against the blue one 0.05 and more.
const MAX_TINT: f32 = 0.04;
/// Two icons differ at a pixel when their channels differ by this much in sum.
const DIFFERENT: f32 = 90.0;

type Rgb = [f32; 3];

struct Template {
    width: u32,
    /// Colour of each pixel taking part in the comparison, row by row.
    grid: Vec<Option<Rgb>>,
}

impl Template {
    fn new(icon: &RgbaImage, height: u32) -> Self {
        let width = scaled_width(height);
        let small = image::imageops::resize(icon, width, height, FilterType::Triangle);
        let grid = small
            .enumerate_pixels()
            .map(|(x, y, p)| {
                let in_count = x as f32 >= COUNT_AREA.0 * width as f32
                    && y as f32 >= COUNT_AREA.1 * height as f32;
                (p.0[3] >= OPAQUE && !in_count)
                    .then(|| [f32::from(p.0[0]), f32::from(p.0[1]), f32::from(p.0[2])])
            })
            .collect();
        Self { width, grid }
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

fn scaled_width(height: u32) -> u32 {
    (height * CANVAS.0 + CANVAS.1 / 2) / CANVAS.1
}

pub struct IconMatcher {
    coarse: Vec<Template>,
    fine: Vec<Template>,
}

impl IconMatcher {
    pub fn new(icons: &[RgbaImage]) -> Self {
        Self {
            coarse: icons
                .iter()
                .map(|i| Template::new(i, COARSE_HEIGHT))
                .collect(),
            fine: icons
                .iter()
                .map(|i| Template::new(i, FINE_HEIGHT))
                .collect(),
        }
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

    /// The icon among `candidates` that the card clearly shows, if any. Image rows
    /// above `hidden` (a badge over the icon) are left out.
    pub fn identify(
        &self,
        image: &RgbImage,
        card: &Card,
        hidden: Option<u32>,
        candidates: &[usize],
    ) -> Option<usize> {
        let coarse_from = Self::first_visible_row(card, hidden, COARSE_HEIGHT, 1);
        let from = Self::first_visible_row(card, hidden, FINE_HEIGHT, SEARCH);
        let coarse = Self::sample(image, card, COARSE_HEIGHT, 0);
        let mut shortlist: Vec<(usize, f32)> = candidates
            .iter()
            .map(|&i| (i, self.coarse[i].score(&coarse, 0, 0, coarse_from)))
            .collect();
        shortlist.sort_by(|a, b| b.1.total_cmp(&a.1));
        shortlist.truncate(SHORTLIST);

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
        let mut ranked: Vec<(usize, f32, &RgbImage, (u32, u32))> = shortlist
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
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));

        let &(best, score, fine, (ox, oy)) = ranked.first()?;
        // A recoloured twin (a green ticket for a blue one) matches in shape only.
        if self.fine[best].tint(fine, ox, oy, from) > MAX_TINT {
            return None;
        }
        let Some(&(runner_up, next, _, _)) = ranked.get(1) else {
            return (score >= CLEAR.0).then_some(best);
        };
        let lead = score - next;
        if (score >= CLOSE.0 && lead >= CLOSE.1) || (score >= CLEAR.0 && lead >= CLEAR.1) {
            return Some(best);
        }
        if score < DETAIL.0 || lead >= CLOSE.1 {
            return None;
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
        (score_a >= DETAIL.1 && score_a - score_b >= DETAIL.2).then_some(best)
    }
}
