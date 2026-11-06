//! The student tab: the students read, their stars, the order by column, the count on the tab.

use gpui::TestAppContext;

use super::{
    app::{equipment, items, reading},
    click, has, keys, open, set, shows,
};
use crate::{
    Row,
    keep::Kept,
    quantity::Count,
    student::{Gear, Training, Weapon},
    watch::{Screen, StudentRow},
};

fn hoshino() -> StudentRow {
    StudentRow {
        id: 10005,
        name: "ホシノ".to_owned(),
        training: Training {
            level: Some(90),
            rarity: Some(5),
            bond: Some(34),
            skills: [Some(5), Some(10), Some(10), Some(10)],
            weapon: Some(Some(Weapon { level: 60, stars: 4 })),
            equipment: [Some(Gear { tier: 10 }); 3],
            gear: Some(2),
            ability: [Some(25), Some(0), Some(25)],
        },
    }
}

fn akane() -> StudentRow {
    StudentRow {
        id: 20019,
        name: "アカネ（バニーガール）".to_owned(),
        training: Training {
            level: None,
            rarity: Some(3),
            bond: Some(20),
            skills: [Some(2), Some(3), Some(3), Some(6)],
            weapon: Some(None),
            equipment: [Some(Gear { tier: 4 }), None, Some(Gear { tier: 3 })],
            gear: Some(0),
            ability: [Some(0), Some(0), Some(0)],
        },
    }
}

/// A student at Lv90 ★5, skills and equipment full, unless `change` says otherwise.
pub fn trained(id: i64, name: &str, change: impl FnOnce(&mut Training)) -> StudentRow {
    let mut training = Training {
        level: Some(90),
        rarity: Some(5),
        bond: Some(20),
        skills: [Some(5), Some(10), Some(10), Some(10)],
        weapon: Some(Some(Weapon { level: 1, stars: 0 })),
        equipment: [Some(Gear { tier: 10 }); 3],
        gear: Some(0),
        ability: [Some(0); 3],
    };
    change(&mut training);
    StudentRow {
        id,
        name: name.to_owned(),
        training,
    }
}

#[gpui::test]
fn shows_the_students_read_from_the_student_screen(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    let eleph = Row {
        id: 10005,
        name: "ホシノの神名文字".to_owned(),
        icon: "item_icon_secretstone_hoshino".to_owned(),
        tier: None,
        count: Count::Exact(148),
        rarity: None,
        eleph: true,
    };
    set(&core, vcx, |s| {
        s.status = reading(Screen::Students, 20.);
        s.items = items().into_iter().chain([eleph]).collect();
        s.equipment = equipment();
        s.students = vec![hoshino(), akane()];
    });
    click(vcx, "tab:生徒");
    assert!(shows(vcx, "生徒を集計中"), "the status");
    for words in ["ホシノ", "90", "5MMM", "T10/10/10", "T2", "25/0/25", "34"] {
        assert!(shows(vcx, words), "ホシノ's {words}");
    }
    for words in ["アカネ（バニーガール）", "?", "2336", "T4/?/3", "—", "0/0/0", "20"] {
        assert!(shows(vcx, words), "アカネ's {words}");
    }
    // The star counts on into the unique weapon's, drawn as the game's.
    assert!(has(vcx, "stars:8"), "ホシノ at 固有4");
    assert!(has(vcx, "stars:3"), "アカネ at ★3");

    click(vcx, "search");
    keys(vcx, "ｱ ｶ ﾈ");
    assert!(!shows(vcx, "ホシノ"), "the search narrows the students");
    assert!(shows(vcx, "アカネ（バニーガール）"), "to the one asked for");
}

#[gpui::test]
fn orders_students_by_level_star_and_bond_or_by_the_column_chosen(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| {
        s.items = items();
        s.students = vec![
            trained(1, "アル", |t| {
                t.level = Some(80);
                t.bond = Some(50);
            }),
            trained(2, "ミカ", |t| {
                t.bond = Some(43);
                t.weapon = Some(Some(Weapon { level: 30, stars: 1 }));
            }),
            trained(3, "ヒナ", |t| {
                t.rarity = Some(4);
                t.weapon = Some(None);
                t.bond = Some(50);
            }),
            trained(4, "ホシノ", |t| t.bond = Some(34)),
        ];
    });
    click(vcx, "tab:生徒");
    let order = |vcx: &mut gpui::VisualTestContext| {
        let mut names: Vec<(gpui::Pixels, &str)> = ["アル", "ミカ", "ヒナ", "ホシノ"]
            .into_iter()
            .map(|n| (super::bounds(vcx, &format!("text:{n}")).expect(n).top(), n))
            .collect();
        names.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("tops compare"));
        names.into_iter().map(|(_, n)| n).collect::<Vec<_>>()
    };
    assert_eq!(order(vcx), ["ホシノ", "ミカ", "ヒナ", "アル"], "by level, then star and bond");
    click(vcx, "text:星");
    assert_eq!(order(vcx), ["ホシノ", "アル", "ミカ", "ヒナ"], "by star");
    click(vcx, "text:絆");
    assert_eq!(order(vcx), ["ヒナ", "アル", "ミカ", "ホシノ"], "by bond");
}

#[gpui::test]
fn counts_the_students_read_on_their_tab(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    assert!(!shows(vcx, "\u{a0}0"), "no count before any is read");
    set(&core, vcx, |s| {
        s.students = (1..=42).map(|i| trained(i, &format!("生徒{i}"), |_| {})).collect();
    });
    assert!(shows(vcx, "\u{a0}42"), "the students read, on the tab");
}

#[gpui::test]
fn marks_a_student_for_training_from_the_student_tab(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    set(&core, vcx, |s| s.students = vec![hoshino(), akane()]);
    click(vcx, "tab:生徒");
    click(vcx, "mark-20019");
    assert!(
        core.snapshot().targets.iter().any(|t| t.id == 20019),
        "marked for training"
    );
    click(vcx, "tab:育成");
    assert!(shows(vcx, "アカネ（バニーガール）"), "the training tab lists the student");
}
