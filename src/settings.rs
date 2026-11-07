//! What the user chose in the window's menu, kept for the next start in
//! `settings.json` beside `state.json`: the theme, the mouse cursor, whether
//! the work tab follows the game, and where the window was left. A file missing or
//! unreadable, or a value this version does not know, means the default for that choice.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The theme: light, dark, or the OS's (自動, the default).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    Light,
    Dark,
    #[default]
    Auto,
}

impl ThemeChoice {
    /// The menu's choices, in order, with their names.
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Light, "ライト"),
        (Self::Dark, "ダーク"),
        (Self::Auto, "自動"),
    ];
}

/// The mouse cursor (the user's of 2026-11-03): Windows' own (the default), or an
/// arrow after the game's in blue, red or green.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorChoice {
    #[default]
    None,
    Blue,
    Red,
    Green,
}

impl CursorChoice {
    /// The menu's choices, in order, with their names.
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::None, "なし"),
        (Self::Blue, "青"),
        (Self::Red, "赤"),
        (Self::Green, "緑"),
    ];

    /// The arrow's colour, none for Windows' own cursor.
    pub fn palette(self) -> Option<crate::cursor::Palette> {
        use crate::cursor::Palette;
        match self {
            Self::None => None,
            Self::Blue => Some(Palette::Blue),
            Self::Red => Some(Palette::Red),
            Self::Green => Some(Palette::Green),
        }
    }
}

/// Whether the work tab follows the game (the user's of 2026-11-06 「お仕事の自動開閉」).
pub const WORK_FOLLOW: [(bool, &str); 2] = [(true, "する"), (false, "しない")];

/// Where the window was left (the user's of 2026-11-07 「配置されているモニター忘れてメインモニターに
/// 移動する」): the monitor, by the id GPUI gives it (on Windows made from the monitor's device
/// name), and the window's place and size in that monitor's logical pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub display: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
    pub cursor: CursorChoice,
    pub work_follow: bool,
    pub window: Option<Placement>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Auto,
            cursor: CursorChoice::None,
            work_follow: true,
            window: None,
        }
    }
}

/// One choice read leniently: a value this version does not know is the default.
fn lenient<'de, T: Deserialize<'de> + Default>(value: serde_json::Value) -> T {
    T::deserialize(value).unwrap_or_default()
}

impl Settings {
    /// The settings `text` holds, each choice that cannot be read at its default.
    pub fn parse(text: &str) -> Self {
        let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(text) else {
            return Self::default();
        };
        let mut settings = Self::default();
        if let Some(theme) = fields.get("theme") {
            settings.theme = lenient(theme.clone());
        }
        if let Some(cursor) = fields.get("cursor") {
            settings.cursor = lenient(cursor.clone());
        }
        if let Some(serde_json::Value::Bool(follow)) = fields.get("work_follow") {
            settings.work_follow = *follow;
        }
        if let Some(window) = fields.get("window") {
            settings.window = Placement::deserialize(window.clone())
                .ok()
                .filter(|p| p.width > 0 && p.height > 0);
        }
        settings
    }

    /// The settings kept at `path`, the defaults when there are none.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).map_or_else(|_| Self::default(), |text| Self::parse(&text))
    }

    /// Keeps the settings at `path` for the next start.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        crate::keep::write(path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_kept_means_the_defaults() {
        let settings = Settings::parse("");
        assert_eq!(settings.theme, ThemeChoice::Auto, "the OS's theme");
        assert_eq!(settings.cursor, CursorChoice::None, "Windows' own cursor");
        assert!(settings.work_follow, "the work tab follows the game");
    }

    #[test]
    fn the_choices_kept_come_back() {
        let kept = Settings {
            theme: ThemeChoice::Light,
            cursor: CursorChoice::Green,
            work_follow: false,
            window: None,
        };
        let text = serde_json::to_string(&kept).unwrap();
        assert_eq!(Settings::parse(&text), kept, "read back as kept");
    }

    #[test]
    fn a_value_unknown_is_that_choice_s_default_and_leaves_the_others() {
        let settings =
            Settings::parse(r#"{"theme": "sepia", "cursor": "red", "work_follow": "maybe"}"#);
        assert_eq!(settings.theme, ThemeChoice::Auto, "unknown theme");
        assert_eq!(settings.cursor, CursorChoice::Red, "the cursor kept");
        assert!(settings.work_follow, "unreadable follow");
    }

    #[test]
    fn the_window_s_place_kept_comes_back() {
        let kept = Settings {
            window: Some(Placement {
                display: "6f1c0b52-3c5a-5a8e-9d2e-4b1f0c7a9e10".to_owned(),
                x: -1580,
                y: 360,
                width: 734,
                height: 858,
            }),
            ..Settings::default()
        };
        let text = serde_json::to_string(&kept).unwrap();
        assert_eq!(Settings::parse(&text), kept, "read back as kept");
    }

    #[test]
    fn a_place_that_cannot_be_read_is_none_and_leaves_the_choices() {
        for window in [
            r#""left""#,
            r#"{"display": "a", "x": 0, "y": 0}"#,
            r#"{"display": "a", "x": 0, "y": 0, "width": 0, "height": 820}"#,
        ] {
            let settings = Settings::parse(&format!(r#"{{"theme": "dark", "window": {window}}}"#));
            assert_eq!(settings.window, None, "{window} is no place");
            assert_eq!(
                settings.theme,
                ThemeChoice::Dark,
                "the theme kept beside {window}"
            );
        }
    }

    #[test]
    fn a_broken_file_means_the_defaults() {
        assert_eq!(
            Settings::parse("{not json"),
            Settings::default(),
            "nothing readable"
        );
    }
}
