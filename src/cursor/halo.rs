//! The gear beside the arrow while a program starts: the halo of Anna, a student of the
//! game, drawn here from its shape as measured off a picture of it (350 pixels square),
//! not copied from the picture. Ported from windows-link's `cursor/halo.rs`, Anna's alone
//! (the user's of 2026-11-13: 「マウスカーソルの読み込み中の矢印+グルグルをwindows-linkから移植」).
//!
//! A ring with fourteen teeth, and in it a small ring with six teeth round a six-sided
//! one. Purple at the top turns light blue at the bottom, and the colours go round it,
//! the gear still (the user's of 2026-11-10, of windows-link's: the picture is not
//! turned, the colour in it flows).

/// How far the gear reaches from its middle and how wide its thinnest line is, in its
/// picture's pixels.
const REACH: f32 = 141.25;
const THIN: f32 = 6.6;
/// Samples a dot is drawn from, across and down.
const SAMPLES: u32 = 8;
/// The gear's lines are kept at least this many of the cursor's dots wide.
const THINNEST: f32 = 1.5;

/// The gear on a clear ground `side` pixels square, `across` pixels wide with its middle
/// at `centre`, once for each of `turns`: how many degrees clockwise its colours have
/// gone round (its shape stays). A navy edge is drawn round it, as round the arrow, and
/// its thinnest lines are kept wide enough to show.
pub fn gear(side: u32, centre: (f32, f32), across: f32, turns: &[f32]) -> Vec<Vec<[u8; 4]>> {
    // The edge lies outside the gear's lines, so the lines stop short of `across`.
    let radius = across / 2.0 - super::EDGE_PIXELS - 0.5;
    // The picture's pixels to one of the cursor's dots.
    let scale = REACH / radius;
    let edge = super::EDGE_PIXELS * scale;
    let grow = ((THINNEST - THIN / scale) / 2.0).max(0.0) * scale;
    let place = |at: u32, part: (f32, f32)| {
        (
            (at % side) as f32 + part.0 - centre.0,
            (at / side) as f32 + part.1 - centre.1,
        )
    };
    // How much of each dot the gear's lines cover, and how much their edge.
    let cover: Vec<(f32, f32)> = (0..side * side)
        .map(|at| {
            let (mut lines, mut rim) = (0.0f32, 0.0f32);
            for sample in 0..SAMPLES * SAMPLES {
                let part = |step: u32| (step as f32 + 0.5) / SAMPLES as f32;
                let (x, y) = place(at, (part(sample % SAMPLES), part(sample / SAMPLES)));
                let depth = depth(x * scale, y * scale) + grow;
                if depth > 0.0 {
                    lines += 1.0;
                } else if depth > -edge {
                    rim += 1.0;
                }
            }
            let samples = (SAMPLES * SAMPLES) as f32;
            (lines / samples, rim / samples)
        })
        .collect();
    let byte = |value: f32| u8::try_from(super::to_byte_range(value, 255.0)).unwrap_or(u8::MAX);
    turns
        .iter()
        .map(|&turn| {
            cover
                .iter()
                .zip(0u32..)
                .map(|(&(lines, rim), at)| {
                    if lines + rim <= 0.0 {
                        return [0; 4];
                    }
                    let (right, down) = place(at, (0.5, 0.5));
                    let own = colour(right / radius, down / radius, turn);
                    let [r, g, b] = [0, 1, 2]
                        .map(|i| byte(own[i].mul_add(lines, super::NAVY[i] * rim) / (lines + rim)));
                    [r, g, b, byte((lines + rim) * 255.0)]
                })
                .collect()
        })
        .collect()
}

/// How far inside the gear's lines (`x`, `y`) is, in its picture's pixels from its
/// middle; negative outside them.
fn depth(x: f32, y: f32) -> f32 {
    let far = x.hypot(y);
    let ring = |inner: f32, outer: f32| (far - inner).min(outer - far);
    let teeth = bar(spoke(x, y, 14.0), (110.0, 141.25), (10.0, 6.0));
    let small = bar(spoke(x, y, 6.0), (45.0, 57.25), (7.5, 5.5));
    // Six sides, a corner up: how far out the side (`x`, `y`) lies on is.
    let six = x.abs().max(0.5f32.mul_add(x.abs(), 0.866 * y.abs()));
    let nut = (34.0 - six).min(six - 27.4);
    ring(96.25, 115.5)
        .max(teeth)
        .max(ring(39.0, 48.0))
        .max(small)
        .max(nut)
}

/// The gear's colour at (`x`, `y`), its reach being 1 from its middle, its colours gone
/// `turn` degrees round: purple at the top shades to light blue at the bottom.
fn colour(x: f32, y: f32, turn: f32) -> [f32; 3] {
    let (sin, cos) = turn.to_radians().sin_cos();
    let down = y.mul_add(cos, -x * sin);
    let (purple, blue): ([f32; 3], [f32; 3]) = ([156.0, 121.0, 211.0], [121.0, 224.0, 216.0]);
    let part = ((down + 0.8) / 1.6).clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (blue[i] - purple[i]).mul_add(part, purple[i]))
}

/// (`x`, `y`) turned back onto the nearest of `spokes` spokes, the first pointing up:
/// how far along that spoke it is, and how far to its side.
fn spoke(x: f32, y: f32, spokes: f32) -> (f32, f32) {
    let step = std::f32::consts::TAU / spokes;
    let (sin, cos) = ((x.atan2(-y) / step).round() * step).sin_cos();
    (x.mul_add(sin, -y * cos), x.mul_add(cos, y * sin))
}

/// How far inside a bar lying on a spoke the point (`along`, `aside`) of `spoke` is:
/// the bar runs between two distances from the middle and is half as wide as `half`
/// at each end.
fn bar((along, aside): (f32, f32), (from, to): (f32, f32), half: (f32, f32)) -> f32 {
    let wide = (half.1 - half.0).mul_add(((along - from) / (to - from)).clamp(0.0, 1.0), half.0);
    (along - from).min(to - along).min(wide - aside.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDE: u32 = 256;

    /// The gear drawn big, its colours turned by `turn`.
    fn big(turn: f32) -> Vec<[u8; 4]> {
        gear(SIDE, (128.0, 128.0), 256.0, &[turn]).remove(0)
    }

    /// The dot `far` of the gear's reach from its middle, `degrees` clockwise of up.
    fn at(picture: &[[u8; 4]], far: f32, degrees: f32) -> [u8; 4] {
        let (sin, cos) = degrees.to_radians().sin_cos();
        // The gear reaches a little short of the picture's side: its edge is outside it.
        let reach = 128.0 - 1.75;
        let column = super::super::to_byte_range(128.0 + far * reach * sin, 255.0);
        let row = super::super::to_byte_range(128.0 - far * reach * cos, 255.0);
        picture[usize::try_from(row * SIDE + column).unwrap()]
    }

    /// How many separate stretches of the gear's own colour a circle `far` out crosses.
    fn stretches(picture: &[[u8; 4]], far: f32) -> usize {
        let lit = |tenth: u16| {
            let [r, g, b, a] = at(picture, far, f32::from(tenth) / 10.0);
            a == 255 && r.max(g).max(b) > 150
        };
        let starts = (0..3600)
            .filter(|&d| lit(d) && !lit((d + 3599) % 3600))
            .count();
        if starts == 0 && lit(0) { 1 } else { starts }
    }

    /// Anna's halo is a gear: fourteen teeth outside its ring, six on the small ring
    /// inside, and a hole in the middle.
    #[test]
    fn annas_halo_is_a_gear() {
        let gear = big(0.0);
        assert_eq!(stretches(&gear, 0.92), 14);
        assert_eq!(stretches(&gear, 0.75), 1, "the ring is whole");
        assert_eq!(stretches(&gear, 0.375), 6);
        assert_eq!(at(&gear, 0.0, 0.0)[3], 0);
        assert_eq!(at(&gear, 0.55, 40.0)[3], 0, "clear between the rings");
    }

    /// While a program is busy the gear stays as it is and its colours go round it:
    /// purple at the top and light blue at the bottom, the other way half a turn later.
    #[test]
    fn annas_colours_go_round_a_gear_that_stays() {
        let frames = gear(SIDE, (128.0, 128.0), 256.0, &[0.0, 90.0, 180.0]);
        let alpha = |frame: &[[u8; 4]]| frame.iter().map(|dot| dot[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&frames[0]), alpha(&frames[1]));
        assert_eq!(alpha(&frames[0]), alpha(&frames[2]));
        let purple = |[r, g, _, _]: [u8; 4]| r > g + 20;
        let blue = |[r, g, _, _]: [u8; 4]| g > r + 60;
        assert!(purple(at(&frames[0], 0.75, 0.0)) && blue(at(&frames[0], 0.75, 180.0)));
        assert!(purple(at(&frames[1], 0.75, 90.0)) && blue(at(&frames[1], 0.75, 270.0)));
        assert!(purple(at(&frames[2], 0.75, 180.0)) && blue(at(&frames[2], 0.75, 0.0)));
    }

    /// A pale gear shows over a pale page by a navy edge round it, outside its own
    /// lines, as the arrow has: out of the ring comes its colour, then navy, then
    /// nothing.
    #[test]
    fn a_navy_edge_runs_round_the_gear() {
        let gear = big(0.0);
        let mut seen = Vec::new();
        // Outward from the ring between two teeth (the teeth stand every 25.7 degrees).
        for step in 0..=60u8 {
            let far = 0.25f32.mul_add(f32::from(step) / 60.0, 0.75);
            let dot = at(&gear, far, 12.9);
            let kind = if dot[3] == 0 {
                "clear"
            } else if dot[3] > 30 && dot[..3].iter().all(|&c| c < 110) {
                "navy"
            } else if dot[3] == 255 && dot[..3].iter().any(|&c| c > 150) {
                "own"
            } else {
                continue;
            };
            if seen.last() != Some(&kind) {
                seen.push(kind);
            }
        }
        assert_eq!(seen, ["own", "navy", "clear"]);
    }
}
