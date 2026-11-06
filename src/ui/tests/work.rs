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
    let rows = crate::view::commission_rows(&core.snapshot().work.commissions);
    for row in &rows {
        assert!(shows(vcx, &row.title), "the commissions' row {}", row.title);
    }
}

#[gpui::test]
fn reserves_a_hard_mission_s_eleph_by_pressing_its_card(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.hard = vec![hoshino_card(false)]);
    click(vcx, "tab:お仕事");
    click(vcx, "fold-work:任務 Hard");
    assert!(has(vcx, "card:hard-10005"), "the student's eleph a card");
    assert!(
        shows(vcx, "148文字") || shows(vcx, "148"),
        "the elephs held"
    );
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
