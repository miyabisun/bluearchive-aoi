//! The settings page, in the tabs' place while its button is on: the update, where the game is
//! read from (with the link to a phone or the wait for a streaming app under the one chosen), the
//! theme, the work tab's following, the cursor (on Windows), and what was read saved to a file or
//! restored from one.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
};

use super::{
    Aoi,
    theme::{FS_SM, SP_1, SP_2, SP_3, SP_4, Theme},
    widgets::{self, choice, outlined_button, text},
};
use crate::{
    android::Link,
    broadcast::Cast,
    settings::{CursorChoice, Feed, ThemeChoice, WORK_FOLLOW},
};

/// A group's choices side by side, wrapping where the window is narrow.
fn row() -> gpui::Div {
    div().flex().flex_wrap().gap(SP_1)
}

/// A group of the page under its heading.
fn group(theme: &Theme, heading: &str) -> gpui::Div {
    div().flex().flex_col().mb(SP_4).child(
        div()
            .mb(SP_1)
            .pb(SP_1)
            .border_b_1()
            .border_color(theme.border)
            .text_size(FS_SM)
            .font_weight(FontWeight::SEMIBOLD)
            .child(text(heading.to_owned())),
    )
}

impl Aoi {
    /// Where the game is read from: what the user chose, or, before they were asked, a phone
    /// while one is linked and looked for or streaming, else the default (the PC's game on
    /// Windows).
    pub(super) fn feed(&self) -> Feed {
        self.settings.feed.unwrap_or_else(|| {
            if matches!(
                self.shown.cast,
                Cast::Waiting { .. } | Cast::Receiving { .. }
            ) {
                Feed::Cast
            } else if matches!(
                self.shown.phone,
                Link::Pairing { .. }
                    | Link::Searching { .. }
                    | Link::Allowing
                    | Link::Streaming { .. }
            ) {
                Feed::Phone
            } else {
                Feed::default()
            }
        })
    }

    /// Reads the game from `feed` from now on, kept for the next start: the others are let
    /// go (a phone disconnected, the wait for a stream ended), and the one chosen started
    /// where it has not been (a phone paired or connected, a stream waited for).
    fn choose_feed(&mut self, feed: Feed, cx: &mut Context<Self>) {
        self.settings.feed = Some(feed);
        self.keep_settings();
        let phone_on = matches!(
            self.shown.phone,
            Link::Pairing { .. } | Link::Searching { .. } | Link::Allowing | Link::Streaming { .. }
        );
        let cast_on = matches!(
            self.shown.cast,
            Cast::Waiting { .. } | Cast::Receiving { .. }
        );
        if feed != Feed::Phone && phone_on {
            self.core.connect_phone(false);
        }
        if feed != Feed::Cast && cast_on {
            self.core.wait_for_stream(false);
        }
        match feed {
            Feed::Window => {}
            Feed::Phone => match self.shown.phone {
                Link::Unpaired | Link::Failed { .. } => self.core.pair_phone(true),
                Link::Stopped { .. } => self.core.connect_phone(true),
                _ => {}
            },
            Feed::Cast => {
                if !cast_on {
                    self.core.wait_for_stream(true);
                }
            }
        }
        cx.notify();
    }

    /// Puts back what the saved file `name` kept and starts the app again over it; a file
    /// that kept nothing changes nothing, and the page says why.
    fn restore(&mut self, name: &str, cx: &mut Context<Self>) {
        self.restoring = None;
        match self.core.restore(name) {
            Ok(()) => {
                // The cursor chosen is put back in by the window that starts.
                self.core.set_cursor(None);
                (self.updates.restart)();
            }
            Err(error) => self.restore_error = Some(error),
        }
        cx.notify();
    }

    pub(super) fn settings_page(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let update = group(theme, "アップデート").child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(SP_3)
                .child(
                    outlined_button(theme, "check", "アップデートを確認", false)
                        .on_click(cx.listener(|this, _, _, cx| this.check_update(cx))),
                )
                .child(
                    div()
                        .text_size(FS_SM)
                        .text_color(theme.muted)
                        .child(text(format!(
                            "今のバージョン v{}",
                            env!("CARGO_PKG_VERSION")
                        ))),
                ),
        );

        let chosen = self.feed();
        let feeds = group(theme, "配信方法").child(row().children(Feed::offered().iter().map(
            |&(feed, name)| {
                choice(theme, &format!("feed-{name}"), name, chosen == feed)
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_feed(feed, cx)))
            },
        )));
        let link: Vec<AnyElement> = match chosen {
            Feed::Window => vec![
                div()
                    .text_size(FS_SM)
                    .text_color(theme.muted)
                    .child(text("この PC のブルーアーカイブの画面を読み取ります"))
                    .into_any_element(),
            ],
            Feed::Phone => self.phone_link(theme, cx),
            Feed::Cast => self.cast_link(theme, cx),
        };
        let feeds = feeds.child(
            div()
                .id("feed-link")
                .mt(SP_2)
                .ml(SP_3)
                .pl(SP_3)
                .border_l_1()
                .border_color(theme.border)
                .children(link),
        );

        let themes = group(theme, "テーマ").child(row().children(
            ThemeChoice::ALL.into_iter().map(|(theme_choice, name)| {
                choice(
                    theme,
                    &format!("theme-{name}"),
                    name,
                    self.settings.theme == theme_choice,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.settings.theme = theme_choice;
                    this.keep_settings();
                    this.apply_theme(window, cx);
                }))
            }),
        ));
        let follow = group(theme, "お仕事の自動開閉").child(row().children(
            WORK_FOLLOW.into_iter().map(|(on, name)| {
                choice(
                    theme,
                    &format!("follow-{name}"),
                    name,
                    self.settings.work_follow == on,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.work_follow = on;
                    this.keep_settings();
                    cx.notify();
                }))
            }),
        ));
        let cursors = self.cursor_shown.then(|| {
            group(theme, "マウスカーソル変更").child(row().children(
                CursorChoice::ALL.into_iter().map(|(cursor, name)| {
                    choice(
                        theme,
                        &format!("cursor-{name}"),
                        name,
                        self.settings.cursor == cursor,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.cursor = cursor;
                        this.keep_settings();
                        this.cursor_shown = this.core.set_cursor(cursor.palette());
                        cx.notify();
                    }))
                }),
            ))
        });

        let nothing_read = self.nothing_read();
        let listing = self.opened.contains("restore");
        let mut data = group(theme, "データ").child(
            div()
                .flex()
                .flex_wrap()
                .gap(SP_2)
                .child(
                    outlined_button(theme, "save", "データを保存", false)
                        .when(nothing_read, |b| {
                            b.aria_description("まだ何も読み取っていません")
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !nothing_read {
                                this.export(cx);
                            }
                        })),
                )
                .child(
                    outlined_button(theme, "restore", "データを復元", false).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.restoring = None;
                            this.restore_error = None;
                            this.toggle_fold("restore", cx);
                        },
                    )),
                ),
        );
        data = data.child(
            div()
                .mt(SP_2)
                .text_size(FS_SM)
                .text_color(theme.muted)
                .child(text(format!(
                    "保存先: {}",
                    self.core.places.exports.display()
                ))),
        );
        if let Some(error) = &self.restore_error {
            data = data.child(div().mt(SP_2).child(widgets::error(
                theme,
                "restore-error",
                format!("復元できませんでした: {error}"),
            )));
        }
        if let Some(name) = self.restoring.clone() {
            let now = name.clone();
            data = data.child(
                div()
                    .id("restore-ask")
                    .mt(SP_2)
                    .flex()
                    .flex_col()
                    .gap(SP_2)
                    .child(text(format!(
                        "{name} の内容で今のデータを置き換え、Aoi を再起動します。"
                    )))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(SP_2)
                            .child(
                                outlined_button(theme, "restore-now", "復元して再起動", true)
                                    .on_click(
                                        cx.listener(move |this, _, _, cx| this.restore(&now, cx)),
                                    ),
                            )
                            .child(
                                outlined_button(theme, "restore-cancel", "やめる", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.restoring = None;
                                        cx.notify();
                                    }),
                                ),
                            ),
                    ),
            );
        } else if listing {
            let saved = self.core.saved();
            if saved.is_empty() {
                data = data.child(
                    div()
                        .mt(SP_2)
                        .text_size(FS_SM)
                        .text_color(theme.muted)
                        .child(text("保存したファイルがありません")),
                );
            }
            data = data.child(
                div()
                    .id("saved")
                    .mt(SP_2)
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(SP_1)
                    .children(saved.into_iter().map(|name| {
                        let id = SharedString::from(format!("saved-{name}"));
                        let chosen = name.clone();
                        outlined_button(theme, &id, &name, false).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.restoring = Some(chosen.clone());
                                this.restore_error = None;
                                cx.notify();
                            },
                        ))
                    })),
            );
        }

        super::scroll("settings", &self.content)
            .child(update)
            .child(feeds)
            .child(themes)
            .child(follow)
            .children(cursors)
            .child(data)
            .into_any_element()
    }
}
