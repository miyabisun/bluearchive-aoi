//! Where a phone's screen comes from, as the settings page shows it under the feed chosen: the
//! link to a phone (paired by a QR code, then connected or not) and waiting for a streaming app
//! (a QR code to send the phone's screen to).

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, canvas, div, fill, point,
    prelude::FluentBuilder as _, px, rgb, size,
};

use super::{
    Aoi, CastCode,
    theme::{FS_SM, SP_1, SP_2, SP_3, Theme},
    widgets::{outlined_button, text},
};
use crate::{android::Link, broadcast::Cast, view};

/// A QR code of `modules` (rows, dark ones true), black on white whatever the theme (a phone
/// reads it so), 240 px square with its quiet zone.
fn qr(modules: &[Vec<bool>], label: &str) -> impl IntoElement {
    let side = modules.len() + 2 * view::QR_QUIET;
    let squares = view::qr_squares(modules);
    div()
        .id(SharedString::from(format!("qr-{label}")))
        .role(Role::Image)
        .aria_label(label.to_owned())
        .flex_none()
        .size(px(240.))
        .mx_auto()
        .mb(SP_3)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let unit = bounds.size.width / side as f32;
                    window.paint_quad(fill(bounds, rgb(0x00ff_ffff)));
                    for &(x, y) in &squares {
                        let at = point(
                            bounds.origin.x + unit * x as f32,
                            bounds.origin.y + unit * y as f32,
                        );
                        // A hair over a unit, so neighbours meet without seams.
                        let cell = gpui::Bounds::new(at, size(unit + px(0.5), unit + px(0.5)));
                        window.paint_quad(fill(cell, rgb(0x0000_0000)));
                    }
                },
            )
            .size_full(),
        )
}

/// Numbered steps (body-sm).
fn steps(items: &[&str]) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(SP_1)
        .mb(SP_3)
        .text_size(FS_SM)
        .children(items.iter().enumerate().map(|(i, step)| {
            div()
                .flex()
                .gap(SP_2)
                .child(div().flex_none().child(format!("{}.", i + 1)))
                .child(text((*step).to_owned()))
        }))
}

impl Aoi {
    /// A help that opens under its title, closed until opened (body-sm, muted).
    fn help(
        &mut self,
        key: &str,
        title: &str,
        body: String,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.opened.contains(key);
        let key = key.to_owned();
        div()
            .mb(SP_3)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(
                super::widgets::fold_title(theme, &key, open)
                    .aria_label(title.to_owned())
                    .child(title.to_owned())
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_fold(&key, cx))),
            )
            .when(open, |help| help.child(div().mt(SP_2).child(text(body))))
    }

    pub(super) fn phone_link(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let phone = self.shown.phone.clone();
        let mut parts = vec![
            div()
                .mb(SP_3)
                .when(matches!(phone, Link::Failed { .. }), |l| {
                    l.text_color(theme.danger)
                })
                .child(text(view::phone_line(&phone)))
                .into_any_element(),
        ];
        if let Link::Pairing { qr: modules } = &phone {
            parts.push(qr(modules, "ペア設定の QR コード").into_any_element());
            parts.push(
                steps(&[
                    "スマホの「設定」→「デバイス情報」→「ビルド番号」を 7 回タップする（初回だけ）",
                    "「設定」→「システム」→「開発者向けオプション」を開く",
                    "USB ケーブルなら「USB デバッグ」をオンにしてつなぐ。Wi-Fi なら「ワイヤレス デバッグ」をオンにして開き、「QR コードによるデバイスのペア設定」でこの QR コードを読み取る",
                ])
                .into_any_element(),
            );
        }
        if matches!(phone, Link::Pairing { .. } | Link::Searching { .. }) {
            let searching = matches!(phone, Link::Searching { .. });
            let body = format!(
                "Wi-Fi では、PC とスマホを同じ Wi-Fi につないでください。ゲスト用の Wi-Fi など、機器どうしが通信できない回線では使えません。USB では、充電専用でない（データを送れる）ケーブルでつないでください。{}",
                if searching {
                    "スマホの「USB デバッグ」か「ワイヤレス デバッグ」がオンかも確かめてください。"
                } else {
                    ""
                }
            );
            let title = if searching {
                "見つからないとき"
            } else {
                "読み取っても進まないとき"
            };
            parts.push(
                self.help("phone-help", title, body, theme, cx)
                    .into_any_element(),
            );
        }
        let mut actions = Vec::new();
        match phone {
            Link::Searching { .. } | Link::Streaming { .. } => actions.push(
                outlined_button(theme, "disconnect", "切断", false)
                    .on_click(cx.listener(|this, _, _, _| this.core.connect_phone(false)))
                    .into_any_element(),
            ),
            Link::Stopped { .. } => actions.push(
                outlined_button(theme, "connect", "接続", true)
                    .on_click(cx.listener(|this, _, _, _| this.core.connect_phone(true)))
                    .into_any_element(),
            ),
            _ => {}
        }
        match phone {
            Link::Failed { .. } => actions.push(
                outlined_button(theme, "pair-again", "もう一度", true)
                    .on_click(cx.listener(|this, _, _, _| this.core.pair_phone(true)))
                    .into_any_element(),
            ),
            Link::Pairing { .. } | Link::Unpaired => {}
            _ => actions.push(
                outlined_button(theme, "pair-other", "別のスマホとペア設定", false)
                    .on_click(cx.listener(|this, _, _, _| this.core.pair_phone(false)))
                    .into_any_element(),
            ),
        }
        parts.push(actions_row(actions).into_any_element());
        parts
    }

    pub(super) fn cast_link(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let cast = self.shown.cast.clone();
        let mut parts = vec![
            div()
                .mb(SP_3)
                .when(matches!(cast, Cast::Failed { .. }), |l| {
                    l.text_color(theme.danger)
                })
                .child(text(view::cast_line(&cast)))
                .into_any_element(),
        ];
        if let Some(note) = view::cast_note(&cast) {
            parts.push(
                div()
                    .mb(SP_3)
                    .text_color(theme.danger)
                    .child(text(note))
                    .into_any_element(),
            );
        }
        if let Cast::Waiting {
            url, moblin, plain, ..
        } = &cast
        {
            let code = self.cast_code;
            let choice = |id: CastCode, name: &str, cx: &mut Context<Self>| {
                let selected = code == id;
                div()
                    .id(SharedString::from(format!("code-{name}")))
                    .debug_selector({
                        let name = name.to_owned();
                        move || format!("code:{name}")
                    })
                    .role(Role::Tab)
                    .aria_label(name.to_owned())
                    .aria_selected(selected)
                    .tab_stop(true)
                    .px(SP_3)
                    .py(SP_1)
                    .border_b_2()
                    .border_color(if selected {
                        theme.accent
                    } else {
                        gpui::transparent_black()
                    })
                    .text_size(FS_SM)
                    .text_color(if selected {
                        theme.on_surface
                    } else {
                        theme.muted
                    })
                    // As bold chosen or not, like every tab, so that neither moves.
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .child(name.to_owned())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.cast_code = id;
                        cx.notify();
                    }))
            };
            parts.push(
                div()
                    .id("code-choice")
                    .role(Role::TabList)
                    .flex()
                    .justify_center()
                    .mb(SP_2)
                    .child(choice(CastCode::Moblin, "Moblin", cx))
                    .child(choice(CastCode::Url, "ほかのアプリ", cx))
                    .into_any_element(),
            );
            let (modules, label) = match code {
                CastCode::Moblin => (moblin.clone(), "Moblin に取り込む QR コード"),
                CastCode::Url => (plain.clone(), "配信先の URL の QR コード"),
            };
            parts.push(qr(&modules, label).into_any_element());
            parts.push(
                div()
                    .mb(SP_3)
                    .flex()
                    .justify_center()
                    .text_size(FS_SM)
                    .child(super::widgets::read("cast-url", url.clone()))
                    .into_any_element(),
            );
            let mut list: Vec<&str> = match code {
                CastCode::Moblin => vec![
                    "iPhone のカメラでこの QR コードを読み取り、Moblin に取り込む",
                    "Moblin で画面の取り込み（Screen capture）を始め、配信を始める",
                ],
                CastCode::Url => vec![
                    "配信アプリの配信先（RTMP）に上の URL を入れる",
                    "映像を H.264 にし、解像度とビットレートを高めにして配信を始める",
                ],
            };
            list.push("ブルーアーカイブを開く");
            parts.push(steps(&list).into_any_element());
            parts.push(
                self.help(
                    "cast-help",
                    "届かないとき",
                    "PC とスマホを同じ Wi-Fi につないでください。ゲスト用の Wi-Fi など、機器どうしが通信できない回線では使えません。PC に「ネットワークへのアクセスを許可しますか」と出たら、プライベート ネットワークで許可してください。10 分間届かないと、待ち受けを終えます。".to_owned(),
                    theme,
                    cx,
                )
                .into_any_element(),
            );
        }
        let action = match cast {
            Cast::Waiting { .. } | Cast::Receiving { .. } => {
                outlined_button(theme, "stop-waiting", "やめる", false)
                    .on_click(cx.listener(|this, _, _, _| this.core.wait_for_stream(false)))
            }
            Cast::Failed { .. } => outlined_button(theme, "wait-again", "もう一度", true)
                .on_click(cx.listener(|this, _, _, _| this.core.wait_for_stream(true))),
            Cast::Off => outlined_button(theme, "wait", "待ち受ける", true)
                .on_click(cx.listener(|this, _, _, _| this.core.wait_for_stream(true))),
        };
        parts.push(actions_row(vec![action.into_any_element()]).into_any_element());
        parts
    }
}

/// A link's buttons in a row under it.
fn actions_row(actions: Vec<AnyElement>) -> impl IntoElement {
    div().flex().flex_wrap().gap(SP_2).children(actions)
}
