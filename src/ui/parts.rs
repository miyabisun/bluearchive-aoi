//! The pieces the item, equipment and work tabs draw their tables from: tiles of icons with
//! their counts, a kind's tiers on their colours, and how many students what is held raises.

use gpui::{
    AnyElement, Div, FontWeight, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, Role, SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _,
    div, prelude::FluentBuilder as _, px,
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
pub fn tiles(
    theme: &Theme,
    width: Pixels,
    tiles: Vec<Tile>,
    under: impl Fn(usize) -> Option<AnyElement>,
) -> Div {
    tiles_at(theme, width, px(64.), tiles, under)
}

/// Tiles in a grid of columns at least `min` wide, 8 px apart, filling `width`; `under` adds
/// what goes under a tile's icon (a good's offers).
pub fn tiles_at(
    theme: &Theme,
    width: Pixels,
    min: Pixels,
    tiles: Vec<Tile>,
    under: impl Fn(usize) -> Option<AnyElement>,
) -> Div {
    let (count, column) = columns(width, min, SP_2);
    let mut rows = Vec::new();
    let mut tiles = tiles.into_iter().enumerate().peekable();
    while tiles.peek().is_some() {
        let row: Vec<_> = tiles.by_ref().take(count).collect();
        rows.push(
            div()
                .flex()
                .gap(SP_2)
                .children(row.into_iter().map(|(i, tile)| {
                    let below = under(i);
                    tile_view(theme, column, &tile, below)
                })),
        );
    }
    div().flex().flex_col().gap(SP_2).children(rows)
}

/// Tiles in a grid of columns at least 64 px wide filling `width`, as a list that draws only the
/// rows in sight: an item list holds hundreds, and the icons of the rows never seen are never
/// loaded. It scrolls up and down by itself.
pub fn tiles_list(id: &str, width: Pixels, tiles: Vec<Tile>) -> gpui::UniformList {
    let (count, column) = columns(width, px(64.), SP_2);
    let tiles = std::rc::Rc::new(tiles);
    let rows = tiles.len().div_ceil(count);
    gpui::uniform_list(
        SharedString::from(id.to_owned()),
        rows,
        move |range, _window, cx| {
            let theme = Theme::get(cx);
            range
                .map(|row| {
                    let start = row * count;
                    let end = (start + count).min(tiles.len());
                    div().flex().gap(SP_2).pb(SP_2).children(
                        tiles[start..end]
                            .iter()
                            .map(|tile| tile_view(&theme, column, tile, None)),
                    )
                })
                .collect()
        },
    )
}

fn tile_view(
    theme: &Theme,
    width: Pixels,
    tile: &Tile,
    below: Option<AnyElement>,
) -> impl IntoElement + use<> {
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
        .child(icons::icon(&tile.icon, px(64.)).when(tile.sold, |i| i.grayscale(true)))
        .when_some(tile.badge.clone(), |p, badge| {
            p.child(label(badge, theme.muted).left(px(2.)))
        })
        .when_some(tile.count.clone(), |p, count| {
            p.child(label(count, theme.on_surface).right(px(2.)))
        });
    div()
        .id(SharedString::from(format!("tile-{}", tile.key)))
        .role(Role::ListItem)
        // What UI Automation reads: the name, then the count (as the tile shows it).
        .aria_label(match &tile.count {
            Some(count) => format!("{} {count}", tile.title.replace('\n', " ")),
            None => tile.title.replace('\n', " "),
        })
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
        .tooltip(tooltip(tile.title.clone()))
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

/// A blank in a row of tier cells, as wide as a cell: the layout keeps a box's padding before it
/// shares out the row, so a blank without it would widen the cells beside it.
pub fn blank_cell() -> Div {
    div().flex_1().min_w_0().py(SP_1).px(px(10.))
}

/// A tier's cell on its colour: the icon at its own shape on the left (its name on hover, or
/// the name and what each set holds for a sum of boxes), the count at the right, 10 px in from
/// each side; empty where the kind has no such tier.
pub fn tier_cell(
    theme: &Theme,
    id: &str,
    tier: usize,
    cell: Option<&HeldCell>,
    parts: Option<&str>,
) -> AnyElement {
    let base = blank_cell().bg(theme.tiers[tier.min(3)]);
    let Some(cell) = cell else {
        return base.into_any_element();
    };
    let title = parts.map_or_else(
        || cell.name.clone(),
        |parts| format!("{}\n{parts}", cell.name),
    );
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
                .child(
                    icons::icon(&cell.icon, px(40.))
                        .w(px(50.))
                        .object_fit(gpui::ObjectFit::Contain),
                )
                .child(held_count(theme, cell)),
        )
        .into_any_element()
}

/// A good's count held, in its colour against what one student takes.
fn held_count(theme: &Theme, cell: &HeldCell) -> Div {
    let tone = view::held_tone(cell.held, cell.need);
    div()
        .font_features(tabular())
        .whitespace_nowrap()
        .when_some(tone_color(theme, tone), |n, color| {
            n.text_color(color).font_weight(FontWeight::SEMIBOLD)
        })
        .child(text(view::held_text(cell.held)))
}

/// A tier's cell of counts alone, no icons, 2 px from top and bottom (the user's 2026-11-08
/// 「スキル素材アイテムの画像は不要」「paddingを2px程度まで削れる」), each at the right so the digits
/// line up (「アイテムは桁数が重要なので、基本的には右揃え」): one good's count, or two goods' in the
/// halves either side of a "/" in the middle (「"/"はセルのど真ん中に記述する」「セルを2つに割る
/// イメージで、それぞれを右揃え」), each in its own colour; every good's name and count on hover.
/// Empty where the kind has no such tier.
pub fn counts_cell(
    theme: &Theme,
    id: String,
    tier: usize,
    cells: &[Option<&HeldCell>],
) -> AnyElement {
    let base = blank_cell().py(px(2.)).bg(theme.tiers[tier.min(3)]);
    if cells.iter().all(Option::is_none) {
        return base.into_any_element();
    }
    let count = |cell: Option<&HeldCell>| cell.map(|cell| held_count(theme, cell));
    let counts = match *cells {
        [left, right] => div()
            .flex()
            .items_center()
            .gap(SP_1)
            .child(div().flex_1().flex().justify_end().children(count(left)))
            .child("/")
            .child(div().flex_1().flex().justify_end().children(count(right))),
        _ => div()
            .flex()
            .justify_end()
            .children(cells.iter().copied().filter_map(count)),
    };
    let lines = view::count_lines(cells);
    let selector = id.clone();
    base.id(SharedString::from(id))
        .role(Role::Cell)
        .aria_label(lines.replace('\n', " "))
        .tooltip(tooltip(lines))
        .debug_selector(move || format!("cell:{selector}"))
        .child(counts)
        .into_any_element()
}

/// A row of a kind's four tiers.
pub fn tier_row(
    theme: &Theme,
    id: &str,
    cells: &[Option<HeldCell>],
    parts: Option<&[Option<String>]>,
) -> Div {
    div().flex().gap(px(0.)).children((0..4).map(|tier| {
        let cell = cells.get(tier).and_then(Option::as_ref);
        let part = parts.and_then(|p| p.get(tier)).and_then(Option::as_deref);
        tier_cell(theme, id, tier, cell, part)
    }))
}

/// How many students what is held raises, at a row's right end, and how in two lines under it:
/// the exp held, then over what one takes.
pub fn raise_cell(
    theme: &Theme,
    students: &str,
    numerator: &str,
    denominator: &str,
    color: Option<Hsla>,
) -> Div {
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
                .when_some(color, gpui::Styled::text_color)
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
