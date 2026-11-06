//! The item tab: everything held as tiles, then tables by kind.

use gpui::{AnyElement, Context, IntoElement as _, div};

use super::{Aoi, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    All,
    Skills,
    Ooparts,
    Currency,
}

impl Aoi {
    pub(super) fn items_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }
}
