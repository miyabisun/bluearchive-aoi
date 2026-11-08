//! The window's frame: the tabs, the status, the notices and what each list tells before it is
//! read (`e2e/app.spec.js`'s first part).

use gpui::TestAppContext;

use super::{click, has, keys, open, report, set, shows};
use crate::{
    Row,
    keep::Kept,
    quantity::Count,
    watch::{Screen, Status},
};

/// 120 items, the fourth with a long name, the sixth counted in thousands.
pub fn items() -> Vec<Row> {
    (0..120)
        .map(|i| Row {
            id: 1000 + i,
            name: if i == 3 {
                "とても長い名前の期間限定でない普通のアイテム（ゲヘナ・トリニティ・ミレニアム共通）"
                    .to_owned()
            } else {
                format!("アイテム{i}")
            },
            icon: format!("item_icon_skillbook_gehenna_{}", i % 4),
            tier: None,
            count: if i == 5 {
                Count::Thousands(51)
            } else {
                Count::Exact(u32::try_from(i).unwrap() * 997)
            },
            rarity: None,
            eleph: false,
        })
        .collect()
}

pub fn equipment() -> Vec<Row> {
    let row = |id, name: &str, icon: &str, tier, count| Row {
        id,
        name: name.to_owned(),
        icon: icon.to_owned(),
        tier,
        count,
        rarity: None,
        eleph: false,
    };
    vec![
        row(
            1,
            "初級強化珠",
            "equipment_icon_exp_0",
            None,
            Count::Exact(1383),
        ),
        row(
            1000,
            "無地のキャップ",
            "equipment_icon_hat_tier1",
            Some(1),
            Count::Exact(249),
        ),
        row(
            101_009,
            "ゲーミングヘルメットの設計図",
            "equipment_icon_hat_tier10_piece",
            Some(10),
            Count::Exact(17),
        ),
        row(
            501_000,
            "帽子の万能設計図",
            "equipment_icon_hat_useall_piece",
            None,
            Count::Thousands(51),
        ),
    ]
}

pub fn reading(list: Screen, fps: f32) -> Status {
    Status::Reading {
        list,
        fps,
        dropped: 0,
    }
}

#[gpui::test]
fn lists_the_item_tally_in_order_and_narrows_it_by_name(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = reading(Screen::Items, 27.4);
        s.items = items();
        s.equipment = equipment();
    });
    assert!(
        shows(vcx, "アイテムを集計中"),
        "the status says what is read"
    );
    assert!(has(vcx, "tile-1002"), "every item a tile");
    assert!(shows(vcx, "1,994"), "counts with separators");
    assert!(shows(vcx, "51K"), "thousands as K");
    // Only the rows in sight are drawn; scrolled down, the last comes.
    assert!(
        !has(vcx, "tile-1119"),
        "the last row out of sight is not drawn"
    );
    let at = super::bounds(vcx, "tile-1002").expect("a tile").center();
    for _ in 0..10 {
        vcx.simulate_event(gpui::ScrollEvent {
            position: at,
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-800.))),
            ..Default::default()
        });
    }
    vcx.run_until_parked();
    assert!(has(vcx, "tile-1119"), "down to the last");

    click(vcx, "search");
    keys(vcx, "ｱ ｲ ﾃ ﾑ 1 1");
    assert!(has(vcx, "tile-1011"), "アイテム11");
    assert!(has(vcx, "tile-1110"), "アイテム110");
    assert!(!has(vcx, "tile-1012"), "the others go");

    keys(
        vcx,
        "backspace backspace backspace backspace backspace backspace",
    );
    keys(vcx, "z z z");
    assert!(
        shows(vcx, "一致する項目がありません"),
        "a search finding nothing says so"
    );
}

#[gpui::test]
fn lines_the_secret_notes_up_with_the_four_tiers_under_them(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let cell = |id: i64, held: u32| crate::work::HeldCell {
        id,
        name: format!("品{id}"),
        icon: String::new(),
        held: Some(Count::Exact(held)),
        need: None,
    };
    set(&core, vcx, |s| {
        s.items = items();
        // Two secret notes, the last two columns blank.
        s.kinds.secret = vec![cell(1, 101), cell(2, 102)];
        s.kinds.schools = vec![crate::kinds::SchoolMaterials {
            name: "ゲヘナ".to_owned(),
            icon: "school_icon_gehenna".to_owned(),
            bd: [0, 1, 2, 3].map(|t| Some(cell(10 + i64::from(t), 201 + t))),
            notes: [0, 1, 2, 3].map(|t| Some(cell(20 + i64::from(t), 301 + t))),
        }];
    });
    click(vcx, "tab:アイテム");
    click(vcx, "sub:スキル素材");
    // A cell's count ends at its right edge, inside the padding every cell has.
    let end = |vcx: &mut gpui::VisualTestContext, held: u32| {
        super::bounds(vcx, &format!("text:{held}"))
            .unwrap_or_else(|| panic!("{held} shows"))
            .right()
    };
    for (secret, school) in [(101, 201), (102, 202)] {
        let (secret, school) = (end(vcx, secret), end(vcx, school));
        assert!(
            (secret - school).abs() < gpui::px(1.),
            "a secret note's column ends at {secret:?}, the school's under it at {school:?}"
        );
    }
}

#[gpui::test]
fn the_status_pauses_the_reading_when_pressed_and_resumes_it(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = reading(Screen::Items, 27.4);
        s.items = items();
    });
    assert!(shows(vcx, "アイテムを集計中"), "reading");
    report(&core, vcx, &Status::Paused, None);
    assert!(shows(vcx, "一時中断"), "paused, it says so");
}

#[gpui::test]
fn guides_to_the_list_before_anything_is_read(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = Status::OtherScreen;
        s.equipment = equipment();
    });
    assert!(shows(vcx, "対象外の画面"), "the status");
    assert!(shows(vcx, crate::ui::items::EMPTY), "how to read the items");
    assert!(!has(vcx, "search"), "no search before anything is read");
    click(vcx, "tab:装備");
    assert!(
        has(vcx, "tile-101009"),
        "the equipment read shows on its tab"
    );
}

#[gpui::test]
fn shows_why_the_capture_failed(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = Status::Failed {
            message: "アクセスが拒否されました。".to_owned(),
        };
        s.items = items();
    });
    assert!(shows(vcx, "キャプチャ失敗"), "the status");
    assert!(
        shows(vcx, "アクセスが拒否されました。"),
        "and why, in the page"
    );
}

#[gpui::test]
fn puts_the_tabs_the_status_and_the_menu_on_one_row(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.status = reading(Screen::Shops, 20.));
    let middle = |vcx: &mut gpui::VisualTestContext, selector: &str| {
        super::bounds(vcx, selector)
            .unwrap_or_else(|| panic!("{selector}"))
            .center()
            .y
    };
    let row = middle(vcx, "tab:アイテム");
    for other in ["tab:お仕事", "status", "menu-button"] {
        let y = middle(vcx, other);
        assert!((y - row).abs() < gpui::px(4.), "{other} on the tabs' row");
    }
    assert!(!shows(vcx, "Aoi"), "the title bar names the app");
}

#[gpui::test]
fn writes_everything_read_to_a_file_and_tells_where(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.items = items());
    click(vcx, "menu-button");
    click(vcx, "menu-export");
    let written = std::fs::read_dir(&core.places.exports)
        .expect("the export folder")
        .count();
    assert_eq!(written, 1, "one file written");
    assert!(has(vcx, "exported"), "and where it is told");
}

#[gpui::test]
fn has_nothing_to_export_before_anything_is_read(cx: &mut TestAppContext) {
    let (_, _core, vcx) = open(cx, &Kept::default());
    click(vcx, "menu-button");
    click(vcx, "menu-export");
    assert!(!has(vcx, "exported"), "nothing written");
}

#[gpui::test]
fn tells_why_what_was_read_could_not_be_kept(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.items = items();
        s.keep_error =
            Some("読み取り結果を保存できませんでした: ディスクがいっぱいです".to_owned());
    });
    assert!(
        shows(
            vcx,
            "読み取り結果を保存できませんでした: ディスクがいっぱいです"
        ),
        "the failure stays in the page"
    );
}

#[gpui::test]
fn keeps_the_tab_chosen_when_the_game_shows_another_list(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.items = items();
        s.equipment = equipment();
    });
    click(vcx, "tab:装備");
    report(&core, vcx, &reading(Screen::Items, 20.), None);
    let tab = aoi.read_with(vcx, |aoi, _| aoi.tab);
    assert_eq!(
        tab,
        crate::ui::Tab::Equipment,
        "the tab stays as the user chose"
    );
}
