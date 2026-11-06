//! The parts the tabs share: text the tests can find, hover tooltips, menu items, notices,
//! buttons, folds (a title that opens and closes what is under it) and table cells.

use gpui::{
    AnyView, App, AppContext as _, Div, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Role, SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, Window,
    div, prelude::FluentBuilder as _, px,
};

use super::{
    icons::{Glyph, glyph},
    theme::{FS_SM, RADIUS_SM, SP_1, SP_2, SP_3, Theme},
};

/// A piece of text that the window's tests can find (`text:<the text>`).
pub fn text(text: impl Into<SharedString>) -> Div {
    let text = text.into();
    let selector = text.clone();
    div()
        .debug_selector(move || format!("text:{selector}"))
        .child(text)
}

/// A piece of text that UI Automation reads (a table's cell, a heading): it has an id and a
/// role, so it is in the tree with its words as its name.
pub fn read(id: impl Into<SharedString>, words: impl Into<SharedString>) -> Stateful<Div> {
    let words = words.into();
    let selector = words.clone();
    div()
        .id(gpui::ElementId::Name(id.into()))
        .role(Role::Label)
        .aria_label(words.clone())
        .debug_selector(move || format!("text:{selector}"))
        .child(words)
}

/// What a hover shows: lines of text in the menu's face.
pub struct Tooltip {
    text: SharedString,
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        div()
            .font_family(super::theme::FONT)
            .max_w(px(360.))
            .px(SP_2)
            .py(SP_1)
            .border_1()
            .border_color(theme.border)
            .rounded(RADIUS_SM)
            .bg(theme.surface)
            .text_color(theme.on_surface)
            .text_size(FS_SM)
            .children(
                self.text
                    .split('\n')
                    .map(|line| div().child(line.to_owned())),
            )
    }
}

/// A tooltip of `text` (lines apart by `\n`), for `.tooltip(...)`; none for empty text.
pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_window, cx| cx.new(|_| Tooltip { text: text.clone() }).into()
}

/// A menu item: `checked` for a choice of one (a tick before the chosen one, its room kept
/// before the others).
pub fn menu_item(
    theme: &Theme,
    id: &str,
    label: &str,
    checked: Option<bool>,
    disabled: bool,
) -> Stateful<Div> {
    let hover = theme.wash_base;
    let selector = format!("menu-{id}");
    div()
        .id(SharedString::from(selector.clone()))
        .debug_selector(move || selector.clone())
        .role(if checked.is_some() {
            Role::MenuItemRadio
        } else {
            Role::MenuItem
        })
        .aria_label(label.to_owned())
        .when_some(checked, |item, checked| {
            item.aria_toggled(if checked {
                gpui::Toggled::True
            } else {
                gpui::Toggled::False
            })
        })
        .when(disabled, |item| item.aria_description("押せません"))
        .tab_stop(!disabled)
        .flex()
        .px(SP_3)
        .py(SP_2)
        .text_size(FS_SM)
        .text_color(if disabled {
            theme.muted
        } else {
            theme.on_surface
        })
        .when(!disabled, |item| {
            item.cursor_pointer()
                .hover(move |s| s.bg(hover))
                .focus(move |s| s.bg(hover))
        })
        .when_some(checked, |item, checked| {
            item.child(
                div()
                    .w(px(14. * 1.4))
                    .flex_none()
                    .when(!checked, gpui::Styled::invisible)
                    .child("✓"),
            )
        })
        .child(label.to_owned())
}

/// A failure in the page: danger on danger-subtle (`error-banner`).
pub fn error(theme: &Theme, id: &str, message: impl Into<SharedString>) -> Stateful<Div> {
    let message = message.into();
    let selector = id.to_owned();
    div()
        .id(SharedString::from(id.to_owned()))
        .debug_selector(move || selector.clone())
        .role(Role::Alert)
        .aria_label(message.clone())
        .flex_none()
        .mb(px(16.))
        .p(SP_3)
        .rounded(RADIUS_SM)
        .bg(theme.danger_subtle)
        .text_color(theme.danger)
        .text_size(FS_SM)
        .child(text(message))
}

/// A quiet line in the page: body-sm, muted.
pub fn quiet(theme: &Theme, id: &str, message: impl Into<SharedString>) -> Stateful<Div> {
    let message = message.into();
    let selector = id.to_owned();
    div()
        .id(SharedString::from(id.to_owned()))
        .debug_selector(move || selector.clone())
        .role(Role::Status)
        .aria_label(message.clone())
        .flex_none()
        .mb(px(16.))
        .text_size(FS_SM)
        .text_color(theme.muted)
        .child(text(message))
}

/// A button drawn as its border alone: the accent's for the main action, else the border's.
pub fn outlined_button(theme: &Theme, id: &str, label: &str, primary: bool) -> Stateful<Div> {
    let (border, color) = if primary {
        (theme.accent, theme.accent)
    } else {
        (theme.border, theme.on_surface)
    };
    let focused = if primary { theme.accent } else { theme.muted };
    let selector = id.to_owned();
    div()
        .id(SharedString::from(id.to_owned()))
        .debug_selector(move || selector.clone())
        .role(Role::Button)
        .aria_label(label.to_owned())
        .tab_stop(true)
        .flex_none()
        .px(SP_3)
        .py(px(2.))
        .border_1()
        .border_color(border)
        .rounded(RADIUS_SM)
        .text_color(color)
        .cursor_pointer()
        .focus(move |s| s.border_color(focused))
        .child(label.to_owned())
}

/// A fold's title: a triangle turned down while open, then the title; pressing it (or Enter
/// and Space on it) opens and closes the fold. Whether it is open goes in its description, not
/// as an expandable state: GPUI answers UI Automation's invoke alone, and an expandable node is
/// offered to expand instead. Its text cannot be selected, so a double click
/// opens and closes it and selects nothing.
pub fn fold_title(theme: &Theme, key: &str, open: bool) -> Stateful<Div> {
    let selector = format!("fold-{key}");
    div()
        .id(SharedString::from(selector.clone()))
        .debug_selector(move || selector.clone())
        .role(Role::Button)
        .when(open, |t| t.aria_description("開いています"))
        .tab_stop(true)
        .flex()
        .items_center()
        .gap(SP_1)
        .cursor_pointer()
        .child(glyph(Glyph::Chevron, px(12.), theme.muted).when(open, |c| {
            c.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.25)))
        }))
}

/// What a tab says before anything of its list is read.
pub fn empty(theme: &Theme, id: &str, message: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("empty-{id}")))
        .role(Role::Label)
        .aria_label(message.to_owned())
        .text_color(theme.muted)
        .child(text(message.to_owned()))
}
