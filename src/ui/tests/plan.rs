//! The training tab: the targets typed and chosen, kept through the frames that keep coming,
//! and making up what is short by hand.

use gpui::{Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext};

use super::{
    app::{items, reading},
    bounds, click, has, keys, open, report, set, shows,
    students::trained,
};
use crate::{
    keep::Kept,
    plan::{Held, Kind, Target},
    quantity::Count,
    supply::{ProcureLine, Source},
    watch::{Screen, Status, Update},
};

/// アカネ（バニーガール） at ★3 and Lv80, marked for training at the default target.
fn marked(
    cx: &mut TestAppContext,
) -> (
    gpui::Entity<crate::ui::Aoi>,
    std::sync::Arc<crate::app::Core>,
    &mut gpui::VisualTestContext,
) {
    let (aoi, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.status = reading(Screen::Items, 20.);
        s.items = items();
        s.students = vec![trained(20019, "アカネ（バニーガール）", |t| {
            t.level = Some(80);
            t.rarity = Some(3);
            t.weapon = Some(None);
        })];
    });
    core.set_target(20019, Some(Target::default()))
        .expect("the target is kept");
    vcx.run_until_parked();
    click(vcx, "tab:育成");
    (aoi, core, vcx)
}

fn target(core: &crate::app::Core) -> Target {
    core.snapshot()
        .targets
        .iter()
        .find(|t| t.id == 20019)
        .expect("marked")
        .target
}

#[gpui::test]
fn guides_to_the_student_tab_before_any_student_is_marked(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.status = Status::OtherScreen);
    click(vcx, "tab:育成");
    assert!(shows(vcx, crate::ui::plan::EMPTY), "how to mark students");
}

#[gpui::test]
fn a_target_being_typed_is_kept_while_the_game_is_read(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = marked(cx);
    click(vcx, "target-20019-0");
    keys(vcx, "ctrl-a 8");
    // Every frame read sends a new snapshot; what is typed stays.
    for fps in [9., 8.] {
        report(&core, vcx, &reading(Screen::Items, fps), None);
    }
    let typed = aoi.read_with(vcx, |aoi, cx| {
        aoi.target_fields[&(20019, 0)].read(cx).text().to_owned()
    });
    assert_eq!(typed, "8", "the typed value stays");
    assert_eq!(target(&core).level, 90, "not set until Enter");
    keys(vcx, "enter");
    assert_eq!(target(&core).level, 8, "Enter sets it");
}

#[gpui::test]
fn a_star_is_chosen_from_the_student_s_on_and_kept_while_the_game_is_read(cx: &mut TestAppContext) {
    let (_, core, vcx) = marked(cx);
    // Never chosen, the target stays at the student's star.
    assert!(has(vcx, "stars:3"), "★3 drawn");
    click(vcx, "star-pick-20019");
    assert!(
        has(vcx, "star-1") && has(vcx, "star-8"),
        "every star offered"
    );
    for fps in [9., 8., 7.] {
        report(&core, vcx, &reading(Screen::Items, fps), None);
    }
    assert!(
        has(vcx, "star-6"),
        "open through the frames that keep coming"
    );
    // None below the student's now.
    click(vcx, "star-2");
    assert_eq!(
        target(&core).star,
        None,
        "★2 is below the student's ★3: not chosen"
    );
    click(vcx, "star-6");
    assert_eq!(target(&core).star, Some(6), "固有2 chosen");
    assert!(!has(vcx, "star-6"), "the menu closes");
    assert!(has(vcx, "stars:6"), "the target drawn as 固有2");
}

/// A line short with its sources: boxes held, a shop's coin, an unread coin.
fn procure() -> Vec<ProcureLine> {
    vec![ProcureLine {
        kind: Kind::Item,
        id: 4033,
        name: "最上級技術ノート（ゲヘナ）".to_owned(),
        icon: "item_icon_skillbook_gehenna_3".to_owned(),
        short: 29,
        sources: vec![
            Source::Box {
                id: 150_003,
                name: "最上級技術ノート選択ボックス".to_owned(),
                icon: "item_icon_skillbook_selection_3".to_owned(),
                held: 4,
                choices: 12,
            },
            Source::Shop {
                shop: "総力戦".to_owned(),
                coin: "総力戦コイン".to_owned(),
                icon: "item_icon_raidcoin".to_owned(),
                held: Some(5940),
                price: 25,
                gives: 1,
                left: 20,
            },
            Source::Shop {
                shop: "指名手配".to_owned(),
                coin: "指名手配コイン".to_owned(),
                icon: "item_icon_chasercoin".to_owned(),
                held: None,
                price: 200,
                gives: 1,
                left: 4,
            },
        ],
    }]
}

fn press(vcx: &mut gpui::VisualTestContext, selector: &str, button: MouseButton, shift: bool) {
    let position = bounds(vcx, selector).expect(selector).center();
    let modifiers = Modifiers {
        shift,
        ..Modifiers::none()
    };
    vcx.simulate_event(MouseDownEvent {
        button,
        position,
        modifiers,
        click_count: 1,
        first_mouse: false,
    });
    vcx.simulate_event(MouseUpEvent {
        button,
        position,
        modifiers,
        click_count: 1,
    });
    vcx.run_until_parked();
}

#[gpui::test]
fn makes_up_what_is_short_by_hand_and_settles_it(cx: &mut TestAppContext) {
    let (_, core, vcx) = marked(cx);
    set(&core, vcx, |s| s.procure = procure());
    let chip = "chip-item-4033-shop:総力戦:総力戦コイン";
    assert!(shows(vcx, "29"), "short 29");
    assert!(
        shows(vcx, "5,940"),
        "the coin held, before anything is paid"
    );
    press(vcx, chip, MouseButton::Left, false);
    assert!(shows(vcx, "+1"), "a click pays one");
    assert!(shows(vcx, "5,915"), "the coin left after it");
    assert!(shows(vcx, "精算"), "something paid: the settlement");
    press(vcx, chip, MouseButton::Right, false);
    assert!(!shows(vcx, "+1"), "a right click takes it back");
    press(vcx, chip, MouseButton::Left, true);
    assert!(
        shows(vcx, "+20"),
        "with Shift, as many as the shop has left"
    );
    // Every frame read leaves what was paid.
    report(&core, vcx, &reading(Screen::Items, 9.), None);
    assert!(shows(vcx, "+20"), "kept through the frames");
    click(vcx, "reset");
    assert!(!shows(vcx, "+20"), "リセット takes every payment back");
    assert!(!shows(vcx, "精算"), "nothing paid, no settlement");
}

/// What the plan holds of its line `kind` (the first of them).
fn held(core: &crate::app::Core, kind: Kind) -> Option<Held> {
    let plan = core.snapshot().plan;
    plan.iter().find(|r| r.kind == kind).expect("a line").have
}

#[gpui::test]
fn what_is_held_is_counted_as_read_now_without_a_word(cx: &mut TestAppContext) {
    let (_, core, vcx) = marked(cx);
    let reading = reading(Screen::Items, 9.);
    let told = |core: &crate::app::Core| core.snapshot().plan_behind;
    assert_eq!(told(&core), None, "nothing read since");
    // The credits, read on nearly every screen.
    report(&core, vcx, &reading, Some(Update::Credits(123_456_789)));
    assert_eq!(held(&core, Kind::Credits), Some(Held::Exact(123_456_789)));
    // Items the plan takes: seven of an activity report.
    let (id, exp) = core.recognizer().planner.growth.reports[0];
    let mut rows = items();
    rows[0].id = id;
    rows[0].count = Count::Exact(7);
    report(
        &core,
        vcx,
        &reading,
        Some(Update::List(crate::List::Items, rows)),
    );
    assert_eq!(held(&core, Kind::ReportExp), Some(Held::Exact(7 * exp)));
    // The shops, as they are read.
    let shops = super::shops::shops();
    report(&core, vcx, &reading, Some(Update::Shops(shops)));
    assert_eq!(told(&core), None, "what is held is not told");
}

#[gpui::test]
fn a_marked_student_trained_since_is_told_until_the_plan_is_reloaded(cx: &mut TestAppContext) {
    let (_, core, vcx) = marked(cx);
    // The plan's ground taken with her in it (a student it lacks is counted as read now).
    click(vcx, "reload");
    let need = |core: &crate::app::Core| core.snapshot().plan[0].need;
    let before = need(&core);
    let leveled = |level| {
        vec![trained(20019, "アカネ（バニーガール）", |t| {
            t.level = Some(level);
            t.rarity = Some(3);
            t.weapon = Some(None);
        })]
    };
    report(
        &core,
        vcx,
        &reading(Screen::Items, 9.),
        Some(Update::Students(leveled(85))),
    );
    let word = "アカネ（バニーガール）の育成状況が変わっています。再読込で数え直します（払った分は取り消されます）";
    assert!(shows(vcx, word), "trained since: told by name");
    assert_eq!(need(&core), before, "the plan counts her as she was");
    click(vcx, "reload");
    assert!(!shows(vcx, word), "counted again: nothing to tell");
    assert!(need(&core) < before, "from Lv85 now");
}

#[gpui::test]
fn what_was_paid_stays_while_what_is_held_is_read_again(cx: &mut TestAppContext) {
    let (aoi, core, vcx) = marked(cx);
    set(&core, vcx, |s| s.procure = procure());
    press(
        vcx,
        "chip-item-4033-shop:総力戦:総力戦コイン",
        MouseButton::Left,
        false,
    );
    assert!(shows(vcx, "+1"), "one paid");
    // The credits read otherwise count the plan again: its lines are the snapshot's own.
    report(
        &core,
        vcx,
        &reading(Screen::Items, 9.),
        Some(Update::Credits(5)),
    );
    let paid = aoi.read_with(vcx, |aoi, _| aoi.payments.len());
    assert_eq!(paid, 1, "the payment is kept");
    // The line short again, what was paid for it counts again.
    set(&core, vcx, |s| s.procure = procure());
    assert!(shows(vcx, "+1"), "paid still");
}

#[gpui::test]
fn names_the_bounties_to_play_for_what_falls_short(cx: &mut TestAppContext) {
    let (_, core, vcx) = marked(cx);
    let cell = |id: i64| crate::work::HeldCell {
        id,
        name: format!("item {id}"),
        icon: format!("icon_{id}"),
        held: None,
        need: None,
    };
    set(&core, vcx, |s| {
        s.plan = vec![crate::plan::NeedRow {
            kind: Kind::Item,
            id: 4033,
            name: "最上級技術ノート（ゲヘナ）".to_owned(),
            icon: "item_icon_skillbook_gehenna_3".to_owned(),
            family: None,
            need: 30,
            have: Some(crate::plan::Held::Exact(12)),
        }];
        let school = || crate::kinds::SchoolMaterials {
            name: "ゲヘナ".to_owned(),
            icon: "school_icon_gehenna".to_owned(),
            bd: [None, None, None, None],
            notes: [None, None, None, Some(cell(4033))],
        };
        s.work.bounties = vec![
            crate::work::BountyRow {
                name: "指名手配：ハイウェイ".to_owned(),
                ooparts: Vec::new(),
                schools: vec![school()],
            },
            crate::work::BountyRow {
                name: "指名手配：砂漠の鉄道".to_owned(),
                ooparts: Vec::new(),
                schools: vec![school()],
            },
        ];
    });
    assert!(shows(vcx, "日々の行動"), "the section");
    assert!(shows(vcx, "18"), "how many are short");
    assert!(
        shows(vcx, "ハイウェイ・砂漠の鉄道"),
        "the bounties that drop it"
    );
}
