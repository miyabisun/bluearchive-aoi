//! The work tab: its folds, and the hard missions' elephs reserved by pressing their cards.

use gpui::TestAppContext;

use super::{app::items, click, has, open, set, shows};
use crate::{keep::Kept, quantity::Count, strategy::Reserve, work::HardCard};

fn hoshino_card(reserved: bool) -> HardCard {
    HardCard {
        id: 10005,
        name: "ホシノの神名文字".to_owned(),
        icon: "item_icon_secretstone_hoshino".to_owned(),
        stages: vec!["任務 Hard 12-3".to_owned()],
        ap: 20,
        star: Some(5),
        held: Some(Count::Exact(148)),
        done: false,
        reserved,
    }
}

#[gpui::test]
fn tells_the_plays_by_when_they_are_played(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.items = items());
    click(vcx, "tab:お仕事");
    for heading in [
        "日課",
        "2倍キャンペーン",
        "月末",
        "ショップ: 通常アイテム",
        "指名手配",
        "学園交流会",
        "任務 Normal",
        "任務 Hard",
        "特別依頼",
    ] {
        assert!(shows(vcx, heading), "{heading}");
    }
    click(vcx, "fold-work:特別依頼");
    for title in ["拠点防衛", "クレジット回収"] {
        assert!(shows(vcx, title), "the commissions' row {title}");
    }
}

#[gpui::test]
fn reserves_a_hard_mission_s_eleph_by_pressing_its_card(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.hard = vec![hoshino_card(false)]);
    click(vcx, "tab:お仕事");
    click(vcx, "fold-work:任務 Hard");
    assert!(has(vcx, "card:hard-10005"), "the student's eleph a card");
    assert!(shows(vcx, "148文字"), "the elephs held");
    click(vcx, "card:hard-10005");
    let reserved = Reserve {
        shop: "任務 Hard".to_owned(),
        id: 10005,
        coin: "AP".to_owned(),
    };
    assert!(
        core.snapshot().reserved.contains(&reserved),
        "pressed, reserved"
    );
    // The app counts the cards again; drawn reserved, the day's AP shows red in the heading.
    set(&core, vcx, |s| s.hard = vec![hoshino_card(true)]);
    assert!(shows(vcx, "予約中"), "marked reserved");
    assert!(shows(vcx, "-20"), "the day's AP it takes, in the heading");
    click(vcx, "card:hard-10005");
    assert!(
        !core.snapshot().reserved.contains(&reserved),
        "again, taken back"
    );
}

#[gpui::test]
fn tables_a_bounty_s_school_on_a_row_its_bds_and_notes_either_side_of_a_slash(
    cx: &mut TestAppContext,
) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let cell = |id: i64, name: String, held: u32| crate::work::HeldCell {
        id,
        name,
        icon: format!("item_icon_{id}"),
        held: Some(Count::Exact(held)),
        need: Some(300),
    };
    let tier = ["初級", "中級", "上級", "最上級"];
    let tiers = |first: i64, name: &str, held: [u32; 4]| -> [Option<crate::work::HeldCell>; 4] {
        std::array::from_fn(|t| {
            let id = first + i64::try_from(t).expect("a tier");
            Some(cell(id, format!("{}{name}", tier[t]), held[t]))
        })
    };
    set(&core, vcx, |s| {
        s.items = items();
        s.work.bounties = vec![crate::work::BountyRow {
            name: "指名手配：ハイウェイ".to_owned(),
            ooparts: vec![tiers(100, "円盤", [11, 12, 13, 14])],
            schools: vec![crate::kinds::SchoolMaterials {
                name: "ゲヘナ".to_owned(),
                icon: "school_icon_gehenna".to_owned(),
                bd: tiers(200, "戦術教育BD（ゲヘナ）", [677, 21, 22, 23]),
                notes: tiers(300, "技術ノート（ゲヘナ）", [1106, 31, 32, 33]),
            }],
        }];
    });
    click(vcx, "tab:お仕事");
    click(vcx, "fold-work:指名手配");
    let at = |vcx: &mut gpui::VisualTestContext, selector: &str| {
        super::bounds(vcx, selector).unwrap_or_else(|| panic!("{selector} shows"))
    };
    let school = |t: usize| format!("cell:bounty-指名手配：ハイウェイ-school_icon_gehenna-{t}");
    let oopart = |t: usize| format!("cell:bounty-指名手配：ハイウェイ-oopart0-{t}");
    // The cell split in two at its middle, the slash there: the BD's count at the right of the
    // left half, the note's at the right of the right half, however many digits (the user's
    // 2026-11-08 「アイテムは桁数が重要なので、基本的には右揃え」「セルを2つに割るイメージで、
    // それぞれを右揃え」).
    let first = at(vcx, &school(0));
    let (bd, note) = (at(vcx, "text:677"), at(vcx, "text:1,106"));
    let middle = first.center().x;
    assert!(
        first.left() < bd.left()
            && bd.right() < middle
            && middle < note.left()
            && note.right() < first.right(),
        "the BD's count left of the middle, the note's right: {bd:?} {note:?} in {first:?}"
    );
    let second = at(vcx, &school(1));
    let (short_bd, short_note) = (at(vcx, "text:21"), at(vcx, "text:31"));
    let near = |a: gpui::Pixels, b: gpui::Pixels| (a - b).abs() < gpui::px(1.);
    assert!(
        near(middle - bd.right(), second.center().x - short_bd.right()),
        "677 and 21 end alike short of the middle: {bd:?} {short_bd:?}"
    );
    let from_right = first.right() - note.right();
    assert!(
        near(from_right, second.right() - short_note.right()),
        "1,106 and 31 end alike short of the right: {note:?} {short_note:?}"
    );
    // An oopart's count, alone in its cell, ends there too.
    let oopart_count = at(vcx, "text:11");
    assert!(
        near(
            from_right,
            at(vcx, &oopart(0)).right() - oopart_count.right()
        ),
        "the oopart's count at the right: {oopart_count:?}"
    );
    // The four tiers in the oopart row's columns.
    for t in 0..4 {
        let (school, oopart) = (at(vcx, &school(t)), at(vcx, &oopart(t)));
        assert!(
            (school.left() - oopart.left()).abs() < gpui::px(1.)
                && (school.right() - oopart.right()).abs() < gpui::px(1.),
            "tier {t}: {school:?} under {oopart:?}"
        );
    }
    assert!(
        first.size.height < gpui::px(40.),
        "lower than an icon: {:?}",
        first.size.height
    );
}
