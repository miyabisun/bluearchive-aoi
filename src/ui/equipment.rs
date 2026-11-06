//! The equipment tab: every piece held as tiles, then the blueprints and the enhancement
//! materials by kind.

use gpui::{AnyElement, Context, IntoElement as _, div};

use super::{Aoi, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    All,
    Blueprints,
    Enhancements,
}

impl Aoi {
    pub(super) fn equipment_tab(&mut self, _theme: &Theme, _cx: &mut Context<Self>) -> AnyElement {
        div().into_any_element()
    }
}
