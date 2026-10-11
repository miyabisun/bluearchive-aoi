//! Cuts the icon of an item SchaleDB lacks out of a screenshot, as a 146x116 PNG
//! with transparency, for `data/extra/icons`.
//!
//! Run with `cargo run --example make_icon -- <screenshot> <cell> <out.png> [shape]`.
//! `cell` counts the fully visible cells from the top left. With `shape` (a SchaleDB
//! icon of the same outline, such as the normal recruitment ticket for an archive
//! one) its transparency is used; otherwise the card's background is taken away by
//! filling it in from the canvas border.

use std::{collections::VecDeque, path::Path};

use aoi::{grid, icon::IconMatcher};
use image::{Rgba, RgbaImage};

/// Neighbouring background pixels differ by at most this much (summed channels).
const BACKGROUND_STEP: u32 = 30;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = image::open(&args[1]).unwrap().to_rgb8();
    let cell: usize = args[2].parse().unwrap();
    let card = grid::find_cards(&image)[cell];
    let canvas = IconMatcher::canvas(&image, &card);
    let (width, height) = canvas.dimensions();
    let mut icon = RgbaImage::from_fn(width, height, |x, y| {
        let p = canvas.get_pixel(x, y).0;
        Rgba([p[0], p[1], p[2], 255])
    });
    if let Some(shape) = args.get(4) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/schaledb/icons")
            .join(format!("{shape}.webp"));
        let shape = image::open(path).unwrap().to_rgba8();
        for (x, y, p) in icon.enumerate_pixels_mut() {
            p.0[3] = shape.get_pixel(x, y).0[3];
        }
    } else {
        let step = |a: [u8; 3], b: [u8; 3]| -> u32 {
            a.iter()
                .zip(b)
                .map(|(&x, y)| u32::from(x.abs_diff(y)))
                .sum()
        };
        let mut background = vec![false; (width * height) as usize];
        let mut queue: VecDeque<(u32, u32)> = (0..width)
            .flat_map(|x| [(x, 0), (x, height - 1)])
            .chain((0..height).flat_map(|y| [(0, y), (width - 1, y)]))
            .collect();
        for &(x, y) in &queue {
            background[(y * width + x) as usize] = true;
        }
        while let Some((x, y)) = queue.pop_front() {
            let here = canvas.get_pixel(x, y).0;
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let i = (ny * width + nx) as usize;
                if !background[i] && step(here, canvas.get_pixel(nx, ny).0) <= BACKGROUND_STEP {
                    background[i] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        // Thin card edges survive the fill; only the largest piece is the icon.
        let mut piece = vec![usize::MAX; background.len()];
        let mut sizes = Vec::new();
        for start in 0..background.len() {
            if background[start] || piece[start] != usize::MAX {
                continue;
            }
            let id = sizes.len();
            let mut size = 0;
            let mut stack = vec![start];
            piece[start] = id;
            while let Some(i) = stack.pop() {
                size += 1;
                let (x, y) = ((i as u32 % width) as i32, (i as u32 / width) as i32);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let j = (ny as u32 * width + nx as u32) as usize;
                    if !background[j] && piece[j] == usize::MAX {
                        piece[j] = id;
                        stack.push(j);
                    }
                }
            }
            sizes.push(size);
        }
        let largest = (0..sizes.len()).max_by_key(|&i| sizes[i]).unwrap_or(0);
        for (i, p) in icon.pixels_mut().enumerate() {
            if piece[i] != largest {
                p.0[3] = 0;
            }
        }
    }
    icon.save(&args[3]).unwrap();
}
