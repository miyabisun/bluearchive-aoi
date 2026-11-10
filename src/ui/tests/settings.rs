//! The settings page: the update checked and taken, where the game is read from (the link to a
//! phone, waiting for a streaming app), the theme and the cursor kept for the next start, the work
//! tab's following, and what was read saved and restored.

use std::sync::atomic::{AtomicUsize, Ordering};

use gpui::{TestAppContext, VisualTestContext};

use super::{app::items, bounds, click, has, open, open_with, set, shows};
use crate::{
    android::Link,
    app::UpdateCheck,
    broadcast::Cast,
    keep::Kept,
    settings::{CursorChoice, Feed, Settings, ThemeChoice},
    ui::{Aoi, Updates, theme::Theme},
    watch::Place,
};

fn kept_settings(core: &crate::app::Core) -> Settings {
    Settings::load(&core.places.settings)
}

/// The page lists what the user asked on top (2026-11-10: 「アップデートを確認: これは一番上にある
/// べきだろう」「配信方法: 何かのグループでくくるべき」「テーマ: 自動が一番上であるべき」), then the
/// rest of what the menu held.
#[gpui::test]
fn the_settings_page_lists_its_groups_in_order(cx: &mut TestAppContext) {
    let (_, _core, vcx) = open(cx, &Kept::default());
    assert!(!has(vcx, "check"), "closed until asked for");
    click(vcx, "settings-button");
    let mut down = vec!["check"];
    if cfg!(windows) {
        down.push("choice-feed-PC版アプリ");
    } else {
        assert!(
            !has(vcx, "choice-feed-PC版アプリ"),
            "the PC's game where it runs alone"
        );
    }
    down.extend([
        "choice-feed-Androidとの通信",
        "choice-feed-配信アプリとの連携",
        "choice-theme-自動",
        "choice-theme-ライト",
        "choice-theme-ダーク",
        "choice-follow-する",
        "save",
        "restore",
    ]);
    let tops: Vec<gpui::Pixels> = down
        .iter()
        .map(|selector| {
            bounds(vcx, selector)
                .unwrap_or_else(|| panic!("{selector}"))
                .top()
        })
        .collect();
    assert!(tops.is_sorted(), "{down:?} at {tops:?}");
    // A group's choices stand side by side, the first asked for at the left.
    let left = |vcx: &mut VisualTestContext, selector: &str| bounds(vcx, selector).unwrap().left();
    assert!(left(vcx, "choice-theme-自動") < left(vcx, "choice-theme-ライト"));
    assert!(left(vcx, "choice-theme-ライト") < left(vcx, "choice-theme-ダーク"));
    if cfg!(windows) {
        assert!(left(vcx, "choice-feed-PC版アプリ") < left(vcx, "choice-feed-Androidとの通信"));
    }
    if cfg!(windows) {
        assert!(
            shows(vcx, "この PC のブルーアーカイブの画面を読み取ります"),
            "the PC's game by default"
        );
    }
}

#[gpui::test]
fn the_settings_page_gives_way_to_a_tab_and_to_escape(cx: &mut TestAppContext) {
    let (_, _core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    assert!(has(vcx, "check"), "open");
    super::keys(vcx, "escape");
    assert!(!has(vcx, "check"), "Escape goes back to the tab");
    click(vcx, "settings-button");
    click(vcx, "tab:装備");
    assert!(!has(vcx, "check"), "a tab chosen shows");
    click(vcx, "settings-button");
    click(vcx, "settings-button");
    assert!(!has(vcx, "check"), "its button again closes it");
}

#[gpui::test]
fn the_settings_choose_the_theme_kept_for_the_next_start(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    click(vcx, "choice-theme-ライト");
    assert!(!vcx.update(|_, cx| Theme::get(cx).dark), "light now");
    assert_eq!(kept_settings(&core).theme, ThemeChoice::Light, "kept");
    assert!(has(vcx, "choice-theme-ダーク"), "the page stays");
    click(vcx, "choice-theme-ダーク");
    assert!(vcx.update(|_, cx| Theme::get(cx).dark), "dark now");
    assert_eq!(kept_settings(&core).theme, ThemeChoice::Dark, "kept");
}

#[gpui::test]
fn the_settings_change_the_mouse_cursor_kept_for_the_next_start(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    if cfg!(windows) {
        click(vcx, "choice-cursor-赤");
        assert_eq!(kept_settings(&core).cursor, CursorChoice::Red, "kept");
    } else {
        assert!(
            !has(vcx, "choice-cursor-赤"),
            "where the cursor cannot change, not offered"
        );
    }
}

fn answering(aoi: &gpui::Entity<Aoi>, vcx: &mut VisualTestContext, updates: Updates) {
    aoi.update(vcx, |aoi, _| aoi.updates = updates);
}

#[gpui::test]
fn the_settings_check_for_an_update_and_tell_what_they_found(cx: &mut TestAppContext) {
    let (aoi, _core, vcx) = open(cx, &Kept::default());
    answering(
        &aoi,
        vcx,
        Updates {
            check: || {
                Ok(UpdateCheck {
                    current: "v0.3.0".to_owned(),
                    latest: None,
                    installs: true,
                })
            },
            ..Updates::default()
        },
    );
    click(vcx, "settings-button");
    click(vcx, "check");
    assert!(
        shows(vcx, "最新のバージョンです（v0.3.0）"),
        "up to date, a toast says so"
    );
    vcx.executor()
        .advance_clock(std::time::Duration::from_secs(4));
    vcx.run_until_parked();
    assert!(!shows(vcx, "最新のバージョンです（v0.3.0）"), "and goes");

    answering(
        &aoi,
        vcx,
        Updates {
            check: || {
                Ok(UpdateCheck {
                    current: "v0.3.0".to_owned(),
                    latest: Some("v0.3.1".to_owned()),
                    installs: false,
                })
            },
            apply: || Err("ページを開けませんでした".to_owned()),
            ..Updates::default()
        },
    );
    click(vcx, "check");
    assert!(
        shows(vcx, "v0.3.1 があります（今 v0.3.0）"),
        "a newer version stays in the page"
    );
    assert!(has(vcx, "install"), "with the button to take it");
    click(vcx, "install");
    assert!(
        shows(
            vcx,
            "アップデートできませんでした: ページを開けませんでした"
        ),
        "a failure says why"
    );

    answering(
        &aoi,
        vcx,
        Updates {
            check: || Err("通信できませんでした".to_owned()),
            ..Updates::default()
        },
    );
    click(vcx, "check");
    assert!(
        shows(vcx, "アップデートできませんでした: 通信できませんでした"),
        "a check that fails says why"
    );
}

#[gpui::test]
fn the_settings_link_a_phone(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    click(vcx, "choice-feed-Androidとの通信");
    assert_eq!(kept_settings(&core).feed, Some(Feed::Phone), "kept");
    set(&core, vcx, |s| {
        s.phone = Link::Pairing {
            qr: vec![vec![true, false], vec![false, true]],
        };
    });
    assert!(
        shows(
            vcx,
            "この QR コードを読み取るか、USB ケーブルでつないでください"
        ),
        "a QR code to pair"
    );
    set(&core, vcx, |s| {
        s.phone = Link::Stopped {
            device: "Pixel 9".to_owned(),
        };
    });
    assert!(
        shows(vcx, "Pixel 9 とは切断しています"),
        "the phone left unconnected"
    );
    assert!(has(vcx, "connect"), "connect offered");
    assert!(!has(vcx, "close-dialog"), "in the page, nothing to close");
}

#[gpui::test]
fn the_settings_wait_for_a_streaming_app(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    click(vcx, "choice-feed-配信アプリとの連携");
    assert_eq!(kept_settings(&core).feed, Some(Feed::Cast), "kept");
    set(&core, vcx, |s| {
        s.cast = Cast::Waiting {
            url: "rtmp://192.168.1.20:1935/live/key".to_owned(),
            moblin: vec![vec![true]],
            plain: vec![vec![false]],
            problem: None,
        };
    });
    assert!(
        shows(vcx, "rtmp://192.168.1.20:1935/live/key"),
        "the URL to send to"
    );
    assert!(has(vcx, "stop-waiting"), "and how to stop");
    set(&core, vcx, |s| {
        s.cast = Cast::Receiving {
            from: "192.168.1.30".to_owned(),
            sideways: true,
        };
    });
    assert!(
        shows(vcx, "192.168.1.30 から画面を受け取っています"),
        "the stream coming in"
    );
}

/// A phone that streamed before the page asked where the game is read from is the choice
/// shown, not the default.
#[gpui::test]
fn a_phone_linked_before_is_the_feed_shown(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.phone = Link::Stopped {
            device: "Legion Tab".to_owned(),
        };
    });
    click(vcx, "settings-button");
    if cfg!(windows) {
        assert!(
            !shows(vcx, "Legion Tab とは切断しています"),
            "a phone left unconnected: the PC's game"
        );
    }
    set(&core, vcx, |s| {
        s.phone = Link::Searching {
            device: "Legion Tab".to_owned(),
        };
    });
    assert!(
        has(vcx, "disconnect"),
        "a phone looked for: the phone's link"
    );
}

static RESTARTS: AtomicUsize = AtomicUsize::new(0);

/// What was read is saved to a file and comes back from it: the file chosen among those saved,
/// the user's word asked before it replaces what is kept, then the app starts again over it
/// (the user's of 2026-11-10: 「書き出したから何？となる、保存・復元機能として使うのが良いかもしれない」).
#[gpui::test]
fn what_is_saved_is_restored_once_the_user_says_so(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = open(cx, &Kept::default());
    answering(
        &aoi,
        vcx,
        Updates {
            restart: || {
                RESTARTS.fetch_add(1, Ordering::Relaxed);
            },
            ..Updates::default()
        },
    );
    set(&core, vcx, |s| s.items = items());
    click(vcx, "settings-button");
    click(vcx, "save");
    assert!(has(vcx, "exported"), "where it was saved is told");
    let saved: Vec<String> = std::fs::read_dir(&core.places.exports)
        .expect("the folder of saved files")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(saved.len(), 1, "one file saved");
    let held: std::collections::BTreeMap<i64, crate::quantity::Count> =
        items().iter().map(|row| (row.id, row.count)).collect();

    // Read on after the save: the items are gone from the list.
    set(&core, vcx, |s| s.items.clear());
    click(vcx, "restore");
    click(vcx, &format!("saved-{}", saved[0]));
    assert!(has(vcx, "restore-now"), "asked before anything is replaced");
    assert_eq!(RESTARTS.load(Ordering::Relaxed), 0);
    click(vcx, "restore-cancel");
    assert!(!has(vcx, "restore-now"), "left as it is");
    click(vcx, &format!("saved-{}", saved[0]));
    click(vcx, "restore-now");
    assert_eq!(
        crate::keep::load(&core.places.state).unwrap().items,
        held,
        "what the file kept is kept again"
    );
    assert_eq!(
        RESTARTS.load(Ordering::Relaxed),
        1,
        "and the app starts over it"
    );
}

/// A file that is no save of Aoi's replaces nothing, and the page says why.
#[gpui::test]
fn a_file_that_is_no_save_restores_nothing(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    std::fs::create_dir_all(&core.places.exports).unwrap();
    std::fs::write(core.places.exports.join("aoi-old.yaml"), "credits: 5\n").unwrap();
    click(vcx, "settings-button");
    click(vcx, "restore");
    click(vcx, "saved-aoi-old.yaml");
    click(vcx, "restore-now");
    assert!(has(vcx, "restore-error"), "why not is told");
    assert!(!core.places.state.exists(), "nothing kept was replaced");
}

#[gpui::test]
fn has_nothing_to_save_before_anything_is_read(cx: &mut TestAppContext) {
    let (_, _core, vcx) = open(cx, &Kept::default());
    click(vcx, "settings-button");
    click(vcx, "save");
    assert!(!has(vcx, "exported"), "nothing written");
}

#[gpui::test]
fn the_work_tab_opens_the_fold_the_game_comes_to_once(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.items = items());
    click(vcx, "tab:お仕事");
    set(&core, vcx, |s| s.place = Some(Place::Screen("学園交流会")));
    let opened = |vcx: &mut VisualTestContext, key: &str| {
        aoi.read_with(vcx, |aoi, _| aoi.opened.contains(key))
    };
    assert!(
        opened(vcx, "work:学園交流会"),
        "the fold of the screen opens"
    );
    // Once: closed by the user, it stays closed while the game stays there.
    click(vcx, "fold-work:学園交流会");
    set(&core, vcx, |s| s.status = crate::watch::Status::OtherScreen);
    assert!(!opened(vcx, "work:学園交流会"), "stays as the user left it");
    // Another screen: its fold opens, the others close.
    click(vcx, "fold-work:特別依頼");
    set(&core, vcx, |s| s.place = Some(Place::Screen("指名手配")));
    assert!(opened(vcx, "work:指名手配"), "the new screen's fold");
    assert!(!opened(vcx, "work:特別依頼"), "the others close");
}

/// A month-end shop read on 11/01 with `cards` goods.
fn month_end_shop(name: &str, cards: usize) -> crate::strategy::PlanShop {
    crate::strategy::PlanShop {
        name: name.to_owned(),
        read_at: Some("2026-11-01 12:00:00".to_owned()),
        coins: vec![crate::strategy::PlanCoin {
            name: "コイン".to_owned(),
            icon: String::new(),
            held: None,
            need: 0,
            short: false,
        }],
        cards: (0..cards)
            .map(|i| crate::strategy::PlanCard {
                id: 9000 + i64::try_from(i).expect("a few"),
                name: format!("品{i}"),
                icon: String::new(),
                coin: 0,
                left: 1,
                price: Some(50),
                amount: Some(1),
                reserved: false,
                barred: false,
                held: None,
                eleph: false,
                rarity: None,
            })
            .collect(),
    }
}

/// The work tab open in a window lower than what follows its folds (大決戦 has 30 goods), so
/// each can come to the top.
fn tall_work_tab(core: &crate::app::Core, vcx: &mut VisualTestContext) {
    vcx.simulate_resize(gpui::size(gpui::px(720.), gpui::px(300.)));
    set(core, vcx, |s| {
        s.items = items();
        s.month_end = ["総力戦", "大決戦", "合同火力演習", "指名手配", "熟達証書"]
            .into_iter()
            .map(|name| month_end_shop(name, if name == "大決戦" { 30 } else { 4 }))
            .collect();
    });
    click(vcx, "tab:お仕事");
}

/// How far a fold's title is below the top of the work tab's frame.
fn below_top(aoi: &gpui::Entity<Aoi>, vcx: &mut VisualTestContext, key: &str) -> gpui::Pixels {
    let frame = aoi.read_with(vcx, |aoi, _| aoi.content.bounds());
    let fold = super::bounds(vcx, &format!("fold:{key}")).expect("the fold shows");
    fold.top() - frame.top()
}

#[gpui::test]
fn the_work_tab_brings_the_fold_the_game_comes_to_the_top_of_its_frame(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = open(cx, &Kept::default());
    tall_work_tab(&core, vcx);
    let at_top =
        |vcx: &mut VisualTestContext, key: &str| below_top(&aoi, vcx, key).abs() < gpui::px(1.);
    set(&core, vcx, |s| s.place = Some(Place::Screen("特別依頼")));
    assert!(at_top(vcx, "work:特別依頼"), "a daily fold");
    set(&core, vcx, |s| s.place = Some(Place::Shop("大決戦")));
    assert!(at_top(vcx, "work:月末:大決戦"), "a month-end shop's fold");
}

#[gpui::test]
fn the_work_tab_keeps_its_scroll_as_the_game_comes_back_to_an_open_fold(cx: &mut TestAppContext) {
    // The user's 2026-11-08: 「既にaoiのお仕事タブ内では、その項目を開いているのであれば、スクロールも行わない」.
    let (aoi, core, vcx) = open(cx, &Kept::default());
    tall_work_tab(&core, vcx);
    for (place, key) in [
        (Place::Screen("指名手配"), "work:指名手配"),
        (Place::Shop("大決戦"), "work:月末:大決戦"),
    ] {
        set(&core, vcx, |s| s.place = Some(place));
        // The user scrolls on.
        let frame = aoi.read_with(vcx, |aoi, _| aoi.content.bounds());
        vcx.simulate_event(gpui::ScrollEvent {
            position: frame.center(),
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-40.))),
            ..Default::default()
        });
        vcx.run_until_parked();
        let scrolled = below_top(&aoi, vcx, key);
        assert!(scrolled < gpui::px(-30.), "scrolled on: {scrolled:?}");
        // A dialog in the game hides where it is, then goes.
        set(&core, vcx, |s| s.place = None);
        set(&core, vcx, |s| s.place = Some(place));
        let back = below_top(&aoi, vcx, key);
        assert!(
            (back - scrolled).abs() < gpui::px(1.),
            "{key} stays where the user scrolled it ({scrolled:?}), not {back:?}"
        );
    }
}

#[gpui::test]
fn the_work_tab_follows_nothing_when_the_settings_say_not_to(cx: &mut TestAppContext) {
    let settings = Settings {
        work_follow: false,
        ..Settings::default()
    };
    let (aoi, core, vcx) = open_with(cx, &Kept::default(), settings);
    click(vcx, "tab:お仕事");
    set(&core, vcx, |s| s.place = Some(Place::Screen("指名手配")));
    let opened = aoi.read_with(vcx, |aoi, _| aoi.opened.contains("work:指名手配"));
    assert!(!opened, "nothing opens by itself");
}
