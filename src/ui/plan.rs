//! The training tab: the students marked, their targets, what they take against what is held,
//! and how to make up what is short.

use gpui::{AnyElement, Context, IntoElement as _, div};

use super::{Aoi, StarMenu, theme::Theme};

impl Aoi {
    pub(super) fn plan_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }

    pub(super) fn star_menu_view(
        &mut self,
        _menu: StarMenu,
        _theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }
}
