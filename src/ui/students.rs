//! The student tab: the students read, ordered by a column, each markable for training.

use gpui::{AnyElement, Context, IntoElement as _, div};

use super::{Aoi, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Level,
}

impl Aoi {
    pub(super) fn students_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }
}
