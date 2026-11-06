//! The pieces the item, equipment and work tabs draw their tables from: tiles of icons with
//! their counts, a kind's tiers on their colours, and how many students what is held raises.

use gpui::{
    StyledImage as _, AnyElement, Div, FontWeight, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, Role, SharedString, StatefulInteractiveElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px,
};

use super::{
    icons,
    theme::{FS_CAPTION, RADIUS_SM, SP_1, SP_2, Theme, tabular},
    widgets::{text, tooltip},
};
use crate::{view, work::HeldCell};

/// How many columns of at least `min` fit `width` `gap` apart, and how wide each is.
pub fn columns(width: Pixels, min: Pixels, gap: Pixels) -> (usize, Pixels) {
    let width = f32::from(width).max(0.);
    let (min, gap) = (f32::from(min), f32::from(gap));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // A few columns at most.
    let count = (((width + gap) / (min + gap)).floor() as usize).max(1);
    #[allow(clippy::cast_precision_loss)]
    let column = (width - gap * (count - 1) as f32) / count as f32;
    (count, px(column.max(0.)))
}

/// The width the tab's content has: the window's, less the content's sides.
pub fn content_width(window: &gpui::Window) -> Pixels {
    window.viewport_size().width - super::theme::SP_3 * 2.
}

/// What a tile shows: its icon, a badge at the left foot (an equipment's tier), the count at
/// the right foot, its face (a rarity's tier or an eleph's pink), and its name on hover.
pub struct Tile {
    pub key: String,
    pub icon: String,
    pub title: String,
    pub badge: Option<String>,
    pub count: Option<String>,
    pub face: Option<Hsla>,
    /// Bought up: greyed.
    pub sold: bool,
}

/// Tiles in a grid of columns at least 64 px wide (a 64 px icon, its padding over), 8 px apart,
/// filling `width`.
pub fn tiles(theme: &Theme, width: Pixels, tiles: Vec<Tile>, under: impl Fn(usize) -> Option<AnyElement>) -> Div {
    tiles_at(theme, width, px(64.), tiles, under)
}

/// Tiles in a grid of columns at least `min` wide, 8 px apart, filling `width`; `under` adds
/// what goes under a tile's icon (a good's offers).
pub fn tiles_at(theme: &Theme, width: Pixels, min: Pixels, tiles: Vec<Tile>, under: impl Fn(usize) -> Option<AnyElement>) -> Div {
    let (count, column) = columns(width, min, SP_2);
    let mut rows = Vec::new();
    let mut tiles = tiles.into_iter().enumerate().peekable();
    while tiles.peek().is_some() {
        let row: Vec<_> = tiles.by_ref().take(count).collect();
        rows.push(
            div().flex().gap(SP_2).children(row.into_iter().map(|(i, tile)| {
                let below = under(i);
                tile_view(theme, column, tile, below)
            })),
        );
    }
    div().flex().flex_col().gap(SP_2).children(rows)
}

fn tile_view(theme: &Theme, width: Pixels, tile: Tile, below: Option<AnyElement>) -> impl IntoElement {
    let surface = theme.surface;
    let label = |words: String, color: Hsla| {
        div()
            .absolute()
            .bottom(px(2.))
            .px(SP_1)
            .rounded(RADIUS_SM)
            .bg(surface)
            .text_color(color)
            .text_size(FS_CAPTION)
            .line_height(gpui::relative(1.4))
            .font_features(tabular())
            .whitespace_nowrap()
            .child(text(words))
    };
    let picture = div()
        .relative()
        .flex()
        .justify_center()
        .child(
            icons::icon(&tile.icon, px(64.)).when(tile.sold, |i| i.grayscale(true)),
        )
        .when_some(tile.badge, |p, badge| p.child(label(badge, theme.muted).left(px(2.))))
        .when_some(tile.count, |p, count| {
            p.child(label(count, theme.on_surface).right(px(2.)))
        });
    div()
        .id(SharedString::from(format!("tile-{}", tile.key)))
        .role(Role::ListItem)
        .aria_label(tile.title.clone())
        .debug_selector({
            let key = tile.key.clone();
            move || format!("tile-{key}")
        })
        .w(width)
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .gap(SP_1)
        .p(SP_1)
        .rounded(RADIUS_SM)
        .bg(tile.face.unwrap_or(theme.wash_base))
        .when(tile.sold, |t| t.opacity(0.55))
        .tooltip(tooltip(tile.title))
        .child(picture)
        .children(below)
}

/// The colour a count held is written in against what one student takes: short of one red,
/// of two blue, enough for two green; none where nothing is needed.
pub fn tone_color(theme: &Theme, tone: Option<view::Tone>) -> Option<Hsla> {
    match tone? {
        view::Tone::Short => Some(theme.danger),
        view::Tone::Fair => Some(theme.accent),
        view::Tone::Enough => Some(theme.ok),
        _ => None,
    }
}

/// A tier's cell on its colour: the icon at its own shape on the left (its name on hover, or
/// the name and what each set holds for a sum of boxes), the count at the right, 10 px in from
/// each side; empty where the kind has no such tier.
pub fn tier_cell(theme: &Theme, id: &str, tier: usize, cell: Option<&HeldCell>, parts: Option<&str>) -> AnyElement {
    let base = div()
        .flex_1()
        .min_w_0()
        .py(SP_1)
        .px(px(10.))
        .bg(theme.tiers[tier.min(3)]);
    let Some(cell) = cell else {
        return base.into_any_element();
    };
    let title = parts.map_or_else(|| cell.name.clone(), |parts| format!("{}\n{parts}", cell.name));
    let tone = view::held_tone(cell.held, cell.need);
    let held = view::held_text(cell.held);
    base.id(SharedString::from(format!("{id}-{}", cell.id)))
        .role(Role::Cell)
        .aria_label(format!("{} {held}", cell.name))
        .tooltip(tooltip(title))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(SP_2)
                .child(icons::icon(&cell.icon, px(40.)).w(px(50.)).object_fit(gpui::ObjectFit::Contain))
                .child(
                    div()
                        .font_features(tabular())
                        .whitespace_nowrap()
                        .when_some(tone_color(theme, tone), |n, color| {
                            n.text_color(color).font_weight(FontWeight::SEMIBOLD)
                        })
                        .child(text(held)),
                ),
        )
        .into_any_element()
}

/// A row of a kind's four tiers.
pub fn tier_row(theme: &Theme, id: &str, cells: &[Option<HeldCell>], parts: Option<&[Option<String>]>) -> Div {
    div().flex().gap(px(0.)).children((0..4).map(|tier| {
        let cell = cells.get(tier).and_then(Option::as_ref);
        let part = parts.and_then(|p| p.get(tier)).and_then(Option::as_deref);
        tier_cell(theme, id, tier, cell, part)
    }))
}

/// How many students what is held raises, at a row's right end, and how in two lines under it:
/// the exp held, then over what one takes.
pub fn raise_cell(theme: &Theme, students: &str, numerator: &str, denominator: &str, color: Option<Hsla>) -> Div {
    div()
        .w(px(104.))
        .flex_none()
        .flex()
        .flex_col()
        .items_end()
        .px(SP_1)
        .font_features(tabular())
        .whitespace_nowrap()
        .child(
            div()
                .font_weight(FontWeight::BOLD)
                .when_some(color, |s, c| s.text_color(c))
                .child(text(students.to_owned())),
        )
        .child(
            div()
                .text_size(FS_CAPTION)
                .text_color(theme.muted)
                .child(text(numerator.to_owned())),
        )
        .child(
            div()
                .text_size(FS_CAPTION)
                .text_color(theme.muted)
                .child(text(denominator.to_owned())),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_fill_the_width_at_least_as_wide_as_asked() {
        let (count, width) = columns(px(696.), px(72.), px(8.));
        assert_eq!(count, 8, "eight 72 px columns and their gaps fit 696 px");
        assert!(width >= px(72.), "each at least as wide as asked");
        assert!(
            (f32::from(width) * 8. + 7. * 8. - 696.).abs() < 0.01,
            "the columns and gaps fill the width"
        );
    }

    #[test]
    fn a_narrow_width_still_has_one_column() {
        assert_eq!(columns(px(40.), px(72.), px(8.)).0, 1, "never none");
    }
}
