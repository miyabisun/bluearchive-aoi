//! The work tab: the daily plays against what is held, a fold each.

use gpui::{AnyElement, Context, IntoElement as _, ScrollHandle, div};

use super::{Aoi, theme::Theme};

/// Brings the fold `key` to the top of the tab's frame.
pub fn scroll_to_fold(_content: &ScrollHandle, _key: &str) {}

impl Aoi {
    pub(super) fn work_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }
}
