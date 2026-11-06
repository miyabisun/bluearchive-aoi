//! TEMPORARY: replaced by the port of view.js.

use crate::{android::Link, app::Snapshot, broadcast::Cast, strategy::PlanShop, watch::Place};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Active,
    Idle,
    Danger,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Window,
    Phone,
    Cast,
}

pub struct StatusView {
    pub label: String,
    pub tone: Tone,
    pub source: Option<Source>,
    pub title: String,
}

pub fn status_view(_snapshot: &Snapshot) -> StatusView {
    StatusView { label: String::new(), tone: Tone::Idle, source: None, title: String::new() }
}

pub fn work_fold(_place: Option<&Place>, _month_end: &[PlanShop]) -> Option<String> {
    None
}

pub fn update_text_checking() -> String { String::new() }
pub fn update_text_latest(_c: &str) -> String { String::new() }
pub fn update_text_failed(_e: &str) -> String { String::new() }
pub fn update_text_available(_c: &str, _l: Option<&str>) -> String { String::new() }
pub fn update_text_installing(_l: Option<&str>) -> String { String::new() }
pub fn phone_line(_l: &Link) -> String { String::new() }
pub fn cast_line(_c: &Cast) -> String { String::new() }
pub fn cast_note(_c: &Cast) -> Option<String> { None }
