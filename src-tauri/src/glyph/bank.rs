//! The glyph bank: the reference pictures of the texts Aoi reads, each a grey
//! tile of one picture (`data/glyphs.png`), with what the tiles stand for
//! (`data/glyphs.json`). `examples/build_glyphs.rs` writes both from the
//! tuning fixtures and, for the students' names, from a font. Small grey tiles
//! in a PNG keep the bank small on disk and in memory, and need no decoder
//! besides the one the screenshots use.

use std::collections::BTreeMap;

use image::{
    GrayImage, ImageEncoder, Luma,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use serde::{Deserialize, Serialize};

use super::{Fonts, GLYPH};

/// The files a bank is kept in, the picture and the index, for screens of
/// `width` (see `screen::WIDTHS`).
pub fn files(width: u32) -> (String, String) {
    (
        format!("glyphs-{width}.png"),
        format!("glyphs-{width}.json"),
    )
}

/// A tile's side in pixels.
pub const TILE: usize = 16;
/// Tiles side by side in the picture.
const ROW: usize = 32;

/// A reference picture: how much of each pixel is ink, 0 to 255, row by row.
pub type Tile = [u8; TILE * TILE];

/// What the tiles stand for, each by its place in the picture.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// How many tiles the picture holds.
    pub tiles: usize,
    /// The characters of each text style (see `Fonts`), a glyph in a tile's
    /// left columns.
    pub fonts: BTreeMap<String, BTreeMap<char, Vec<usize>>>,
    /// The students' names as a font draws them (see `word`).
    pub names: Vec<Word>,
    /// The letters of the names as the game draws them, from the fixtures;
    /// they stand in for the font's where there are some.
    pub letters: BTreeMap<char, Vec<usize>>,
    /// The shops' names as the menu draws them.
    pub shops: Vec<Word>,
}

/// A word as `word::compare` takes it: what it stands for and how it reads,
/// its width over its height, where its letters part (each 0 to 255 of its
/// width, the ends included) and a tile for each letter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub id: i64,
    pub text: String,
    pub aspect: f32,
    pub bounds: Vec<u8>,
    pub tiles: Vec<usize>,
}

/// The tiles and what they stand for.
#[derive(Debug, Clone, Default)]
pub struct Bank {
    pub index: Index,
    pub tiles: Vec<Tile>,
}

impl Bank {
    /// The bank kept under `data` (the crate's `data` directory).
    pub fn load(data: &std::path::Path) -> Result<Self, String> {
        let read = |name: &str| std::fs::read(data.join(name)).map_err(|e| format!("{name}: {e}"));
        let index = String::from_utf8(read("glyphs.json")?).map_err(|e| e.to_string())?;
        Self::decode(&read("glyphs.png")?, &index)
    }

    /// The bank from its picture (a PNG) and its index (JSON).
    pub fn decode(png: &[u8], index: &str) -> Result<Self, String> {
        let index: Index = serde_json::from_str(index).map_err(|e| e.to_string())?;
        let picture = image::load_from_memory(png)
            .map_err(|e| e.to_string())?
            .to_luma8();
        let tiles = (0..index.tiles)
            .map(|i| {
                let (left, top) = ((i % ROW * TILE) as u32, (i / ROW * TILE) as u32);
                let mut tile = [0; TILE * TILE];
                for (at, value) in tile.iter_mut().enumerate() {
                    let (x, y) = ((at % TILE) as u32, (at / TILE) as u32);
                    *value = picture.get_pixel(left + x, top + y).0[0];
                }
                tile
            })
            .collect();
        Ok(Self { index, tiles })
    }

    /// The picture and the index, as `decode` takes them.
    pub fn encode(&self) -> Result<(Vec<u8>, String), String> {
        let rows = self.tiles.len().div_ceil(ROW).max(1);
        let mut picture = GrayImage::new((ROW * TILE) as u32, (rows * TILE) as u32);
        for (i, tile) in self.tiles.iter().enumerate() {
            let (left, top) = ((i % ROW * TILE) as u32, (i / ROW * TILE) as u32);
            for (at, &value) in tile.iter().enumerate() {
                let (x, y) = ((at % TILE) as u32, (at / TILE) as u32);
                picture.put_pixel(left + x, top + y, Luma([value]));
            }
        }
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
            .write_image(
                picture.as_raw(),
                picture.width(),
                picture.height(),
                image::ExtendedColorType::L8,
            )
            .map_err(|e| e.to_string())?;
        let index = Index {
            tiles: self.tiles.len(),
            ..self.index.clone()
        };
        let json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
        Ok((png, json))
    }

    /// Adds `tile` (or finds it, added before) and gives its place.
    pub fn add(&mut self, tile: Tile) -> usize {
        self.tiles
            .iter()
            .position(|t| *t == tile)
            .unwrap_or_else(|| {
                self.tiles.push(tile);
                self.tiles.len() - 1
            })
    }

    /// The text styles' references (see `Fonts`).
    pub fn fonts(&self) -> Fonts {
        let (gw, gh) = GLYPH;
        let glyph = |&i: &usize| -> Vec<f32> {
            let tile = &self.tiles[i];
            (0..gw * gh)
                .map(|at| f32::from(tile[at / gw * TILE + at % gw]) / 255.0)
                .collect()
        };
        Fonts(
            self.index
                .fonts
                .iter()
                .map(|(style, chars)| {
                    let chars = chars
                        .iter()
                        .map(|(c, tiles)| (*c, tiles.iter().map(glyph).collect()))
                        .collect();
                    (style.clone(), chars)
                })
                .collect(),
        )
    }
}

/// A glyph (see `GLYPH`) as a tile, in its left columns.
pub fn glyph_tile(glyph: &[f32]) -> Tile {
    let (gw, gh) = GLYPH;
    let mut tile = [0; TILE * TILE];
    for at in 0..gw * gh {
        tile[at / gw * TILE + at % gw] = (glyph[at].clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    tile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bank_reads_back_as_written_each_tile_once() {
        let mut bank = Bank::default();
        let mut tile = [0; TILE * TILE];
        tile[17] = 200;
        assert_eq!(bank.add(tile), 0);
        assert_eq!(bank.add([9; TILE * TILE]), 1);
        assert_eq!(bank.add(tile), 0);
        bank.index.names.push(Word {
            id: 10000,
            text: "アル".into(),
            aspect: 2.1,
            bounds: vec![0, 120, 255],
            tiles: vec![0, 1],
        });
        // More tiles than a row, so the picture wraps.
        for v in 0..40 {
            bank.add([v; TILE * TILE]);
        }
        let (png, index) = bank.encode().unwrap();
        let read = Bank::decode(&png, &index).unwrap();
        assert_eq!(read.tiles, bank.tiles);
        assert_eq!(read.index.names, bank.index.names);
    }

    #[test]
    fn a_glyph_takes_a_tiles_left_columns_and_reads_back() {
        let (gw, gh) = GLYPH;
        let glyph: Vec<f32> = (0..gw * gh).map(|i| (i % 3) as f32 / 2.0).collect();
        let mut bank = Bank::default();
        let at = bank.add(glyph_tile(&glyph));
        bank.index
            .fonts
            .entry("digits".into())
            .or_default()
            .insert('7', vec![at]);
        let fonts = bank.fonts();
        let read = &fonts.0["digits"][&'7'][0];
        assert!(read.iter().zip(&glyph).all(|(a, b)| (a - b).abs() < 0.01));
    }
}
