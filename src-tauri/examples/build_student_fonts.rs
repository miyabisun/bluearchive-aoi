//! Writes `data/student-fonts.json`: the references of each text style of the
//! student screen, taken from the fixtures labeled `"role": "tune"` in
//! `tests/fixtures/students.json`. Evaluation fixtures are never used here.
//! The cuts of a character are grouped by likeness and each group averaged, so
//! a character drawn in more than one way keeps a reference for each.
//! Run with `cargo run --example build_student_fonts`.

use std::{collections::BTreeMap, path::Path};

use aoi::student::{
    self, Label,
    glyph::{self, Fonts},
};

/// A cut farther than this from every group of its character starts a group.
const JOIN: f32 = 10.0;

/// A group of alike cuts: their sum and how many.
struct Group(Vec<f32>, u32);

impl Group {
    fn mean(&self) -> Vec<f32> {
        self.0.iter().map(|v| v / self.1 as f32).collect()
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("tests/fixtures");
    let labels: Vec<Label> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("students.json")).unwrap()).unwrap();
    let mut groups: BTreeMap<String, BTreeMap<char, Vec<Group>>> = BTreeMap::new();
    for label in labels.iter().filter(|l| l.role == "tune") {
        let image = image::open(dir.join(&label.image)).unwrap().to_rgb8();
        for (style, area, text) in label.texts() {
            let shapes = student::shapes(&image, style, area).unwrap();
            if shapes.len() != text.chars().count() {
                println!(
                    "{} {style} {text:?}: {} characters cut out",
                    label.image,
                    shapes.len()
                );
                continue;
            }
            for (c, shape) in text.chars().zip(shapes) {
                let found = groups
                    .entry(style.to_owned())
                    .or_default()
                    .entry(c)
                    .or_default();
                let nearest = found
                    .iter_mut()
                    .map(|g| (glyph::distance(&g.mean(), &shape), g))
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .filter(|(d, _)| *d <= JOIN);
                match nearest {
                    Some((_, group)) => {
                        group.0.iter_mut().zip(&shape).for_each(|(s, v)| *s += v);
                        group.1 += 1;
                    }
                    None => found.push(Group(shape, 1)),
                }
            }
        }
    }
    let mut fonts = Fonts::default();
    for (style, chars) in groups {
        let counts: Vec<String> = chars
            .iter()
            .map(|(c, gs)| {
                format!(
                    "{c}:{}",
                    gs.iter()
                        .map(|g| g.1.to_string())
                        .collect::<Vec<_>>()
                        .join("+")
                )
            })
            .collect();
        println!("{style}: {}", counts.join(" "));
        // Each reference drawn side by side, to check by eye that no label went
        // to the wrong character.
        let refs: Vec<(char, Vec<f32>)> = chars
            .iter()
            .flat_map(|(c, gs)| gs.iter().map(|g| (*c, g.mean())))
            .collect();
        let (gw, gh) = glyph::GLYPH;
        println!(
            "  {}",
            refs.iter()
                .map(|(c, _)| c.to_string() + &" ".repeat(gw))
                .collect::<String>()
        );
        for y in 0..gh {
            let row: String = refs
                .iter()
                .map(|(_, r)| {
                    r[y * gw..(y + 1) * gw]
                        .iter()
                        .map(|v| if *v > 0.5 { '#' } else { '.' })
                        .collect::<String>()
                        + " "
                })
                .collect();
            println!("  {row}");
        }
        let font = chars
            .into_iter()
            .map(|(c, gs)| {
                let refs = gs
                    .iter()
                    .map(|g| {
                        g.mean()
                            .iter()
                            .map(|v| (v * 1000.0).round() / 1000.0)
                            .collect()
                    })
                    .collect();
                (c, refs)
            })
            .collect();
        fonts.0.insert(style, font);
    }
    std::fs::write(
        root.join("data/student-fonts.json"),
        serde_json::to_string(&fonts).unwrap() + "\n",
    )
    .unwrap();
}
