//! Tabs stay where they are as one or another is chosen: the chosen one is told by its rule and its
//! colour, never by a bolder name, which would widen it and push the tabs after it aside. Laid out
//! with the platform's own text system, where a bolder name is wider (the tests' default draws
//! every weight alike).

use gpui::{Pixels, TestAppContext, TestDispatcher, VisualTestContext};

use super::{
    app::{equipment, items},
    bounds, click, open, set,
};
use crate::{broadcast::Cast, keep::Kept};

/// A test window laid out with the platform's text system, and what to do with it.
fn with_real_text(test: impl FnOnce(&mut TestAppContext)) {
    let mut cx = TestAppContext::build_with_text_system(
        TestDispatcher::new(0),
        None,
        // Not headless: a headless Windows platform measures with the same stand-in as the tests.
        gpui_platform::current_platform(false).text_system(),
    );
    test(&mut cx);
    cx.run_until_parked();
    cx.update(|cx| cx.quit());
    cx.run_until_parked();
}

/// The left and right edges of each of `selectors`.
fn edges(vcx: &mut VisualTestContext, selectors: &[String]) -> Vec<(Pixels, Pixels)> {
    selectors
        .iter()
        .map(|s| {
            let b = bounds(vcx, s).unwrap_or_else(|| panic!("{s} is drawn"));
            (b.left(), b.right())
        })
        .collect()
}

/// Chooses `first` then `second` (selectors) and tells whether `row` stayed put.
fn assert_still(vcx: &mut VisualTestContext, row: &[String], first: &str, second: &str) {
    click(vcx, first);
    let before = edges(vcx, row);
    click(vcx, second);
    assert_eq!(
        edges(vcx, row),
        before,
        "the row moved as {second} was chosen after {first}"
    );
    click(vcx, first);
    assert_eq!(
        edges(vcx, row),
        before,
        "the row moved as {first} was chosen again"
    );
}

#[test]
fn the_header_tabs_stay_put_as_another_is_chosen() {
    with_real_text(|cx| {
        let (_, core, vcx) = open(cx, &Kept::default());
        set(&core, vcx, |s| {
            s.items = items();
            s.equipment = equipment();
        });
        let row: Vec<String> = crate::ui::Tab::ALL
            .iter()
            .map(|t| format!("tab:{}", t.name()))
            .collect();
        assert_still(vcx, &row, "tab:アイテム", "tab:装備");
    });
}

#[test]
fn the_item_and_equipment_sub_tabs_stay_put_as_another_is_chosen() {
    with_real_text(|cx| {
        let (_, core, vcx) = open(cx, &Kept::default());
        set(&core, vcx, |s| {
            s.items = items();
            s.equipment = equipment();
        });
        click(vcx, "tab:アイテム");
        let row: Vec<String> = crate::ui::items::Sub::ALL
            .iter()
            .map(|(_, n)| format!("sub:{n}"))
            .collect();
        assert_still(vcx, &row, "sub:全体", "sub:スキル素材");
        click(vcx, "tab:装備");
        let row: Vec<String> = crate::ui::equipment::Sub::ALL
            .iter()
            .map(|(_, n)| format!("sub:{n}"))
            .collect();
        assert_still(vcx, &row, "sub:全体", "sub:設計図");
    });
}

#[test]
fn the_streaming_dialog_s_tabs_stay_put_as_another_is_chosen() {
    with_real_text(|cx| {
        let (_, core, vcx) = open(cx, &Kept::default());
        click(vcx, "menu-button");
        click(vcx, "menu-cast");
        set(&core, vcx, |s| {
            s.cast = Cast::Waiting {
                url: "rtmp://192.168.1.20:1935/live/key".to_owned(),
                moblin: vec![vec![true]],
                plain: vec![vec![false]],
                problem: None,
            };
        });
        let row = ["code:Moblin".to_owned(), "code:ほかのアプリ".to_owned()];
        assert_still(vcx, &row, "code:Moblin", "code:ほかのアプリ");
    });
}
