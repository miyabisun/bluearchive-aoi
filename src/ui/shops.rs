//! The shop tab: each shop's goods and how many times each can still be bought.

use gpui::{AnyElement, Context, IntoElement as _, div};

use super::{Aoi, theme::Theme};

impl Aoi {
    pub(super) fn shops_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }
}
