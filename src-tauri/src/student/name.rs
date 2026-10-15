//! Tells which student a screen shows from the name on its plate: the name's
//! white letters, right of the bond heart and short of the rarity stars, told
//! among the students' names (see `glyph::word`).

use image::RgbImage;

use crate::{
    glyph::{
        bank::{Bank, Word},
        word::{self, Ink},
    },
    screen,
    student::{NAME, rect},
};

/// The name's columns on the plate, in units (`screen::unit`) from the left: past the
/// bond heart (it ends at 0.09) and short of the stars (from 0.37), measured on
/// 2026-10-14 over the student fixtures at 2222x1250. A long name is drawn
/// smaller to fit.
const COLUMNS: (f32, f32) = (0.10, 0.365);
/// Parts of the letters smaller than this share of the unit squared are specks
/// (6 pixels at 1250 rows; a voiced mark's dots are 8 or more).
const SPECK: f32 = 6.0 / (1250.0 * 1250.0);
/// The farthest a letter of the plate may lie from the name's own letter.
/// Measured on 2026-10-14 over the student fixtures at their own size (2222
/// wide, where 24 was taken): a right name's farthest letter lies
/// within 22.6 (an evaluation plate), and with each student left out in turn,
/// the nearest other name has a letter 26.1 or farther ("ハルカ" for "ハルナ").
/// Measured again on 2026-10-15 over them scaled down: from 1280 to 1920 wide
/// another name comes as near as 22.9 (for "ツムギ" at 1600), while some right
/// ones lie up to 29. A name not told is safer than one told wrong.
const WORST: f32 = 22.0;

/// The white letters of the name.
fn white(c: [u8; 3]) -> bool {
    c.iter().all(|&v| v > 200)
}

/// The name's letters on the plate of `image` (none where the window is too
/// narrow for it).
pub fn plate(image: &RgbImage) -> Option<Ink> {
    let unit = screen::unit(image.width(), image.height());
    let (_, top, _, bottom) = rect(image, NAME)?;
    let (left, right) = ((COLUMNS.0 * unit) as u32, (COLUMNS.1 * unit) as u32);
    if right > image.width() {
        return None;
    }
    let least = (SPECK * unit * unit).round().max(1.0) as usize;
    Some(Ink::parts(image, (left, top, right, bottom), white, least))
}

/// The student among `names` whose name the plate of `image` shows. A name
/// SchaleDB lists under two ids (one a form, as "ホシノ（臨戦）") is one word,
/// under the lower id.
pub fn read<'a>(
    image: &RgbImage,
    bank: &Bank,
    names: impl IntoIterator<Item = &'a Word>,
) -> Option<i64> {
    let ink = plate(image)?;
    word::nearest(&ink, names, &bank.tiles, &bank.index.letters, WORST).map(|w| w.id)
}
