//! The equipment tab: every piece held as tiles, then the blueprints slot by slot and the
//! enhancement materials kind by kind.

use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use super::{
    Aoi, icons,
    parts::{raise_cell, tier_row},
    theme::{FS_CAPTION, FS_SM, SP_1, SP_2, Theme, tabular},
    widgets::{text, tooltip},
};
use crate::{view, work::HeldCell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    All,
    Blueprints,
    Enhancements,
}

impl Sub {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::All, "全体"),
        (Self::Blueprints, "設計図"),
        (Self::Enhancements, "強化素材"),
    ];
}

/// What the equipment tab tells before anything of its list is read.
pub const EMPTY: &str = "ブルーアーカイブの装備一覧を「デフォルト」の昇順で開いてスクロールすると、見えた装備の個数がここに並びます。";

/// The tiers a slot's blueprints are counted at: its T1 piece's, then T2 to T10.
const PRINT_TIERS: std::ops::RangeInclusive<u8> = 1..=10;

impl Aoi {
    pub(super) fn equipment_tab(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.shown.equipment.is_empty() {
            return super::widgets::empty(theme, "equipment", EMPTY).into_any_element();
        }
        let sub = self.equipment_sub;
        let subtabs = super::subtabs(
            theme,
            "equipment",
            &Sub::ALL,
            sub,
            cx.listener(|this, sub: &Sub, _, cx| {
                this.equipment_sub = *sub;
                cx.notify();
            }),
        );
        let body = match sub {
            Sub::All => self.held_tiles(theme, window, cx),
            Sub::Blueprints => self.blueprints(theme).into_any_element(),
            Sub::Enhancements => self.enhancements(theme).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(subtabs)
            .child(super::scroll("equipment-scroll", &self.content).child(body))
            .into_any_element()
    }

    /// A slot a row: its T1 pieces, its blueprints of T2 to T10 and its universal ones held,
    /// each named on hover; the blueprints' choice boxes at the bottom, each under the tier it
    /// gives.
    fn blueprints(&self, theme: &Theme) -> impl IntoElement + use<> {
        let kinds = &self.shown.kinds;
        let count_cell = |id: String, cell: Option<&HeldCell>| {
            let base = div()
                .w(gpui::relative(0.07))
                .flex_none()
                .flex()
                .justify_end()
                .px(SP_1)
                .text_size(FS_SM)
                .font_features(tabular())
                .whitespace_nowrap();
            match cell {
                Some(cell) => {
                    let held = view::held_text(cell.held);
                    base.id(SharedString::from(id))
                        .role(Role::Cell)
                        .aria_label(format!("{} {held}", cell.name))
                        .tooltip(tooltip(cell.name.clone()))
                        .child(text(held))
                        .into_any_element()
                }
                None => base.into_any_element(),
            }
        };
        let rule = theme.border;
        let row = || {
            div()
                .flex()
                .items_center()
                .py(SP_1)
                .border_b_1()
                .border_color(rule)
        };
        let head = row()
            .text_size(FS_CAPTION)
            .text_color(theme.muted)
            .child(div().flex_1().px(SP_1).child(text("部位")))
            .children(PRINT_TIERS.map(|tier| {
                div()
                    .w(gpui::relative(0.07))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .px(SP_1)
                    .child(text(format!("T{tier}")))
            }))
            .child(
                div()
                    .w(gpui::relative(0.07))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .px(SP_1)
                    .child(text("万能")),
            );
        let slots = kinds.blueprints.iter().map(|slot| {
            row()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(SP_2)
                        .px(SP_1)
                        .whitespace_nowrap()
                        .child(icons::icon(&slot.icon, px(32.)))
                        .child(text(slot.name.clone())),
                )
                .children(
                    slot.tiers
                        .iter()
                        .chain(std::iter::once(&slot.universal))
                        .enumerate()
                        .map(|(i, cell)| {
                            count_cell(format!("print-{}-{i}", slot.name), cell.as_ref())
                        }),
                )
        });
        let boxes = kinds.print_boxes.iter().any(Option::is_some).then(|| {
            row()
                .child(
                    div()
                        .flex_1()
                        .px(SP_1)
                        .whitespace_nowrap()
                        .child(text("選択ボックス")),
                )
                .children(
                    kinds
                        .print_boxes
                        .iter()
                        .chain(std::iter::once(&None))
                        .enumerate()
                        .map(|(i, cell)| count_cell(format!("print-box-{i}"), cell.as_ref())),
                )
        });
        div()
            .flex()
            .flex_col()
            .child(head)
            .children(slots)
            .children(boxes)
    }

    /// A kind of enhancement material a row: its tiers on their colours, then how many students
    /// what is held raises, and how, as the work tab counts it.
    fn enhancements(&self, theme: &Theme) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .children(self.shown.kinds.enhancements.iter().map(|row| {
                let cells = view::enhancement_cells(row);
                div()
                    .flex()
                    .items_center()
                    .child(
                        tier_row(
                            theme,
                            &format!("enhancement-{}", row.category),
                            &row.tiers,
                            None,
                        )
                        .flex_1(),
                    )
                    .child(raise_cell(
                        theme,
                        &cells.students,
                        &cells.numerator,
                        &cells.denominator,
                        None,
                    ))
            }))
    }
}
