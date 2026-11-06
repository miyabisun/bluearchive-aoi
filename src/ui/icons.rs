//! What the window draws from pictures, served to GPUI as assets: the item, equipment and
//! school icons built into the program (`icon/<name>`), a school's emblem at the theme's
//! brightness (`emblem/<brightness>/<name>`), the line icons drawn in the text colour
//! (`glyph/<name>.svg`) and the game's stars in their own colours (`star/<kind>.svg`).

use std::{borrow::Cow, io::Cursor};

use gpui::{AssetSource, Hsla, Img, Pixels, SharedString, Styled, Svg, img, svg};

/// A line icon, drawn in the text colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// The game's window, a phone or a streaming app: where the frames read come from.
    Window,
    Phone,
    Cast,
    Menu,
    Chevron,
    Close,
}

impl Glyph {
    fn name(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Phone => "phone",
            Self::Cast => "cast",
            Self::Menu => "menu",
            Self::Chevron => "chevron",
            Self::Close => "close",
        }
    }

    fn svg(self) -> &'static str {
        match self {
            Self::Window => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><rect x="1.25" y="2.25" width="13.5" height="9" rx="1"/><path d="M8 11.25v3M5 14.5h6"/></svg>"#
            }
            Self::Phone => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><rect x="4.25" y="1.25" width="7.5" height="13.5" rx="1.5"/><path d="M7 12h2"/></svg>"#
            }
            Self::Cast => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><circle cx="8" cy="8" r="1.5" fill="black" stroke="none"/><path d="M5.2 5.2a4 4 0 0 0 0 5.6M10.8 5.2a4 4 0 0 1 0 5.6"/><path d="M3 3a7 7 0 0 0 0 10M13 3a7 7 0 0 1 0 10"/></svg>"#
            }
            Self::Menu => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 7h16M4 12h16M4 17h16" stroke="black" stroke-width="2" stroke-linecap="round"/></svg>"#
            }
            Self::Chevron => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M4 2l5 4-5 4z" fill="black"/></svg>"#
            }
            Self::Close => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M3 3l6 6M9 3l-6 6" stroke="black" stroke-width="1.6" stroke-linecap="round"/></svg>"#
            }
        }
    }

    const ALL: [Self; 6] = [
        Self::Window,
        Self::Phone,
        Self::Cast,
        Self::Menu,
        Self::Chevron,
        Self::Close,
    ];
}

/// `glyph` at `size`, in `color`.
pub fn glyph(glyph: Glyph, size: Pixels, color: Hsla) -> Svg {
    svg()
        .path(format!("glyph/{}.svg", glyph.name()))
        .size(size)
        .flex_none()
        .text_color(color)
}

/// The stars the game draws, cut from its student screen by the reading's own inks
/// (`student.rs`'s STAR and WEAPON_STAR, 2026-10-22 at 1240 high): a five-pointed star
/// with its rim and faces lit and shaded; the student's gold in an orange rim, the
/// weapon's cyan in a grey-blue one. In the units of that ink box, with room for the rim.
fn star(fill: (&str, &str), rim: &str, light: f32, shade: (&str, f32)) -> String {
    const OUTLINE: &str =
        "M14 0.69L18.33 9.45L28 10.86L21 17.68L22.65 27.31L14 22.77L5.35 27.31L7 17.68L0 10.86L9.67 9.45Z";
    const LIT: &str = "M14 15.41L9.67 9.45L14 0.69ZM14 15.41L18.33 9.45L28 10.86ZM14 15.41L21 17.68L22.65 27.31ZM14 15.41L14 22.77L5.35 27.31ZM14 15.41L7 17.68L0 10.86Z";
    const SHADED: &str = "M14 15.41L14 0.69L18.33 9.45ZM14 15.41L28 10.86L21 17.68ZM14 15.41L22.65 27.31L14 22.77ZM14 15.41L5.35 27.31L7 17.68ZM14 15.41L0 10.86L9.67 9.45Z";
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-2 -2 32 32" width="64" height="64"><defs><linearGradient id="f" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs><path d="{OUTLINE}" fill="url(#f)" stroke="{rim}" stroke-width="2" stroke-linejoin="round"/><path d="{LIT}" fill="#fff" opacity="{light}"/><path d="{SHADED}" fill="{}" opacity="{}"/></svg>"##,
        fill.0, fill.1, shade.0, shade.1
    )
}

/// A star of the student (gold) or of the weapon (cyan), `size` square.
pub fn star_image(weapon: bool, size: Pixels) -> Img {
    img(if weapon {
        "star/weapon.svg"
    } else {
        "star/student.svg"
    })
    .size(size)
    .flex_none()
}

/// The built-in icon `name` (an item, equipment or school), `size` square.
pub fn icon(name: &str, size: Pixels) -> Img {
    img(SharedString::from(format!("icon/{name}")))
        .size(size)
        .flex_none()
}

/// A school's emblem, white in its picture, drawn at the theme's text brightness.
pub fn emblem(name: &str, brightness: f32, size: Pixels) -> Img {
    img(SharedString::from(format!("emblem/{brightness}/{name}")))
        .size(size)
        .flex_none()
}

/// The picture `name` with each colour channel scaled by `brightness` (as CSS's
/// `filter: brightness()`), as PNG.
fn brightened(name: &str, brightness: f32) -> Option<Vec<u8>> {
    let (bytes, _) = crate::icon(name)?;
    let mut picture = image::load_from_memory(bytes).ok()?.to_rgba8();
    for pixel in picture.pixels_mut() {
        for channel in &mut pixel.0[..3] {
            *channel = (f32::from(*channel) * brightness).round().min(255.) as u8;
        }
    }
    let mut png = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("icon/") {
            return Ok(crate::icon(name).map(|(bytes, _)| Cow::Borrowed(bytes)));
        }
        if let Some((brightness, name)) = path
            .strip_prefix("emblem/")
            .and_then(|rest| rest.split_once('/'))
        {
            let brightness: f32 = brightness.parse().unwrap_or(1.);
            return Ok(brightened(name, brightness).map(Cow::Owned));
        }
        match path {
            "star/student.svg" => {
                return Ok(Some(Cow::Owned(
                    star(("#fcc63a", "#fed92c"), "#e8890c", 0.45, ("#d98a00", 0.18)).into_bytes(),
                )));
            }
            "star/weapon.svg" => {
                return Ok(Some(Cow::Owned(
                    star(("#6ce3fe", "#79e4fc"), "#6489a3", 0.5, ("#2a9cc8", 0.2)).into_bytes(),
                )));
            }
            _ => {}
        }
        let Some(name) = path
            .strip_prefix("glyph/")
            .and_then(|p| p.strip_suffix(".svg"))
        else {
            return Ok(None);
        };
        Ok(Glyph::ALL
            .into_iter()
            .find(|g| g.name() == name)
            .map(|g| Cow::Borrowed(g.svg().as_bytes())))
    }

    fn list(&self, _path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
