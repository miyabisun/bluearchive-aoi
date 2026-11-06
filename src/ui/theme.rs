//! The design tokens of `DESIGN.md`: Sumi (dark) and Kinari (light), chosen in the menu or
//! following the OS (自動).

use gpui::{App, FontFeatures, Global, Hsla, Pixels, WindowAppearance, hsla, px, rgb, rgba};

use crate::settings::ThemeChoice;

pub const SP_1: Pixels = px(4.);
pub const SP_2: Pixels = px(8.);
pub const SP_3: Pixels = px(12.);
pub const SP_4: Pixels = px(16.);
pub const SP_5: Pixels = px(24.);

pub const RADIUS_SM: Pixels = px(6.);

pub const FS_CAPTION: Pixels = px(12.);
pub const FS_SM: Pixels = px(14.);
pub const FS_MD: Pixels = px(16.);
pub const FS_TITLE: Pixels = px(17.);

/// The OS's UI font, for the design's `system-ui`.
#[cfg(windows)]
pub const FONT: &str = "Yu Gothic UI";
#[cfg(not(windows))]
pub const FONT: &str = ".SystemUIFont";

/// Figures of the same width, so that counts line up (`tabular-nums`).
pub fn tabular() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)]))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub surface: Hsla,
    pub on_surface: Hsla,
    pub muted: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    /// The selection in a text field.
    pub accent_subtle: Hsla,
    pub danger: Hsla,
    pub danger_subtle: Hsla,
    pub ok: Hsla,
    pub warn: Hsla,
    /// The four tiers of a material, 初級 to 最上級.
    pub tiers: [Hsla; 4],
    pub eleph: Hsla,
    pub wash_base: Hsla,
    /// How bright a school's emblem (white in its picture) is drawn, as the text.
    pub emblem: f32,
}

impl Global for Theme {}

impl Theme {
    pub fn sumi() -> Self {
        Self {
            dark: true,
            surface: rgb(0x0019_1919).into(),
            on_surface: rgb(0x00e6_e6e6).into(),
            muted: rgb(0x009a_9a9a).into(),
            border: rgb(0x0033_3333).into(),
            accent: rgb(0x005c_b3ff).into(),
            accent_subtle: rgba(0x5cb3_ff40).into(),
            danger: rgb(0x00ff_6b6b).into(),
            danger_subtle: rgb(0x003a_1a1a).into(),
            ok: rgb(0x006f_cf8a).into(),
            warn: rgb(0x00f0_a050).into(),
            tiers: [
                rgb(0x002b_2b2b).into(),
                rgb(0x001b_2b3f).into(),
                rgb(0x003a_3000).into(),
                rgb(0x003e_2066).into(),
            ],
            eleph: rgb(0x004a_2438).into(),
            wash_base: rgb(0x0023_2323).into(),
            emblem: 0.9,
        }
    }

    pub fn kinari() -> Self {
        Self {
            dark: false,
            surface: rgb(0x00fa_f6ef).into(),
            on_surface: rgb(0x003a_2f28).into(),
            muted: rgb(0x006f_6257).into(),
            border: rgb(0x00e3_d9c9).into(),
            accent: rgb(0x001a_5fa8).into(),
            accent_subtle: rgba(0x1a5f_a833).into(),
            danger: rgb(0x009c_2b1d).into(),
            danger_subtle: rgb(0x00f9_e9e4).into(),
            ok: rgb(0x001d_6b35).into(),
            warn: rgb(0x009a_4a06).into(),
            tiers: [
                rgb(0x00eb_e7e0).into(),
                rgb(0x00dc_e8f6).into(),
                rgb(0x00f5_e8be).into(),
                rgb(0x00eb_dff5).into(),
            ],
            eleph: rgb(0x00f9_dfe8).into(),
            wash_base: rgb(0x00f6_efe0).into(),
            emblem: 0.23,
        }
    }

    /// The theme chosen in the menu, the OS's for 自動.
    pub fn chosen(choice: ThemeChoice, appearance: WindowAppearance) -> Self {
        match choice {
            ThemeChoice::Light => Self::kinari(),
            ThemeChoice::Dark => Self::sumi(),
            ThemeChoice::Auto => match appearance {
                WindowAppearance::Light | WindowAppearance::VibrantLight => Self::kinari(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::sumi(),
            },
        }
    }

    pub fn get(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_else(Self::sumi)
    }
}

/// What covers the window under a dialog: black at 50%.
pub fn backdrop() -> Hsla {
    hsla(0., 0., 0., 0.5)
}
