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
        (Self::Auto, "自動"),
        (Self::Light, "ライト"),
        (Self::Dark, "ダーク"),
    ];
}

/// The mouse cursor (the user's of 2026-11-03): Windows' own (the default), or an
/// arrow after the game's in blue, red or green.
/// Where the game is read from (the settings page's 配信方法).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feed {
    /// The PC's game, its window captured (Windows alone runs it).
    Window,
    /// An Android phone's screen over ADB.
    Phone,
    /// A streaming app's stream.
    Cast,
}

impl Feed {
    /// The choices offered where the app runs, with their names, the default first.
    pub fn offered() -> &'static [(Self, &'static str)] {
        const ALL: [(Feed, &str); 3] = [
            (Feed::Window, "PC版アプリ"),
            (Feed::Phone, "Androidとの通信"),
            (Feed::Cast, "配信アプリとの連携"),
        ];
        if cfg!(windows) { &ALL } else { &ALL[1..] }
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::offered()[0].0
    }
}

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
    /// Where the game is read from; none chosen yet in a file of before it was asked.
    pub feed: Option<Feed>,
    pub window: Option<Placement>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Auto,
            cursor: CursorChoice::None,
            work_follow: true,
            feed: None,
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
        if let Some(feed) = fields.get("feed") {
            settings.feed = Feed::deserialize(feed.clone())
                .ok()
                .filter(|feed| Feed::offered().iter().any(|(offered, _)| offered == feed));
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
    fn nothing_kept_or_nothing_readable_means_the_defaults() {
        for kept in ["", "{not json"] {
            let settings = Settings::parse(kept);
            assert_eq!(settings.theme, ThemeChoice::Auto, "the OS's theme");
            assert_eq!(settings.cursor, CursorChoice::None, "Windows' own cursor");
            assert!(settings.work_follow, "the work tab follows the game");
            assert_eq!(
                settings.feed, None,
                "where the game is read from not asked yet"
            );
            assert_eq!(settings.window, None, "no place kept");
        }
    }

    #[test]
    fn the_choices_kept_come_back() {
        let kept = Settings {
            theme: ThemeChoice::Light,
            cursor: CursorChoice::Green,
            work_follow: false,
            feed: Some(Feed::Phone),
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

    /// Where the game is read from is kept; a file of before it was asked keeps none, and
    /// one naming what is not offered here (the PC's game on a Mac) none either.
    #[test]
    fn where_the_game_is_read_from_is_kept() {
        assert_eq!(Settings::parse(r#"{"theme":"dark"}"#).feed, None);
        assert_eq!(Settings::parse(r#"{"feed":"cast"}"#).feed, Some(Feed::Cast));
        assert_eq!(Settings::parse(r#"{"feed":"tv"}"#).feed, None);
        let window = Settings::parse(r#"{"feed":"window"}"#).feed;
        assert_eq!(window, cfg!(windows).then_some(Feed::Window));
        // The PC's game where there is one, first and by default; else a phone.
        let offered: Vec<&str> = Feed::offered().iter().map(|(_, name)| *name).collect();
        if cfg!(windows) {
            assert_eq!(
                offered,
                ["PC版アプリ", "Androidとの通信", "配信アプリとの連携"]
            );
            assert_eq!(Feed::default(), Feed::Window);
        } else {
            assert_eq!(offered, ["Androidとの通信", "配信アプリとの連携"]);
            assert_eq!(Feed::default(), Feed::Phone);
        }
    }
}
