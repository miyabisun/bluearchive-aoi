//! The window's frame: the tabs, the status, the notices and what each list tells before it is
//! read (`e2e/app.spec.js`'s first part).

use gpui::TestAppContext;

use super::{click, has, keys, open, report, set, shows};
use crate::{
    Row,
    keep::Kept,
    quantity::Count,
    watch::{Place, Screen, Status, Update},
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
fn tables_the_skill_materials_as_the_bounties_a_row_a_school_its_bds_and_notes_either_side_of_a_slash(
    cx: &mut TestAppContext,
) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let cell = |id: i64, name: &str, held: u32| crate::work::HeldCell {
        id,
        name: name.to_owned(),
        icon: format!("item_icon_{id}"),
        held: Some(Count::Exact(held)),
        need: Some(300),
    };
    let tiers = |first: i64, name: &str, held: [u32; 4]| -> [Option<crate::work::HeldCell>; 4] {
        std::array::from_fn(|t| {
            let id = first + i64::try_from(t).expect("a tier");
            Some(cell(id, &format!("{name}{t}"), held[t]))
        })
    };
    set(&core, vcx, |s| {
        s.items = items();
        s.kinds.secret = vec![
            cell(1, "秘伝ノート", 101),
            cell(2, "秘伝ノートの断片", 2345),
        ];
        // Two sets of choice boxes, summed: 429 + 5 BD boxes, 50 + 9 note boxes at the first tier.
        let boxes = |all: bool, bd: [u32; 4], notes: [u32; 4]| crate::kinds::SkillBoxes {
            all,
            schools: vec![],
            bd: tiers(400, "BD選択ボックス", bd),
            notes: tiers(500, "ノート選択ボックス", notes),
        };
        s.kinds.skill_boxes = vec![
            boxes(true, [429, 41, 42, 43], [50, 51, 52, 53]),
            boxes(false, [5, 0, 0, 0], [9, 0, 0, 0]),
        ];
        s.kinds.schools = vec![crate::kinds::SchoolMaterials {
            name: "ゲヘナ".to_owned(),
            icon: "school_icon_gehenna".to_owned(),
            bd: tiers(200, "戦術教育BD（ゲヘナ）", [677, 21, 22, 23]),
            notes: tiers(300, "技術ノート（ゲヘナ）", [1106, 31, 32, 33]),
        }];
    });
    click(vcx, "tab:アイテム");
    click(vcx, "sub:スキル素材");
    let at = |vcx: &mut gpui::VisualTestContext, selector: &str| {
        super::bounds(vcx, selector).unwrap_or_else(|| panic!("{selector} shows"))
    };
    let school = |t: usize| format!("cell:skills-school_icon_gehenna-{t}");
    let boxes = |t: usize| format!("cell:skills-boxes-{t}");
    // The user's 2026-11-09 「アイテムのスキル素材も同じ形にしてください」: as the bounties' table, a
    // cell split at its middle, the one good's count at the right of the left half, the other's
    // at the right of the right half; the secret notes so in the top tier's column, the summed
    // choice boxes on one row.
    let near = |a: gpui::Pixels, b: gpui::Pixels| (a - b).abs() < gpui::px(1.);
    let mut ends = Vec::new();
    for (cell, left, right) in [
        (school(0), "677", "1,106"),
        (boxes(0), "434", "59"),
        ("cell:skills-secret-3".to_owned(), "101", "2,345"),
    ] {
        let (bounds, left, right) = (
            at(vcx, &cell),
            at(vcx, &format!("text:{left}")),
            at(vcx, &format!("text:{right}")),
        );
        let middle = bounds.center().x;
        assert!(
            bounds.left() < left.left()
                && left.right() < middle
                && middle < right.left()
                && right.right() < bounds.right(),
            "{cell}: {left:?} left of the middle, {right:?} right, in {bounds:?}"
        );
        assert!(
            bounds.size.height < gpui::px(40.),
            "{cell} lower than an icon: {:?}",
            bounds.size.height
        );
        ends.push((cell, middle - left.right(), bounds.right() - right.right()));
    }
    // However many digits, each count ends alike short of its half's right edge.
    let (_, half, end) = ends[0].clone();
    for (cell, left, right) in &ends {
        assert!(
            near(*left, half) && near(*right, end),
            "{cell}: counts end {left:?} and {right:?} short, the school's {half:?} and {end:?}"
        );
    }
    // The rows' four tiers in the same columns, the secret notes in the top tier's.
    for t in 0..4 {
        let (school, boxes) = (at(vcx, &school(t)), at(vcx, &boxes(t)));
        assert!(
            near(school.left(), boxes.left()) && near(school.right(), boxes.right()),
            "tier {t}: {school:?} under {boxes:?}"
        );
    }
    let (secret, top) = (at(vcx, "cell:skills-secret-3"), at(vcx, &school(3)));
    assert!(
        near(secret.left(), top.left()) && near(secret.right(), top.right()),
        "the secret notes {secret:?} over the school's top tier {top:?}"
    );
}

#[gpui::test]
fn tables_the_ooparts_as_the_bounties_counts_alone_their_top_tier_at_the_left(
    cx: &mut TestAppContext,
) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let tiers = |first: i64, name: &str, held: [u32; 4]| -> [Option<crate::work::HeldCell>; 4] {
        std::array::from_fn(|t| {
            let id = first + i64::try_from(t).expect("a tier");
            Some(crate::work::HeldCell {
                id,
                name: format!("{name}{t}"),
                icon: format!("item_icon_{id}"),
                held: Some(Count::Exact(held[t])),
                need: Some(300),
            })
        })
    };
    set(&core, vcx, |s| {
        s.items = items();
        s.kinds.oopart_boxes = tiers(600, "オーパーツ選択ボックス", [70, 80, 90, 1234]);
        s.kinds.ooparts = vec![
            tiers(100, "円盤", [11, 12, 13, 14]),
            tiers(110, "ネックレス", [2048, 33, 44, 55]),
        ];
    });
    click(vcx, "tab:アイテム");
    click(vcx, "sub:オーパーツ");
    let at = |vcx: &mut gpui::VisualTestContext, selector: &str| {
        super::bounds(vcx, selector).unwrap_or_else(|| panic!("{selector} shows"))
    };
    let oopart = |k: usize, t: usize| format!("cell:ooparts-oopart{k}-{t}");
    let boxes = |t: usize| format!("cell:ooparts-boxes-{t}");
    let near = |a: gpui::Pixels, b: gpui::Pixels| (a - b).abs() < gpui::px(1.);
    // The user's 2026-11-09 「オーパーツのタブも同じ形にしてください」: as the bounties' oopart rows,
    // the counts alone at their cells' right, however many digits, the rows low.
    let mut ends = Vec::new();
    for (cell, count) in [
        (oopart(0, 0), "11"),
        (oopart(1, 0), "2,048"),
        (boxes(0), "70"),
        (boxes(3), "1,234"),
    ] {
        let (bounds, count) = (at(vcx, &cell), at(vcx, &format!("text:{count}")));
        assert!(
            bounds.center().x < count.left() && count.right() < bounds.right(),
            "{cell}: {count:?} at the right of {bounds:?}"
        );
        assert!(
            bounds.size.height < gpui::px(40.),
            "{cell} lower than an icon: {:?}",
            bounds.size.height
        );
        ends.push((cell, bounds.right() - count.right()));
    }
    let (_, end) = ends[0].clone();
    for (cell, from_right) in &ends {
        assert!(
            near(*from_right, end),
            "{cell}: its count ends {from_right:?} short, the first's {end:?}"
        );
    }
    // The rows' four tiers in the same columns.
    for t in 0..4 {
        let first = at(vcx, &boxes(t));
        for k in 0..2 {
            let row = at(vcx, &oopart(k, t));
            assert!(
                near(first.left(), row.left()) && near(first.right(), row.right()),
                "tier {t}: oopart {k} {row:?} under the boxes' {first:?}"
            );
        }
    }
    // At each row's left, the picture of its top tier (its name on hover); the boxes' caption.
    for (picture, cell) in [
        ("picture:円盤3", oopart(0, 0)),
        ("picture:ネックレス3", oopart(1, 0)),
        ("picture:選択ボックス", boxes(0)),
    ] {
        let (picture, row) = (at(vcx, picture), at(vcx, &cell));
        assert!(
            picture.right() <= row.left()
                && row.top() <= picture.center().y
                && picture.center().y <= row.bottom(),
            "{picture:?} at the left of {row:?}"
        );
    }
    // The boxes apart from the ooparts under them (the user's of 2026-10-29).
    let (top, first, second) = (
        at(vcx, &boxes(0)),
        at(vcx, &oopart(0, 0)),
        at(vcx, &oopart(1, 0)),
    );
    assert!(
        first.top() - top.bottom() > second.top() - first.bottom(),
        "a wider gap under the boxes {top:?} than between the ooparts {first:?} {second:?}"
    );
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
fn the_status_names_a_screen_the_work_tab_follows(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let place = |name| Some(Update::Place(name));
    report(&core, vcx, &Status::OtherScreen, None);
    assert!(shows(vcx, "対象外の画面"), "no screen told");
    report(
        &core,
        vcx,
        &Status::OtherScreen,
        place(Some(Place::Screen("指名手配"))),
    );
    assert!(shows(vcx, "指名手配"), "the screen the game came to");
    assert!(
        !shows(vcx, "対象外の画面"),
        "not told as no screen of Aoi's"
    );
    report(&core, vcx, &Status::OtherScreen, place(None));
    assert!(shows(vcx, "対象外の画面"), "left: no screen told again");
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
fn puts_the_tabs_the_status_and_the_settings_on_one_row(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.status = reading(Screen::Shops, 20.));
    let middle = |vcx: &mut gpui::VisualTestContext, selector: &str| {
        super::bounds(vcx, selector)
            .unwrap_or_else(|| panic!("{selector}"))
            .center()
            .y
    };
    let row = middle(vcx, "tab:アイテム");
    for other in ["tab:お仕事", "status", "settings-button"] {
        let y = middle(vcx, other);
        assert!((y - row).abs() < gpui::px(4.), "{other} on the tabs' row");
    }
    assert!(!shows(vcx, "Aoi"), "the title bar names the app");
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

/// The left and right edges of what `selector` finds.
fn sides(vcx: &mut gpui::VisualTestContext, selector: &str) -> (gpui::Pixels, gpui::Pixels) {
    let at = super::bounds(vcx, selector).unwrap_or_else(|| panic!("{selector} shows"));
    (at.left(), at.right())
}

#[gpui::test]
fn fills_a_window_made_wider_side_to_side(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.items = items();
        s.shops = super::shops::shops();
    });
    vcx.simulate_resize(gpui::size(gpui::px(1200.), gpui::px(820.)));
    vcx.run_until_parked();
    // The content's sides are 12 px in from the window's: 1176 px hold 16 tiles of 64 px and more.
    assert_eq!(
        sides(vcx, "tile-1000").0,
        gpui::px(12.),
        "the first tile at the left side"
    );
    assert_eq!(
        sides(vcx, "tile-1015").1,
        gpui::px(1188.),
        "the row's last tile ends at the right side"
    );
    assert_eq!(
        sides(vcx, "tile-1016").0,
        gpui::px(12.),
        "the next begins a row: none stands past the side"
    );
    click(vcx, "tab:ショップ");
    assert_eq!(
        sides(vcx, "fold-shop-総力戦"),
        (gpui::px(12.), gpui::px(1188.)),
        "a shop from side to side"
    );
    click(vcx, "tab:お仕事");
    assert_eq!(
        sides(vcx, "fold:work:任務 Hard"),
        (gpui::px(12.), gpui::px(1188.)),
        "a work from side to side"
    );
}
