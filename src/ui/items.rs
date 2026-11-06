//! The item tab: everything held as tiles, then tables by kind: the skill materials, the
//! ooparts, and the currencies (the user's of 2026-10-21).

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::{
    Aoi, icons,
    parts::{self, Tile, raise_cell, tier_row},
    theme::{FS_CAPTION, FS_SM, SP_1, SP_2, SP_3, SP_4, Theme},
    widgets::{text, tooltip},
};
use crate::view;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    All,
    Skills,
    Ooparts,
    Currency,
}

impl Sub {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::All, "全体"),
        (Self::Skills, "スキル素材"),
        (Self::Ooparts, "オーパーツ"),
        (Self::Currency, "通貨等"),
    ];
}

/// What the item tab tells before anything of its list is read.
pub const EMPTY: &str = "ブルーアーカイブのアイテム一覧を「デフォルト」の昇順で開いてスクロールすると、見えたアイテムの個数がここに並びます。";

impl Aoi {
    pub(super) fn items_tab(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.shown.items.is_empty() {
            return super::widgets::empty(theme, "items", EMPTY).into_any_element();
        }
        let sub = self.items_sub;
        let subtabs = super::subtabs(
            theme,
            "items",
            &Sub::ALL,
            sub,
            cx.listener(|this, sub: &Sub, _, cx| {
                this.items_sub = *sub;
                cx.notify();
            }),
        );
        let body = match sub {
            Sub::All => self.held_tiles(theme, window, cx),
            Sub::Skills => self.skills(theme).into_any_element(),
            Sub::Ooparts => self.ooparts(theme).into_any_element(),
            Sub::Currency => self.currency(theme, window).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(subtabs)
            .child(super::scroll("items-scroll", &self.content).child(body))
            .into_any_element()
    }

    /// Every item or piece of equipment held, by its icon, the count at its corner and the
    /// name on hover, narrowed by the search.
    pub(super) fn held_tiles(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items = self.tab == super::Tab::Items;
        let query = self.query.read(cx).text().to_owned();
        let rows = if items {
            &self.shown.items
        } else {
            &self.shown.equipment
        };
        let shown = view::filter_rows(rows, &query);
        let search = super::search(theme, &self.query, window, cx);
        if shown.is_empty() {
            return div()
                .flex()
                .flex_col()
                .child(search)
                .child(super::widgets::empty(
                    theme,
                    "no-match",
                    "一致する項目がありません",
                ))
                .into_any_element();
        }
        let tiles = shown
            .iter()
            .map(|row| Tile {
                key: row.id.to_string(),
                icon: row.icon.clone(),
                title: view::tile_title(row.id, &row.name, row.eleph, &self.shown.students),
                badge: row.tier.map(|t| format!("T{t}")),
                count: Some(view::format_count(row.count)),
                face: items
                    .then(|| super::face_color(theme, view::good_face(row.eleph, row.rarity)))
                    .flatten(),
                sold: false,
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .child(search)
            .child(parts::tiles(
                theme,
                parts::content_width(window),
                tiles,
                |_| None,
            ))
            .into_any_element()
    }

    /// The secret notes, the choice boxes, then each school's BDs over its tech notes, its
    /// emblem at the left, the three great schools first.
    fn skills(&self, theme: &Theme) -> impl IntoElement + use<> {
        let kinds = &self.shown.kinds;
        let emblem_column = |child: AnyElement| {
            div()
                .w(px(48.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .p(SP_1)
                .child(child)
        };
        let mut groups = Vec::new();
        if !kinds.secret.is_empty() {
            let mut cells: Vec<_> = kinds.secret.iter().cloned().map(Some).collect();
            cells.resize(4, None);
            // Both 最上級: drawn on the last tier's colour.
            groups.push(
                div()
                    .flex()
                    .child(emblem_column(div().into_any_element()))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .children(cells.iter().map(|cell| match cell {
                                Some(cell) => {
                                    parts::tier_cell(theme, "secret", 3, Some(cell), None)
                                }
                                None => div().flex_1().into_any_element(),
                            })),
                    )
                    .into_any_element(),
            );
        }
        if let Some(boxes) = view::box_totals(&kinds.skill_boxes) {
            let cells = |row: &[Option<view::BoxTotal>; 4]| -> (Vec<Option<crate::work::HeldCell>>, Vec<Option<String>>) {
                (
                    row.iter().map(|b| b.as_ref().map(|b| b.cell.clone())).collect(),
                    row.iter().map(|b| b.as_ref().map(|b| b.parts.clone())).collect(),
                )
            };
            let (bd, bd_parts) = cells(&boxes.bd);
            let (notes, notes_parts) = cells(&boxes.notes);
            groups.push(
                div()
                    .flex()
                    .child(emblem_column(
                        div()
                            .id("set")
                            .text_size(FS_CAPTION)
                            .whitespace_nowrap()
                            .tooltip(tooltip("選択ボックスの合計"))
                            .child(text("選択"))
                            .into_any_element(),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(tier_row(theme, "box-bd", &bd, Some(&bd_parts)))
                            .child(tier_row(theme, "box-notes", &notes, Some(&notes_parts))),
                    )
                    .into_any_element(),
            );
        }
        for school in &kinds.schools {
            groups.push(
                div()
                    .flex()
                    .child(emblem_column(
                        div()
                            .id(SharedString::from(format!("emblem-{}", school.icon)))
                            .tooltip(tooltip(school.name.clone()))
                            .child(icons::emblem(&school.icon, theme.emblem, px(36.)).w(px(40.)))
                            .into_any_element(),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(tier_row(
                                theme,
                                &format!("bd-{}", school.icon),
                                &school.bd,
                                None,
                            ))
                            .child(tier_row(
                                theme,
                                &format!("notes-{}", school.icon),
                                &school.notes,
                                None,
                            )),
                    )
                    .into_any_element(),
            );
        }
        div().flex().flex_col().gap(SP_2).children(groups)
    }

    /// An oopart a row, its tiers on their colours; their choice boxes first (the user's of
    /// 2026-10-29), apart from the ooparts under them.
    fn ooparts(&self, theme: &Theme) -> impl IntoElement + use<> {
        let kinds = &self.shown.kinds;
        let boxes = kinds.oopart_boxes.iter().any(Option::is_some);
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .when(boxes, |t| {
                t.child(tier_row(theme, "oopart-boxes", &kinds.oopart_boxes, None).mb(SP_2))
            })
            .children(
                kinds
                    .ooparts
                    .iter()
                    .enumerate()
                    .map(|(k, tiers)| tier_row(theme, &format!("oopart-{k}"), tiers, None)),
            )
    }

    /// The credits and the reports in students raised, as the commissions count them, then the
    /// coins held (the user's of 2026-10-29: 「通貨等」).
    fn currency(&self, theme: &Theme, window: &Window) -> impl IntoElement + use<> {
        let rows = view::currency_rows(&self.shown.work.commissions);
        let coins = view::currency_coins(&self.shown.kinds.coins, &self.shown.shops);
        let tiles = coins
            .iter()
            .map(|coin| Tile {
                key: format!("coin-{}", coin.name),
                icon: coin.icon.clone(),
                title: coin.name.clone(),
                badge: None,
                count: Some(view::held_text(coin.held)),
                face: None,
                sold: false,
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .child(raise_rows(theme, "commissions", &rows, false))
            .child(
                div()
                    .mt(SP_4)
                    .mb(SP_2)
                    .text_size(FS_SM)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted)
                    .child(text("コイン")),
            )
            .child(parts::tiles(
                theme,
                parts::content_width(window),
                tiles,
                |_| None,
            ))
    }
}

/// Rows of students raised (the work tab's and the item tab's 通貨等, drawn as the enhancement
/// materials' rows): a title over the tiers or the credits held, the students it raises at the
/// right; the one raising more green, fewer orange (red where `fewer_red`), the school to play
/// first red.
pub fn raise_rows(
    theme: &Theme,
    kind: &str,
    rows: &[view::RaiseRow],
    fewer_red: bool,
) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .children(rows.iter().enumerate().map(|(i, row)| {
            let color = match row.tone {
                Some(view::Tone::More) => Some(theme.ok),
                Some(view::Tone::Fewer) if fewer_red => Some(theme.danger),
                Some(view::Tone::Fewer) => Some(theme.warn),
                Some(view::Tone::First) => Some(theme.danger),
                _ => None,
            };
            let title_color = if row.tone == Some(view::Tone::First) {
                theme.danger
            } else {
                theme.muted
            };
            let held = match &row.held {
                view::RaiseHeld::Tiers(tiers) => {
                    tier_row(theme, &format!("{kind}-{}", row.key), tiers, None)
                        .flex_1()
                        .into_any_element()
                }
                view::RaiseHeld::Credits(credits) => div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(SP_2)
                    .py(SP_1)
                    .px(px(10.))
                    .bg(theme.tiers[0])
                    .child(icons::icon("currency_icon_gold", px(40.)).w(px(50.)))
                    .child(text(credits.clone()))
                    .into_any_element(),
            };
            div()
                .id(SharedString::from(format!("{kind}-{}", row.key)))
                .role(Role::Row)
                .aria_label(row.title.clone())
                .when_some(row.why.clone().filter(|w| !w.is_empty()), |r, why| {
                    r.tooltip(tooltip(why))
                })
                .flex()
                .flex_col()
                .child(
                    div()
                        .pt(if i == 0 { SP_1 } else { SP_3 })
                        .pb(SP_1)
                        .text_size(FS_SM)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(title_color)
                        .child(text(row.title.clone())),
                )
                .child(div().flex().items_center().child(held).child(raise_cell(
                    theme,
                    &row.students,
                    &row.numerator,
                    &row.denominator,
                    color,
                )))
        }))
}
