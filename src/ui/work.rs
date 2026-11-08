//! The work tab: the plays by when they are played (each day, in a doubled-drop campaign, at
//! the month's end), a fold each, against what is held.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Window,
    div, prelude::FluentBuilder as _, px,
};

use super::{
    Aoi, icons,
    items::raise_rows,
    parts,
    students::stars,
    theme::{FS_CAPTION, FS_MD, FS_SM, RADIUS_SM, SP_1, SP_2, SP_3, SP_4, SP_5, Theme, tabular},
    widgets::{fold_title, text, tooltip},
};
use crate::{strategy::Reserve, view};

impl Aoi {
    /// A fold onto `folds`: its title (pressed, it opens and closes), and what it holds while
    /// open; brought to the top of the tab's frame if it is the one to bring up (`bring_up`).
    fn fold(
        &mut self,
        folds: &mut Vec<AnyElement>,
        theme: &Theme,
        key: &str,
        head: Head,
        body: impl FnOnce(&Self) -> Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) {
        let Head {
            label,
            title,
            extra,
        } = head;
        let open = self.opened.contains(key);
        if self.bring_up.as_deref() == Some(key) {
            self.content.scroll_to_top_of_item(folds.len());
        }
        let owned = key.to_owned();
        let fold = div()
            .id(SharedString::from(format!("fold-box-{key}")))
            .role(Role::Group)
            .debug_selector({
                let key = key.to_owned();
                move || format!("fold:{key}")
            })
            .border_b_1()
            .border_color(theme.border)
            .when(open, |f| f.pb(SP_4))
            .child(
                fold_title(theme, key, open)
                    .aria_label(label)
                    .gap(SP_2)
                    .py(SP_2)
                    .child(title)
                    .children(extra)
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_fold(&owned, cx))),
            )
            .when(open, |f| f.children(body(self)))
            .into_any_element();
        folds.push(fold);
    }

    pub(super) fn work_tab(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let width = parts::content_width(window);
        let heading = |title: &str, first: bool| {
            div()
                .when(!first, |h| h.mt(SP_5))
                .text_size(FS_SM)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted)
                .child(text(title.to_owned()))
                .into_any_element()
        };
        let h3 = |title: &str| {
            div()
                .text_size(FS_MD)
                .font_weight(FontWeight::SEMIBOLD)
                .child(text(title.to_owned()))
                .into_any_element()
        };
        let h4 = |theme: &Theme, title: String| {
            div()
                .mt(SP_3)
                .mb(SP_1)
                .text_size(FS_SM)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted)
                .child(text(title))
                .into_any_element()
        };
        let orbs = self
            .shown
            .kinds
            .enhancements
            .iter()
            .find(|row| row.category == "Exp")
            .cloned();
        let mut folds: Vec<AnyElement> = vec![heading("日課", true)];
        let t = *theme;
        self.fold(
            &mut folds,
            theme,
            "work:通常アイテム",
            Head {
                label: "ショップ: 通常アイテム".to_owned(),
                title: h3("ショップ: 通常アイテム"),
                extra: None,
            },
            |this| {
                vec![
                    raise_rows(
                        &t,
                        "raises",
                        &view::normal_item_rows(&this.shown.work.commissions, orbs.as_ref()),
                        true,
                    )
                    .into_any_element(),
                ]
            },
            cx,
        );
        self.fold(
            &mut folds,
            theme,
            "work:指名手配",
            Head {
                label: "指名手配".to_owned(),
                title: h3("指名手配"),
                extra: None,
            },
            |this| {
                this.shown
                    .work
                    .bounties
                    .iter()
                    .flat_map(|place| {
                        [
                            h4(&t, view::place_name(&place.name).to_owned()),
                            bounty_table(&t, place),
                        ]
                    })
                    .collect()
            },
            cx,
        );
        self.fold(
            &mut folds,
            theme,
            "work:学園交流会",
            Head {
                label: "学園交流会".to_owned(),
                title: h3("学園交流会"),
                extra: None,
            },
            |this| {
                vec![
                    raise_rows(
                        &t,
                        "schools",
                        &view::scrimmage_rows(
                            &this.shown.work.scrimmages,
                            &this.shown.work.schools,
                        ),
                        false,
                    )
                    .into_any_element(),
                ]
            },
            cx,
        );
        folds.push(heading("2倍キャンペーン", false));
        self.fold(
            &mut folds,
            theme,
            "work:任務",
            Head {
                label: "任務 Normal".to_owned(),
                title: h3("任務 Normal"),
                extra: None,
            },
            |this| this.missions(&t),
            cx,
        );
        let hard_ap = view::hard_cost(&self.shown.hard);
        let hard_extra = (hard_ap > 0).then(|| {
            div()
                .id("hard-ap")
                .ml_auto()
                .flex()
                .items_center()
                .gap(px(2.))
                .text_size(FS_SM)
                .font_features(tabular())
                .tooltip(tooltip(format!(
                    "任務 Hard の予約: 1日 {} AP",
                    view::exact(hard_ap)
                )))
                .child(icons::icon("currency_icon_ap", px(20.)))
                .child(
                    div()
                        .text_color(theme.danger)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(text(format!("-{}", view::exact(hard_ap)))),
                )
                .into_any_element()
        });
        let hard_cards = self.hard_cards(theme, width, cx);
        self.fold(
            &mut folds,
            theme,
            "work:任務 Hard",
            Head {
                label: "任務 Hard".to_owned(),
                title: h3("任務 Hard"),
                extra: hard_extra,
            },
            move |_| vec![hard_cards],
            cx,
        );
        self.fold(
            &mut folds,
            theme,
            "work:特別依頼",
            Head {
                label: "特別依頼".to_owned(),
                title: h3("特別依頼"),
                extra: None,
            },
            |this| {
                vec![
                    raise_rows(
                        &t,
                        "commissions",
                        &view::commission_rows(&this.shown.work.commissions),
                        false,
                    )
                    .into_any_element(),
                ]
            },
            cx,
        );
        folds.push(heading("月末", false));
        let shops = self.shown.month_end.clone();
        for shop in &shops {
            let key = format!("work:月末:{}", shop.name);
            let title = div()
                .flex()
                .items_baseline()
                .gap(SP_2)
                .text_size(FS_MD)
                .font_weight(FontWeight::SEMIBOLD)
                .child(text(format!("ショップ: {}", shop.name)))
                .when_some(shop.read_at.clone(), |h, read_at| {
                    h.child(
                        div()
                            .text_size(FS_SM)
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme.muted)
                            .font_features(tabular())
                            .child(text(view::short_time(&read_at))),
                    )
                    .when(
                        view::shop_stale(true, &read_at, &crate::keep::now()),
                        |h| {
                            h.child(
                                div()
                                    .text_size(FS_SM)
                                    .font_weight(FontWeight::NORMAL)
                                    .child(text("前の月の値")),
                            )
                        },
                    )
                })
                .into_any_element();
            // The coins left once the cards reserved are paid: blue while any is reserved, so
            // it is not taken for the game's count; short red.
            let coins = div()
                .ml_auto()
                .flex()
                .gap(SP_3)
                .text_size(FS_SM)
                .font_features(tabular())
                .children(shop.coins.iter().map(|coin| {
                    let left = view::plan_coin(coin);
                    let color = if left.short {
                        Some(theme.danger)
                    } else if left.reserved {
                        Some(theme.accent)
                    } else {
                        None
                    };
                    div()
                        .id(SharedString::from(format!(
                            "plan-coin-{}-{}",
                            shop.name, coin.name
                        )))
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .tooltip(tooltip(left.title.clone()))
                        .child(icons::icon(&coin.icon, px(20.)))
                        .child(
                            div()
                                .when_some(color, gpui::Styled::text_color)
                                .when(left.short, |d| d.font_weight(FontWeight::SEMIBOLD))
                                .child(text(left.left.clone())),
                        )
                }))
                .into_any_element();
            let cards = self.month_end_cards(theme, width, shop, cx);
            let label = format!("ショップ: {}", shop.name);
            self.fold(
                &mut folds,
                theme,
                &key,
                Head {
                    label,
                    title,
                    extra: Some(coins),
                },
                move |_| vec![cards],
                cx,
            );
        }
        self.bring_up = None;
        super::scroll("work-scroll", &self.content)
            .children(folds)
            .into_any_element()
    }

    /// 任務 Normal: a slot's T10 blueprints and apart its universal ones with those of T9 and
    /// below, then how many times to play each mission in a campaign to stock the golden ratio.
    fn missions(&self, theme: &Theme) -> Vec<AnyElement> {
        let work = &self.shown.work;
        let rule = theme.border;
        let mut parts: Vec<AnyElement> = Vec::new();
        let header = div()
            .flex()
            .py(SP_1)
            .border_b_1()
            .border_color(rule)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(div().flex_1().child(text("部位")))
            .child(div().flex_1().child(text("T10設計図")))
            .child(div().flex_1().child(text("万能設計図")));
        parts.push(header.into_any_element());
        for slot in &work.slots {
            let cells = |stock| {
                let c = view::stock_cells(stock, slot.golden.students);
                let color = match c.tone {
                    Some(view::Tone::Short) => Some(theme.danger),
                    Some(view::Tone::Low) => Some(theme.warn),
                    _ => None,
                };
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font_weight(if color.is_some() {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::BOLD
                            })
                            .when_some(color, gpui::Styled::text_color)
                            .font_features(tabular())
                            .child(text(c.pieces)),
                    )
                    .child(
                        div()
                            .text_size(FS_CAPTION)
                            .text_color(theme.muted)
                            .font_features(tabular())
                            .child(text(c.held)),
                    )
            };
            parts.push(
                div()
                    .flex()
                    .items_center()
                    .py(SP_1)
                    .border_b_1()
                    .border_color(rule)
                    .child(
                        div()
                            .id(SharedString::from(format!("slot-{}", slot.name)))
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(SP_2)
                            .whitespace_nowrap()
                            .tooltip(tooltip(view::golden_title(&slot.golden)))
                            .child(icons::icon(&slot.icon, px(32.)))
                            .child(text(slot.name.clone())),
                    )
                    .child(cells(&slot.top))
                    .child(cells(&slot.universal))
                    .into_any_element(),
            );
        }
        if let Some(farm) = &work.farm {
            let farm = view::farm_cells(farm);
            let h4 = |title: &str| {
                div()
                    .mt(SP_3)
                    .mb(SP_1)
                    .text_size(FS_SM)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted)
                    .child(text(title.to_owned()))
                    .into_any_element()
            };
            parts.push(h4("黄金比までの周回（2倍キャンペーン中）"));
            if farm.lines.is_empty() {
                parts.push(
                    div()
                        .text_size(FS_CAPTION)
                        .text_color(theme.muted)
                        .child(text("どの部位も黄金比に届いています"))
                        .into_any_element(),
                );
            } else {
                let row =
                    |name: AnyElement, runs: String, ap: String, slots: String, total: bool| {
                        div()
                            .flex()
                            .items_center()
                            .px(SP_2)
                            .py(SP_1)
                            .when(!total, |r| r.border_b_1().border_color(rule))
                            .when(total, |r| {
                                r.border_t_2()
                                    .border_color(rule)
                                    .font_weight(FontWeight::SEMIBOLD)
                            })
                            .child(div().flex_1().child(name))
                            .child(
                                div()
                                    .w(px(64.))
                                    .flex()
                                    .justify_end()
                                    .font_features(tabular())
                                    .child(text(runs)),
                            )
                            .child(
                                div()
                                    .w(px(80.))
                                    .pl(SP_3)
                                    .flex()
                                    .justify_end()
                                    .font_features(tabular())
                                    .child(text(ap)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .pl(SP_2)
                                    .text_size(FS_CAPTION)
                                    .text_color(theme.muted)
                                    .child(text(slots)),
                            )
                            .into_any_element()
                    };
                for line in &farm.lines {
                    parts.push(row(
                        text(line.name.clone()).into_any_element(),
                        line.runs.clone(),
                        line.ap.clone(),
                        line.slots.clone(),
                        false,
                    ));
                }
                parts.push(row(
                    text("計").into_any_element(),
                    farm.total.runs.clone(),
                    farm.total.ap.clone(),
                    String::new(),
                    true,
                ));
            }
            if !farm.next.is_empty() {
                parts.push(h4("その後に黄金比を整える周回"));
                for line in &farm.next {
                    parts.push(
                        div()
                            .flex()
                            .px(SP_2)
                            .py(SP_1)
                            .border_b_1()
                            .border_color(rule)
                            .child(div().flex_1().child(text(line.name.clone())))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(FS_CAPTION)
                                    .text_color(theme.muted)
                                    .child(text(line.slots.clone())),
                            )
                            .into_any_element(),
                    );
                }
            }
        }
        parts
    }

    /// 任務 Hard: a student's eleph a card, as the month-end shops show a good: how many missions
    /// drop it, the student's star and the elephs held; pressing one reserves it, again takes it
    /// back. Those at 固有4 greyed, last, yet reserved as the others.
    fn hard_cards(
        &mut self,
        theme: &Theme,
        width: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cards: Vec<Card> = self
            .shown
            .hard
            .iter()
            .map(|card| {
                let tile = view::hard_tile(card);
                Card {
                    key: format!("hard-{}", card.id),
                    icon: card.icon.clone(),
                    title: tile.title.clone(),
                    amount: Some(tile.amount.clone()),
                    face: Some(theme.eleph),
                    sold: card.done,
                    reserved: card.reserved,
                    coin_icon: None,
                    star: tile.star,
                    text: tile.held.clone(),
                    reserve: Reserve {
                        shop: "任務 Hard".to_owned(),
                        id: card.id,
                        coin: "AP".to_owned(),
                    },
                }
            })
            .collect();
        Self::cards(theme, width, cards, cx)
    }

    /// A month-end shop's cards: a good and a coin, as the game shows them; pressing one
    /// reserves it, again takes it back.
    fn month_end_cards(
        &mut self,
        theme: &Theme,
        width: gpui::Pixels,
        shop: &crate::strategy::PlanShop,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if shop.read_at.is_none() {
            return div()
                .text_size(FS_CAPTION)
                .text_color(theme.muted)
                .child(text("まだ読んでいません"))
                .into_any_element();
        }
        let cards: Vec<Card> = shop
            .cards
            .iter()
            .map(|card| {
                let coin = &shop.coins[card.coin];
                let tile = view::plan_card(card);
                Card {
                    key: format!("plan-{}-{}-{}", shop.name, card.id, card.coin),
                    icon: card.icon.clone(),
                    title: view::plan_title(card, &self.shown.students),
                    amount: Some(tile.amount.clone()),
                    face: super::face_color(theme, view::good_face(card.eleph, card.rarity)),
                    sold: card.left == 0 || card.barred,
                    reserved: card.reserved,
                    coin_icon: Some(coin.icon.clone()),
                    star: tile.star,
                    text: tile.text.clone(),
                    reserve: Reserve {
                        shop: shop.name.clone(),
                        id: card.id,
                        coin: coin.name.clone(),
                    },
                }
            })
            .collect();
        Self::cards(theme, width, cards, cx)
    }

    fn cards(
        theme: &Theme,
        width: gpui::Pixels,
        cards: Vec<Card>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (count, column) = parts::columns(width, px(112.), SP_2);
        let mut rows = Vec::new();
        let mut cards = cards.into_iter().peekable();
        while cards.peek().is_some() {
            let row: Vec<Card> = cards.by_ref().take(count).collect();
            rows.push(div().flex().gap(SP_2).children(row.into_iter().map(|card| {
                let reserve = card.reserve.clone();
                let reserved = card.reserved;
                div()
                    .id(SharedString::from(card.key.clone()))
                    .role(Role::Button)
                    .aria_label(card.title.clone())
                    .aria_toggled(if reserved {
                        gpui::Toggled::True
                    } else {
                        gpui::Toggled::False
                    })
                    .debug_selector({
                        let key = card.key.clone();
                        move || format!("card:{key}")
                    })
                    .tab_stop(true)
                    .relative()
                    .w(column)
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(SP_1)
                    .p(SP_1)
                    .border_2()
                    .border_color(if reserved {
                        theme.accent
                    } else {
                        gpui::transparent_black()
                    })
                    .rounded(RADIUS_SM)
                    .bg(card.face.unwrap_or(theme.wash_base))
                    .when(card.sold, |c| c.opacity(0.55))
                    .cursor_pointer()
                    .tooltip(tooltip(card.title.clone()))
                    .child(
                        div()
                            .relative()
                            .child(
                                icons::icon(&card.icon, px(64.))
                                    .when(card.sold, |i| i.grayscale(true)),
                            )
                            .when_some(card.amount.filter(|a| !a.is_empty()), |p, amount| {
                                p.child(
                                    div()
                                        .absolute()
                                        .right(px(2.))
                                        .bottom(px(2.))
                                        .px(SP_1)
                                        .rounded(RADIUS_SM)
                                        .bg(theme.surface)
                                        .text_size(FS_CAPTION)
                                        .font_features(tabular())
                                        .child(text(amount)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .min_h(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(2.))
                            .text_size(FS_CAPTION)
                            .font_features(tabular())
                            .whitespace_nowrap()
                            .when_some(card.coin_icon, |o, icon| {
                                o.child(icons::icon(&icon, px(18.)))
                            })
                            .when_some(card.star, |o, star| {
                                o.child(stars(&card.key, star, px(12.)))
                            })
                            .child(text(card.text)),
                    )
                    .when(reserved, |c| {
                        c.child(
                            div()
                                .absolute()
                                .top(px(-2.))
                                .right(px(-2.))
                                .px(SP_1)
                                .rounded_tr(RADIUS_SM)
                                .rounded_bl(RADIUS_SM)
                                .bg(theme.accent)
                                .text_color(theme.surface)
                                .text_size(FS_CAPTION)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(text("予約中")),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let _ = this.core.set_reserved(reserve.clone(), !reserved);
                        cx.notify();
                    }))
            })));
        }
        div()
            .mt(SP_1)
            .flex()
            .flex_col()
            .gap(SP_2)
            .children(rows)
            .into_any_element()
    }
}

/// A bounty's table (the user's 2026-11-08 「スキル素材(ノート・BD)は1行にまとめる」): a row for each
/// oopart, its top tier's picture at the left (「一番左の学園エンブレムの代わりとして、オーパーツの
/// 最上級の画像を表示させる」), then a row for each school, its emblem at the left and its BDs and
/// tech notes over one another in each tier; the pictures no taller than a row of text, outside
/// the four tiers (「学園のエンブレムが4分割の表の外に出るので、1行の高さをギリギリまで削る」).
fn bounty_table(theme: &Theme, place: &crate::work::BountyRow) -> AnyElement {
    let id = format!("bounty-{}", place.name);
    let ooparts = place
        .ooparts
        .iter()
        .enumerate()
        .map(|(k, tiers)| parts::oopart_row(theme, &format!("{id}-oopart{k}"), tiers));
    let schools = place
        .schools
        .iter()
        .map(|school| parts::school_row(theme, &id, school));
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .children(ooparts)
        .children(schools)
        .into_any_element()
}

/// A fold's title: its name for UI Automation, what it shows, and what goes at its right end.
struct Head {
    label: String,
    title: AnyElement,
    extra: Option<AnyElement>,
}

/// A card of the hard missions or a month-end shop, as the window draws it.
struct Card {
    key: String,
    icon: String,
    title: String,
    amount: Option<String>,
    face: Option<gpui::Hsla>,
    sold: bool,
    reserved: bool,
    coin_icon: Option<String>,
    star: Option<u8>,
    text: String,
    reserve: Reserve,
}
