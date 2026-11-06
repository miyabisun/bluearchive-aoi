//! The shop tab: each shop read, closed until opened, its coins held on its title and its goods
//! as tiles, what each coin buys under each.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::{
    Aoi, icons,
    parts::{self, Tile},
    theme::{FS_CAPTION, FS_MD, FS_SM, SP_1, SP_2, SP_3, SP_4, Theme, tabular},
    widgets::{fold_title, text, tooltip},
};
use crate::view;

/// What the shop tab tells before any shop is read.
pub const EMPTY: &str = "ブルーアーカイブのショップで総力戦・大決戦・カケラ(文字)・合同火力演習・指名手配・熟達証書を開いて下まで送ると、品ごとにあと何回買えるかがここに並びます。";

impl Aoi {
    pub(super) fn shops_tab(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.shown.shops.is_empty() {
            return super::widgets::empty(theme, "shops", EMPTY).into_any_element();
        }
        let query = self.query.read(cx).text().to_owned();
        let searching = !query.trim().is_empty();
        let shops = view::filter_shops(&self.shown.shops, &query);
        let search = super::search(theme, &self.query);
        if shops.is_empty() {
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
        let width = parts::content_width(window);
        let folds: Vec<AnyElement> = shops
            .into_iter()
            .map(|shop| {
                // Closed until opened, and open while a search narrows the goods.
                let open = searching || self.opened.contains(&shop.name);
                let key = shop.name.clone();
                let read_at = view::short_time(&shop.read_at);
                let held = div()
                    .ml_auto()
                    .flex()
                    .gap(SP_3)
                    .text_size(FS_SM)
                    .font_features(tabular())
                    .children(shop.coins.iter().map(|coin| {
                        div()
                            .id(SharedString::from(format!(
                                "coin-{}-{}",
                                shop.name, coin.name
                            )))
                            .tooltip(tooltip(coin.name.clone()))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(icons::icon(&coin.icon, px(20.)))
                            .child(text(
                                coin.held.map_or_else(|| "?".to_owned(), view::format_count),
                            ))
                    }));
                let title = fold_title(theme, &format!("shop-{}", shop.name), open)
                    .aria_label(shop.name.clone())
                    .flex_wrap()
                    .gap(SP_3)
                    .py(SP_2)
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(SP_2)
                            .text_size(FS_MD)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(text(shop.name.clone()))
                            .child(
                                div()
                                    .text_size(FS_SM)
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(theme.muted)
                                    .font_features(tabular())
                                    .child(text(read_at)),
                            )
                            .when(
                                view::shop_stale(shop.resets, &shop.read_at, &crate::keep::now()),
                                |h| h.child(div().text_size(FS_SM).child(text("前の月の値"))),
                            ),
                    )
                    .child(held)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        // While a search narrows the goods, every shop stays open.
                        if !searching {
                            this.toggle_fold(&key, cx);
                        }
                    }));
                let goods = open.then(|| {
                    let tiles: Vec<Tile> = shop
                        .goods
                        .iter()
                        .map(|good| {
                            let tile = view::good_tile(good);
                            Tile {
                                key: format!("{}-{}", shop.name, good.id),
                                icon: good.icon.clone(),
                                title: good.name.clone(),
                                badge: None,
                                count: (!tile.amount.is_empty()).then(|| tile.amount.clone()),
                                face: super::face_color(
                                    theme,
                                    view::good_face(good.eleph, good.rarity),
                                ),
                                sold: good.sold,
                            }
                        })
                        .collect();
                    let offers: Vec<Vec<Option<view::OfferText>>> = shop
                        .goods
                        .iter()
                        .map(|good| view::good_tile(good).offers)
                        .collect();
                    let coins = shop.coins.clone();
                    let muted = theme.muted;
                    let (count, column) = parts::columns(width, px(112.), SP_2);
                    let _ = count;
                    div()
                        .mt(SP_1)
                        .child(parts::tiles_at(theme, width, px(112.), tiles, move |i| {
                            let offers = offers.get(i)?;
                            Some(
                                div()
                                    .w(column)
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .children(offers.iter().enumerate().map(|(c, offer)| {
                                        div()
                                            .min_h(px(18.))
                                            .flex()
                                            .justify_center()
                                            .items_center()
                                            .gap(px(2.))
                                            .text_size(FS_CAPTION)
                                            .font_features(tabular())
                                            .whitespace_nowrap()
                                            .when_some(offer.as_ref(), |line, offer| {
                                                let coin = &coins[c];
                                                line.when(offer.sold, |l| l.text_color(muted))
                                                    .child(icons::icon(&coin.icon, px(18.)))
                                                    .child(text(offer.text.clone()))
                                            })
                                    }))
                                    .into_any_element(),
                            )
                        }))
                });
                div()
                    .id(SharedString::from(format!("shop-{}", shop.name)))
                    .role(Role::Group)
                    .aria_label(shop.name.clone())
                    .border_b_1()
                    .border_color(theme.border)
                    .when(open, |f| f.pb(SP_4))
                    .child(title)
                    .children(goods)
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(search)
            .child(super::scroll("shops-scroll", &self.content).children(folds))
            .into_any_element()
    }
}
