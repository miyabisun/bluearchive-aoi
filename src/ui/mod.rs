//! The window: the tabs, the status and the menu on one row over the tab's content, the
//! notices, the toasts and the dialogs. It shows the snapshot `Core` keeps current and calls
//! `Core`'s commands; what it derives for display comes from `view`.

// A screen's layout reads top to bottom in one function, as the page did; and a layout's
// bindings are named after what they draw (`row`, `rows`), alike by nature.
#![allow(clippy::too_many_lines, clippy::similar_names)]

mod dialogs;
mod equipment;
pub mod icons;
mod items;
mod parts;
mod plan;
mod shops;
mod students;
#[cfg(test)]
mod tests;
pub mod text_input;
pub mod theme;
mod widgets;
mod work;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use futures::{StreamExt as _, channel::mpsc};
use gpui::{
    App, AppContext as _, AsyncApp, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, Task, TitlebarOptions, WeakEntity,
    Window, WindowBounds, WindowOptions, actions, anchored, deferred, div,
    prelude::FluentBuilder as _, px, size,
};

use crate::{
    Recognizer,
    app::{Core, Places, Snapshot, UpdateCheck},
    settings::{CursorChoice, Settings, ThemeChoice, WORK_FOLLOW},
    view,
};

use self::{
    icons::{Assets, Glyph, glyph},
    text_input::TextInput,
    theme::{FONT, FS_CAPTION, FS_MD, FS_SM, RADIUS_SM, SP_1, SP_2, SP_3, SP_4, Theme},
    widgets::text,
};

actions!(aoi, [FocusNext, FocusPrev, Escape]);

/// The window's size as the Tauri versions opened it.
const WINDOW: (f32, f32) = (720., 820.);

/// How long a toast stays before it goes.
const TOAST_STAYS: Duration = Duration::from_secs(3);

/// The tabs, in order (see `view::LISTS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Items,
    Equipment,
    Shops,
    Students,
    Plan,
    Work,
}

impl Tab {
    pub const ALL: [Self; 6] = [
        Self::Items,
        Self::Equipment,
        Self::Shops,
        Self::Students,
        Self::Plan,
        Self::Work,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Items => "アイテム",
            Self::Equipment => "装備",
            Self::Shops => "ショップ",
            Self::Students => "生徒",
            Self::Plan => "育成",
            Self::Work => "お仕事",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Items => "items",
            Self::Equipment => "equipment",
            Self::Shops => "shops",
            Self::Students => "students",
            Self::Plan => "plan",
            Self::Work => "work",
        }
    }
}

/// Where checking for an update stands, while it stays in the page (checking and up to date
/// are toasts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Available(UpdateCheck),
    Installing(UpdateCheck),
    Failed(String),
}

/// How the window asks for the latest release and takes it: GitHub's in the app, a test's own
/// in the tests.
#[derive(Clone, Copy)]
pub struct Updates {
    pub check: fn() -> Result<UpdateCheck, String>,
    pub apply: fn() -> Result<(), String>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            check: crate::app::check_update,
            apply: crate::app::apply_update,
        }
    }
}

/// A dialog over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    Phone,
    Cast,
}

/// The QR code shown while waiting for a streaming app: Moblin's link or the plain URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastCode {
    Moblin,
    Url,
}

/// The star menu open by a target (its student's id), placed by its button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarMenu {
    pub id: i64,
    pub at: Bounds<gpui::Pixels>,
}

pub struct Aoi {
    core: Arc<Core>,
    /// The snapshot shown, taken from `core` on every change.
    shown: Snapshot,
    settings: Settings,
    tab: Tab,
    items_sub: items::Sub,
    equipment_sub: equipment::Sub,
    menu_open: bool,
    menu_focus: FocusHandle,
    /// The window's own focus, where it starts (the search field is not focused at start).
    root_focus: FocusHandle,
    /// Whether the cursor can be changed here (Windows), so the menu offers it.
    cursor_shown: bool,
    query: Entity<TextInput>,
    sort_by: students::Column,
    /// The last export: the path, or why it failed.
    exported: Option<Result<String, String>>,
    update: Option<UpdateState>,
    pub updates: Updates,
    toast: Option<SharedString>,
    toast_timer: Option<Task<()>>,
    /// The folds opened by hand, by key: kept here, so the snapshots of every frame leave them.
    pub(crate) opened: HashSet<String>,
    /// The work tab's fold the game came to last (see `follow_work`).
    followed: Option<String>,
    /// What is paid from each source for each line short (`line|source`), the user's choice,
    /// kept while the plan counts from the same reading (`plan_version`).
    payments: BTreeMap<String, u64>,
    paid_for: u32,
    star_menu: Option<StarMenu>,
    /// The fields of the targets being typed, by student and column.
    target_fields: HashMap<(i64, usize), Entity<TextInput>>,
    dialog: Option<Dialog>,
    cast_code: CastCode,
    content: ScrollHandle,
    /// A held chip paying again and again.
    repeating: Option<Task<()>>,
    _changes: Task<()>,
}

impl Aoi {
    pub fn new(
        core: Arc<Core>,
        mut changes: mpsc::UnboundedReceiver<()>,
        settings: Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| TextInput::new("", "", cx));
        cx.subscribe(&query, |_, _, _: &text_input::Changed, cx| cx.notify())
            .detach();
        let shown = core.snapshot();
        let cursor_shown = core.set_cursor(settings.cursor.palette());
        let changes = cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            while changes.next().await.is_some() {
                // Every change since is in the one snapshot taken.
                while changes.try_recv().is_ok() {}
                if this.update(cx, Aoi::refresh).is_err() {
                    break;
                }
            }
        });
        cx.observe_window_appearance(window, |this: &mut Self, window, cx| {
            this.apply_theme(window, cx);
        })
        .detach();
        let paid_for = shown.plan_version;
        Self {
            core,
            shown,
            settings,
            tab: Tab::Items,
            items_sub: items::Sub::All,
            equipment_sub: equipment::Sub::All,
            menu_open: false,
            menu_focus: cx.focus_handle(),
            root_focus: cx.focus_handle(),
            cursor_shown,
            query,
            sort_by: students::Column::Level,
            exported: None,
            update: None,
            updates: Updates::default(),
            toast: None,
            toast_timer: None,
            opened: HashSet::new(),
            followed: None,
            payments: BTreeMap::new(),
            paid_for,
            star_menu: None,
            target_fields: HashMap::new(),
            dialog: None,
            cast_code: CastCode::Moblin,
            content: ScrollHandle::new(),
            repeating: None,
            _changes: changes,
        }
    }

    /// Takes the snapshot now: the payments go with a new plan, and the work tab follows the
    /// game.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.shown = self.core.snapshot();
        if self.shown.plan_version != self.paid_for {
            self.paid_for = self.shown.plan_version;
            self.payments.clear();
        }
        self.follow_work();
        cx.notify();
    }

    /// The work tab's fold of where the game is, opened as the game comes there while the tab
    /// is open (the user's 2026-11-06 「お仕事タブを開いている時は、各項目に画面遷移したらそのタブを開く」):
    /// once, so one the user closes there stays closed. The others close and it comes to the top
    /// (「他の項目は全て閉じる」「画面上部に表示させる」); none of it while the menu says not to.
    fn follow_work(&mut self) {
        let fold = view::work_fold(self.shown.place, &self.shown.month_end);
        if fold == self.followed {
            return;
        }
        self.followed.clone_from(&fold);
        let Some(fold) = fold else { return };
        if self.tab != Tab::Work || !self.settings.work_follow {
            return;
        }
        self.opened
            .retain(|key| !key.starts_with("work:") || *key == fold);
        self.opened.insert(fold.clone());
        work::scroll_to_fold(&self.content, &fold);
    }

    fn apply_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.set_global(Theme::chosen(self.settings.theme, window.appearance()));
        cx.notify();
    }

    fn keep_settings(&self) {
        let _ = self.settings.save(&self.core.places.settings);
    }

    /// Shows `text` as a toast, gone after a while (staying while `stays` is false).
    fn notify_toast(&mut self, text: Option<&str>, stays: bool, cx: &mut Context<Self>) {
        self.toast = text.map(|t| SharedString::from(t.to_owned()));
        self.toast_timer = None;
        if text.is_some() && !stays {
            self.toast_timer = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TOAST_STAYS).await;
                let _ = this.update(cx, |this, cx| {
                    this.toast = None;
                    cx.notify();
                });
            }));
        }
        cx.notify();
    }

    /// Checks for an update: while checking and when up to date, a toast says so; a newer
    /// version (with the button to take it) and a failure stay in the page.
    fn check_update(&mut self, cx: &mut Context<Self>) {
        self.update = None;
        self.notify_toast(
            Some(&view::update_text(&view::UpdateState::Checking)),
            true,
            cx,
        );
        let check = self.updates.check;
        let found = cx.background_executor().spawn(async move { check() });
        cx.spawn(async move |this, cx| {
            let found = found.await;
            let _ = this.update(cx, |this, cx| match found {
                Ok(found) if found.latest.is_some() => {
                    this.notify_toast(None, false, cx);
                    this.update = Some(UpdateState::Available(found));
                }
                Ok(found) => {
                    this.notify_toast(
                        Some(&view::update_text(&view::UpdateState::Latest {
                            current: found.current.clone(),
                        })),
                        false,
                        cx,
                    );
                }
                Err(error) => {
                    this.notify_toast(None, false, cx);
                    this.update = Some(UpdateState::Failed(error));
                }
            });
        })
        .detach();
    }

    /// Takes the update; on success the app restarts as it. Where the app cannot replace
    /// itself the release page opens and the notice stays.
    fn install(&mut self, cx: &mut Context<Self>) {
        let Some(UpdateState::Available(check)) = self.update.clone() else {
            return;
        };
        if check.installs {
            self.update = Some(UpdateState::Installing(check.clone()));
            cx.notify();
        }
        let apply = self.updates.apply;
        let applied = cx.background_executor().spawn(async move { apply() });
        cx.spawn(async move |this, cx| {
            let applied = applied.await;
            let _ = this.update(cx, |this, cx| match applied {
                Ok(()) if check.installs => {
                    // The cursor chosen is put back in by the window of the new version.
                    this.core.set_cursor(None);
                    crate::app::restart(cx);
                }
                Ok(()) => {}
                Err(error) => {
                    this.update = Some(UpdateState::Failed(error));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        self.exported = Some(self.core.export());
        cx.notify();
    }

    fn toggle_fold(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.opened.remove(key) {
            self.opened.insert(key.to_owned());
        }
        cx.notify();
    }

    fn open_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.menu_open = false;
        self.dialog = Some(dialog);
        match dialog {
            Dialog::Phone => {
                if matches!(
                    self.shown.phone,
                    crate::android::Link::Unpaired | crate::android::Link::Failed { .. }
                ) {
                    self.core.pair_phone(true);
                }
            }
            Dialog::Cast => {
                if matches!(
                    self.shown.cast,
                    crate::broadcast::Cast::Off | crate::broadcast::Cast::Failed { .. }
                ) {
                    self.core.wait_for_stream(true);
                }
            }
        }
        cx.notify();
    }

    /// Whether nothing of any list was read: then there is nothing to export.
    fn nothing_read(&self) -> bool {
        let s = &self.shown;
        s.items.is_empty()
            && s.equipment.is_empty()
            && s.shops.is_empty()
            && s.students.is_empty()
            && s.plan.is_empty()
    }

    fn header(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let students = self.shown.students.len();
        let tabs = div()
            .id("tabs")
            .role(gpui::Role::TabList)
            .flex()
            .flex_1()
            .min_w_0()
            .px(SP_1)
            .overflow_x_scroll()
            .children(Tab::ALL.into_iter().map(|tab| {
                let selected = self.tab == tab;
                let name = if tab == Tab::Students && students > 0 {
                    format!("{} {students}", tab.name())
                } else {
                    tab.name().to_owned()
                };
                div()
                    .id(SharedString::from(format!("tab-{}", tab.id())))
                    .debug_selector(move || format!("tab:{}", tab.name()))
                    .role(gpui::Role::Tab)
                    .aria_label(name)
                    .aria_selected(selected)
                    .tab_stop(true)
                    .flex()
                    .flex_none()
                    .items_center()
                    .h(px(40.))
                    .px(SP_3)
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
                    .when(selected, |t| t.font_weight(FontWeight::SEMIBOLD))
                    .cursor_pointer()
                    .child(tab.name())
                    .when(tab == Tab::Students && students > 0, |t| {
                        t.child(
                            div()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(theme.muted)
                                .font_features(theme::tabular())
                                .child(text(format!("\u{a0}{students}"))),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        cx.notify();
                    }))
            }));
        let status = view::status_view(&self.shown.status, &self.shown.phone, &self.shown.cast);
        let paused = matches!(self.shown.status, crate::watch::Status::Paused);
        let (lamp, words) = match status.tone {
            view::Tone::Active => (theme.accent, theme.on_surface),
            view::Tone::Danger => (theme.danger, theme.danger),
            view::Tone::Paused => (theme.danger, theme.on_surface),
            _ => (theme.muted, theme.muted),
        };
        let source = status.source.map(|source| match source {
            view::Capture::Window => Glyph::Window,
            view::Capture::Phone => Glyph::Phone,
            view::Capture::Cast => Glyph::Cast,
        });
        let border = theme.border;
        let hover = theme.muted;
        let badge = div()
            .id("status")
            .debug_selector(|| "status".to_owned())
            .role(gpui::Role::Button)
            .aria_label(status.label.clone())
            .aria_description(status.title.clone())
            .aria_toggled(if paused {
                gpui::Toggled::True
            } else {
                gpui::Toggled::False
            })
            .tab_stop(true)
            .flex()
            .flex_none()
            .items_center()
            .gap(SP_2)
            .px(SP_3)
            .py(px(2.))
            .border_1()
            .border_color(border)
            .rounded_full()
            .text_size(FS_SM)
            .text_color(words)
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(move |s| s.border_color(hover))
            .focus(move |s| s.border_color(hover))
            .tooltip(widgets::tooltip(status.title.clone()))
            .child(match source {
                Some(g) => glyph(g, px(14.), lamp).into_any_element(),
                None => div()
                    .size(px(8.))
                    .flex_none()
                    .rounded_full()
                    .bg(lamp)
                    .into_any_element(),
            })
            .child(text(status.label.clone()))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.core.set_paused(!paused);
                cx.notify();
            }));
        let menu_button = div()
            .id("menu-button")
            .debug_selector(|| "menu-button".to_owned())
            .role(gpui::Role::Button)
            .aria_label("メニュー")
            .when(self.menu_open, |b| b.aria_description("開いています"))
            .tab_stop(true)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(32.))
            .rounded(RADIUS_SM)
            .cursor_pointer()
            .child(glyph(Glyph::Menu, px(20.), theme.on_surface))
            .on_click(cx.listener(|this, _, window, cx| {
                this.menu_open = !this.menu_open;
                if this.menu_open {
                    this.menu_focus.focus(window, cx);
                }
                cx.notify();
            }));
        div()
            .flex_none()
            .bg(theme.wash_base)
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(SP_2)
                    .pr(SP_3)
                    .child(tabs)
                    .child(
                        div()
                            .id("status-slot")
                            .ml_auto()
                            .flex_none()
                            .role(gpui::Role::Status)
                            .child(badge),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_none()
                            .child(menu_button)
                            .when(self.menu_open, |m| m.child(self.menu(theme, cx))),
                    ),
            )
    }

    /// The menu under its button: exports, updates, the phone and the streaming app, then the
    /// theme, the work tab's following and (on Windows) the cursor, each a group of choices.
    fn menu(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let nothing_read = self.nothing_read();
        let item = |id: &str, label: &str, disabled: bool| {
            widgets::menu_item(theme, id, label, None, disabled)
        };
        let group = |heading: &str| {
            div()
                .flex()
                .flex_col()
                .mt(SP_1)
                .pt(SP_1)
                .border_t_1()
                .border_color(theme.border)
                .child(
                    div()
                        .px(SP_3)
                        .pt(SP_1)
                        .text_size(FS_CAPTION)
                        .text_color(theme.muted)
                        .child(heading.to_owned()),
                )
        };
        let themes =
            group("テーマ").children(ThemeChoice::ALL.into_iter().map(|(choice, name)| {
                widgets::menu_item(
                    theme,
                    &format!("theme-{name}"),
                    name,
                    Some(self.settings.theme == choice),
                    false,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.settings.theme = choice;
                    this.keep_settings();
                    this.menu_open = false;
                    this.apply_theme(window, cx);
                }))
            }));
        let follow =
            group("お仕事の自動開閉").children(WORK_FOLLOW.into_iter().map(|(on, name)| {
                widgets::menu_item(
                    theme,
                    &format!("follow-{name}"),
                    name,
                    Some(self.settings.work_follow == on),
                    false,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.work_follow = on;
                    this.keep_settings();
                    this.menu_open = false;
                    cx.notify();
                }))
            }));
        let cursors = self.cursor_shown.then(|| {
            group("マウスカーソル変更").children(CursorChoice::ALL.into_iter().map(
                |(choice, name)| {
                    widgets::menu_item(
                        theme,
                        &format!("cursor-{name}"),
                        name,
                        Some(self.settings.cursor == choice),
                        false,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.cursor = choice;
                        this.keep_settings();
                        this.cursor_shown = this.core.set_cursor(choice.palette());
                        this.menu_open = false;
                        cx.notify();
                    }))
                },
            ))
        });
        deferred(
            anchored().snap_to_window().child(
                div()
                    .id("menu")
                    .role(gpui::Role::Menu)
                    .aria_label("メニュー")
                    .track_focus(&self.menu_focus)
                    .key_context("Menu")
                    .on_action(cx.listener(|this, _: &Escape, _, cx| {
                        this.menu_open = false;
                        cx.notify();
                    }))
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        cx.notify();
                    }))
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .min_w(px(11. * 14.))
                    .py(SP_1)
                    .border_1()
                    .border_color(theme.border)
                    .rounded(RADIUS_SM)
                    .bg(theme.surface)
                    .child(
                        item("export", "YAMLを書き出す", nothing_read).on_click(cx.listener(
                            move |this, _, _, cx| {
                                if !nothing_read {
                                    this.menu_open = false;
                                    this.export(cx);
                                }
                            },
                        )),
                    )
                    .child(
                        item("check", "アップデートを確認", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.menu_open = false;
                                this.check_update(cx);
                            },
                        )),
                    )
                    .child(item("phone", "Androidとの通信", false).on_click(
                        cx.listener(|this, _, _, cx| this.open_dialog(Dialog::Phone, cx)),
                    ))
                    .child(
                        item("cast", "配信アプリとの連携", false).on_click(
                            cx.listener(|this, _, _, cx| this.open_dialog(Dialog::Cast, cx)),
                        ),
                    )
                    .child(themes)
                    .child(follow)
                    .children(cursors),
            ),
        )
        .with_priority(2)
    }

    /// The notices over the tab's content: the capture's failure, what could not be kept, the
    /// export, and the update.
    fn notices(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let mut notices = Vec::new();
        if let crate::watch::Status::Failed { message } = &self.shown.status {
            notices
                .push(widgets::error(theme, "capture-error", message.clone()).into_any_element());
        }
        if let Some(error) = &self.shown.keep_error {
            notices.push(widgets::error(theme, "keep-error", error.clone()).into_any_element());
        }
        match &self.exported {
            Some(Err(error)) => notices.push(
                widgets::error(
                    theme,
                    "export-error",
                    format!("書き出せませんでした: {error}"),
                )
                .into_any_element(),
            ),
            Some(Ok(path)) => notices.push(
                widgets::quiet(theme, "exported", format!("書き出しました: {path}"))
                    .into_any_element(),
            ),
            None => {}
        }
        match self.update.clone() {
            Some(UpdateState::Failed(error)) => notices.push(
                widgets::error(
                    theme,
                    "update-error",
                    view::update_text(&view::UpdateState::Failed { error }),
                )
                .into_any_element(),
            ),
            Some(UpdateState::Available(check)) => {
                let label = if check.installs {
                    "更新して再起動"
                } else {
                    "入手ページを開く"
                };
                notices.push(
                    widgets::quiet(
                        theme,
                        "update",
                        view::update_text(&view::UpdateState::Available {
                            current: check.current.clone(),
                            latest: check.latest.clone().unwrap_or_default(),
                        }),
                    )
                    .flex()
                    .items_center()
                    .child(
                        widgets::outlined_button(theme, "install", label, true)
                            .ml(SP_3)
                            .on_click(cx.listener(|this, _, _, cx| this.install(cx))),
                    )
                    .into_any_element(),
                );
            }
            Some(UpdateState::Installing(check)) => notices.push(
                widgets::quiet(
                    theme,
                    "update",
                    view::update_text(&view::UpdateState::Installing {
                        latest: check.latest.clone().unwrap_or_default(),
                    }),
                )
                .into_any_element(),
            ),
            None => {}
        }
        notices
    }
}

impl Focusable for Aoi {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.root_focus.clone()
    }
}

impl Render for Aoi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        let header = self.header(&theme, cx);
        let notices = self.notices(&theme, cx);
        let content = match self.tab {
            Tab::Items => self.items_tab(&theme, window, cx),
            Tab::Equipment => self.equipment_tab(&theme, window, cx),
            Tab::Shops => self.shops_tab(&theme, window, cx),
            Tab::Students => self.students_tab(&theme, window, cx),
            Tab::Plan => self.plan_tab(&theme, window, cx),
            Tab::Work => self.work_tab(&theme, window, cx),
        };
        let dialog = self.dialog.map(|d| self.dialog_view(d, &theme, cx));
        let star_menu = self
            .star_menu
            .and_then(|m| self.star_menu_view(m, &theme, cx));
        div()
            .id("aoi")
            .key_context("Aoi")
            .track_focus(&self.root_focus)
            .on_action(cx.listener(|_, _: &FocusNext, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrev, window, cx| window.focus_prev(cx)))
            .on_action(cx.listener(|this, _: &Escape, _, cx| {
                this.menu_open = false;
                this.star_menu = None;
                this.dialog = None;
                cx.notify();
            }))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.surface)
            .text_color(theme.on_surface)
            .font_family(FONT)
            .text_size(FS_MD)
            .line_height(gpui::relative(1.6))
            .child(header)
            .child(
                div()
                    .id("list")
                    .role(gpui::Role::TabPanel)
                    .aria_label(self.tab.name())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .max_w(px(WINDOW.0))
                    .mx_auto()
                    .pt(SP_4)
                    .px(SP_3)
                    .children(notices)
                    .child(content),
            )
            .when_some(self.toast.clone(), |root, toast| {
                root.child(
                    div()
                        .absolute()
                        .left(SP_4)
                        .right(SP_4)
                        .bottom(SP_4)
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .id("toast")
                                .role(gpui::Role::Status)
                                .aria_label(toast.clone())
                                .px(SP_4)
                                .py(SP_2)
                                .border_1()
                                .border_color(theme.border)
                                .rounded(RADIUS_SM)
                                .bg(theme.surface)
                                .text_size(FS_SM)
                                .child(text(toast)),
                        ),
                )
            })
            .children(star_menu)
            .children(dialog)
    }
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrev, None),
        KeyBinding::new("escape", Escape, None),
    ]);
    text_input::bind_keys(cx);
}

/// Opens the window over what `core` keeps, its changes signalled on `changes`.
pub fn open(
    core: Arc<Core>,
    changes: mpsc::UnboundedReceiver<()>,
    settings: Settings,
    cx: &mut App,
) -> gpui::WindowHandle<Aoi> {
    let bounds = Bounds::centered(None, size(px(WINDOW.0), px(WINDOW.1)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Aoi".into()),
                ..Default::default()
            }),
            app_id: Some(crate::app::IDENTIFIER.into()),
            ..Default::default()
        },
        |window, cx| {
            cx.set_global(Theme::chosen(settings.theme, window.appearance()));
            let aoi = cx.new(|cx| Aoi::new(core, changes, settings, window, cx));
            aoi.focus_handle(cx).focus(window, cx);
            aoi
        },
    )
    .expect("the window opens")
}

/// Runs the app: reads the game, and shows it in the window.
pub fn run(recognizer: Arc<Recognizer>, places: Places) {
    let settings = Settings::load(&places.settings);
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            bind_keys(cx);
            let (kept, trouble) = crate::app::load_kept(&recognizer, &places);
            let (changed, changes) = mpsc::unbounded();
            let phone_dir = places
                .state
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            let core = Arc::new(Core::new(recognizer, places, &kept, trouble, changed, true));
            core.watch(kept, phone_dir);
            let _ = open(core, changes, settings, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        });
}

/// A tab's sub-tabs over a rule; narrower than they are, they scroll sideways.
pub fn subtabs<T: Copy + PartialEq + 'static>(
    theme: &Theme,
    id: &str,
    choices: &[(T, &'static str)],
    chosen: T,
    on_choose: impl Fn(&T, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_choose = std::rc::Rc::new(on_choose);
    div()
        .id(SharedString::from(format!("subtabs-{id}")))
        .role(gpui::Role::TabList)
        .aria_label("分類")
        .flex()
        .flex_none()
        .mb(SP_3)
        .overflow_x_scroll()
        .border_b_1()
        .border_color(theme.border)
        .children(choices.iter().map(|&(choice, name)| {
            let selected = choice == chosen;
            let on_choose = on_choose.clone();
            div()
                .id(SharedString::from(format!("sub-{id}-{name}")))
                .debug_selector(move || format!("sub:{name}"))
                .role(gpui::Role::Tab)
                .aria_label(name)
                .aria_selected(selected)
                .tab_stop(true)
                .flex_none()
                .px(SP_3)
                .py(SP_1)
                .mb(px(-1.))
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
                .when(selected, |t| t.font_weight(FontWeight::SEMIBOLD))
                .whitespace_nowrap()
                .cursor_pointer()
                .child(name)
                .on_click(move |_, window, cx| on_choose(&choice, window, cx))
        }))
}

/// The box a tab's content scrolls in, under what stays above it.
pub fn scroll(id: &str, handle: &ScrollHandle) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(handle)
}

/// The search field with its caption over it; its border the accent while it has the focus.
pub fn search(
    theme: &Theme,
    field: &Entity<TextInput>,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let focused = field.read(cx).focus_handle(cx).is_focused(window);
    let border = if focused { theme.accent } else { theme.border };
    div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(SP_1)
        .mb(SP_4)
        .child(
            div()
                .text_size(FS_CAPTION)
                .text_color(theme.muted)
                .child("名前で検索"),
        )
        .child(
            div()
                .id("search")
                .debug_selector(|| "search".to_owned())
                .role(gpui::Role::SearchInput)
                .aria_label("名前で検索")
                .h(px(40.))
                .px(SP_3)
                .border_1()
                .border_color(border)
                .rounded(RADIUS_SM)
                .bg(theme.surface)
                .child(field.clone()),
        )
}

/// The face a tile is painted: a rarity's tier colour, or an eleph's pink.
pub fn face_color(theme: &Theme, face: Option<view::Face>) -> Option<gpui::Hsla> {
    use crate::icon::Rarity;
    face.map(|face| match face {
        view::Face::Rarity(Rarity::N) => theme.tiers[0],
        view::Face::Rarity(Rarity::R) => theme.tiers[1],
        view::Face::Rarity(Rarity::SR) => theme.tiers[2],
        view::Face::Rarity(Rarity::SSR) => theme.tiers[3],
        view::Face::Eleph => theme.eleph,
    })
}
