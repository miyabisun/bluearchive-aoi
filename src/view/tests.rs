//! The derivations as the Svelte page's `view.test.js` checked them, case by case.

use super::*;
use crate::{
    plan::TierBlueprint,
    price::Price,
    shop::{CoinRow, OfferRow},
    student::{Gear, Weapon},
    work::{FarmStage, NextStage, Raise},
};

/// A line of a tally.
fn row(id: i64, name: &str, count: u32) -> Row {
    Row {
        id,
        name: name.to_owned(),
        icon: String::new(),
        tier: None,
        count: Count::Exact(count),
        rarity: None,
        eleph: false,
    }
}

/// The rows the searches look through.
fn tally() -> Vec<Row> {
    vec![
        row(10, "初級戦術教育BD（ゲヘナ）", 12),
        row(20, "ＡＰ回復ドリンク", 3),
        row(30, "Ooparts Nebra Disk", 1480),
    ]
}

fn ids(rows: &[&Row]) -> Vec<i64> {
    rows.iter().map(|r| r.id).collect()
}

fn student_ids(rows: &[&StudentRow]) -> Vec<i64> {
    rows.iter().map(|r| r.id).collect()
}

/// A training read as nothing at all.
fn unread() -> Training {
    Training {
        level: None,
        rarity: None,
        bond: None,
        skills: [None; 4],
        weapon: None,
        equipment: [None; 3],
        gear: None,
        ability: [None; 3],
    }
}

fn lines(lines: &[&str]) -> String {
    lines.join("\n")
}

// filterRows

#[test]
fn filter_rows_keeps_every_row_for_an_empty_query() {
    let rows = tally();
    assert_eq!(filter_rows(&rows, "  "), rows.iter().collect::<Vec<_>>());
}

#[test]
fn filter_rows_matches_part_of_a_name_ignoring_case_and_full_width_forms() {
    let rows = tally();
    assert_eq!(ids(&filter_rows(&rows, "ap回復")), [20]);
    assert_eq!(ids(&filter_rows(&rows, "nebra")), [30]);
}

#[test]
fn filter_rows_matches_katakana_typed_in_hiragana() {
    let named = |id, name: &str| StudentRow {
        id,
        name: name.to_owned(),
        training: unread(),
    };
    let students = [named(1, "アカネ（バニーガール）"), named(2, "ホシノ")];
    assert_eq!(student_ids(&filter_rows(&students, "あかね")), [1]);
    assert_eq!(student_ids(&filter_rows(&students, "ばにー")), [1]);
    assert!(filter_rows(&tally(), "ぎじゅつ").is_empty());
}

#[test]
fn filter_rows_needs_every_word_of_the_query() {
    let rows = tally();
    assert_eq!(ids(&filter_rows(&rows, "BD ゲヘナ")), [10]);
    assert!(filter_rows(&rows, "BD ミレニアム").is_empty());
}

// statusView: the JS snapshot without `phone` or `cast` is a phone never
// linked (`Link::Unpaired`) and no streaming app waited for (`Cast::Off`).

#[test]
fn status_view_names_each_state_and_marks_which_are_trouble() {
    let badge = |status: Status| {
        let view = status_view(&status, &Link::Unpaired, &Cast::Off);
        (view.label, view.tone)
    };
    let label = |text: &str, tone| (text.to_owned(), tone);
    assert_eq!(badge(Status::NoGame), label("ゲーム未検出", Tone::Idle));
    assert_eq!(badge(Status::NoSource), label("取り込み元なし", Tone::Idle));
    assert_eq!(
        badge(Status::OtherScreen),
        label("対象外の画面", Tone::Idle)
    );
    let items = Status::Reading {
        list: Screen::Items,
        fps: 27.5,
        dropped: 3,
    };
    assert_eq!(badge(items), label("アイテムを集計中", Tone::Active));
    let equipment = Status::Reading {
        list: Screen::Equipment,
        fps: 9.6,
        dropped: 0,
    };
    assert_eq!(badge(equipment), label("装備を集計中", Tone::Active));
    let failed = Status::Failed {
        message: "denied".to_owned(),
    };
    assert_eq!(badge(failed), label("キャプチャ失敗", Tone::Danger));
    assert_eq!(badge(Status::Paused), label("一時中断", Tone::Paused));
    assert_eq!(
        badge(Status::TooSmall),
        label("画面が小さすぎます", Tone::Idle)
    );
}

#[test]
fn status_view_shows_where_the_frames_come_from_and_says_it_and_the_frame_rate_on_hover() {
    let reading = Status::Reading {
        list: Screen::Shops,
        fps: 4.8,
        dropped: 0,
    };
    let tap = "押すと読み取りを一時中断";
    let at = |status: &Status, phone: &Link, cast: &Cast| {
        let view = status_view(status, phone, cast);
        (view.source, view.title)
    };
    assert_eq!(
        at(&reading, &Link::Unpaired, &Cast::Off),
        (
            Some(Capture::Window),
            lines(&["取り込み元: ゲームのウィンドウ", "5 fps", tap])
        )
    );
    let usb = Link::Streaming {
        device: "Pixel 9".to_owned(),
        via: Via::Usb,
    };
    assert_eq!(
        at(&reading, &usb, &Cast::Off),
        (
            Some(Capture::Phone),
            lines(&["取り込み元: Pixel 9（USB）", "5 fps", tap])
        )
    );
    let wifi = Link::Streaming {
        device: "TB322FC".to_owned(),
        via: Via::Wifi,
    };
    assert_eq!(
        at(&Status::Paused, &wifi, &Cast::Off),
        (
            Some(Capture::Phone),
            lines(&["取り込み元: TB322FC（Wi-Fi）", "押すと読み取りを再開"])
        )
    );
    // A phone linked but not sending leaves the window the source.
    let searching = Link::Searching {
        device: "Pixel 9".to_owned(),
    };
    assert_eq!(
        at(&reading, &searching, &Cast::Off).0,
        Some(Capture::Window)
    );
    let cast = Cast::Receiving {
        from: "192.168.1.5".to_owned(),
        sideways: false,
    };
    assert_eq!(
        at(&reading, &Link::Unpaired, &cast),
        (
            Some(Capture::Cast),
            lines(&["取り込み元: 配信アプリ（192.168.1.5）", "5 fps", tap])
        )
    );
    assert_eq!(
        at(&Status::NoSource, &Link::Unpaired, &Cast::Off),
        (None, tap.to_owned())
    );
}

// formatCount

#[test]
fn format_count_shows_exact_counts_with_separators_and_rounded_ones_as_the_game_does() {
    assert_eq!(format_count(Count::Exact(1383)), "1,383");
    assert_eq!(format_count(Count::Exact(7)), "7");
    assert_eq!(format_count(Count::Thousands(51)), "51K");
}

// starTitle

#[test]
fn star_title_tells_the_unique_weapons_level_then_the_elephs_held() {
    assert_eq!(
        star_title("固有武器 Lv60", Some(Count::Exact(148))),
        lines(&["固有武器 Lv60", "神名文字: 148"])
    );
    assert_eq!(star_title("", Some(Count::Thousands(2))), "神名文字: 2K");
    // Never read in the item list: not told from none held.
    assert_eq!(star_title("", None), "神名文字: ?");
}

// studentCells: the JS `weapon: null` (locked) is `Some(None)`, a weapon left
// out (`undefined`, unknown) is `None`.

/// A student whose training was read whole.
fn known() -> Training {
    Training {
        level: Some(90),
        rarity: Some(5),
        bond: Some(34),
        skills: [Some(5), Some(10), Some(7), Some(10)],
        weapon: Some(Some(Weapon {
            level: 60,
            stars: 4,
        })),
        equipment: [
            Some(Gear { tier: 10 }),
            Some(Gear { tier: 9 }),
            Some(Gear { tier: 8 }),
        ],
        gear: Some(2),
        ability: [Some(25), Some(0), Some(25)],
    }
}

#[test]
fn student_cells_writes_a_students_training_the_way_timeline_arona_does() {
    assert_eq!(
        student_cells(&known()),
        StudentCells {
            level: "90".to_owned(),
            star: Some(8),
            weapon: "固有武器 Lv60".to_owned(),
            skills: "5M7M".to_owned(),
            equipment: "T10/9/8".to_owned(),
            gear: "T2".to_owned(),
            ability: "25/0/25".to_owned(),
            bond: "34".to_owned(),
        }
    );
}

#[test]
fn student_cells_marks_what_is_locked_missing_or_unknown() {
    let cells = student_cells(&Training {
        level: None,
        weapon: Some(None),
        skills: [Some(3), None, Some(1), Some(1)],
        equipment: [Some(Gear { tier: 1 }), None, Some(Gear { tier: 1 })],
        gear: Some(0),
        ability: [Some(0), None, Some(0)],
        ..known()
    });
    assert_eq!(cells.level, "?");
    // Under ★5 the weapon is locked; at ★5 its stars must be read.
    let locked = student_cells(&Training {
        rarity: Some(3),
        weapon: Some(None),
        ..known()
    });
    assert_eq!((locked.star, locked.weapon.as_str()), (Some(3), ""));
    let rarity = student_cells(&Training {
        rarity: None,
        ..known()
    });
    assert_eq!(rarity.star, None);
    assert_eq!(cells.skills, "3?11");
    assert_eq!(cells.equipment, "T1/?/1");
    assert_eq!(cells.gear, "—");
    assert_eq!(cells.ability, "0/?/0");
    let unknown = student_cells(&Training {
        weapon: None,
        ..known()
    });
    assert_eq!((unknown.star, unknown.weapon.as_str()), (None, ""));
}

// sortStudents: the JS column names are `StudentColumn`s, "level" the default.

/// A student whose training is a 5MMM T10 one, as `change` leaves it.
fn student(id: i64, change: impl FnOnce(&mut Training)) -> StudentRow {
    let mut training = Training {
        level: Some(90),
        rarity: Some(5),
        bond: Some(20),
        skills: [Some(5), Some(10), Some(10), Some(10)],
        weapon: Some(Some(Weapon {
            stars: 4,
            level: 60,
        })),
        equipment: [Some(Gear { tier: 10 }); 3],
        gear: Some(0),
        ability: [Some(0); 3],
    };
    change(&mut training);
    StudentRow {
        id,
        name: format!("生徒{id}"),
        training,
    }
}

fn weapon(stars: u8, level: u8) -> Weapon {
    Weapon { level, stars }
}

#[test]
fn sort_students_puts_the_highest_level_first_then_the_star_then_the_bond_by_default() {
    let rows = [
        student(1, |t| {
            t.level = Some(80);
            t.bond = Some(50);
        }),
        student(2, |t| {
            t.rarity = Some(4);
            t.bond = Some(50);
        }),
        student(3, |t| t.bond = Some(20)),
        student(4, |t| {
            t.bond = Some(30);
            t.weapon = Some(Some(weapon(1, 30)));
            t.skills = [Some(1); 4];
        }),
    ];
    // The star counts on into the unique weapon's: ★5 with it at ★4 is above ★5 with it at ★1.
    let sorted = sort_students(&rows, StudentColumn::default());
    assert_eq!(student_ids(&sorted), [3, 4, 2, 1]);
}

#[test]
fn sort_students_puts_what_could_not_be_read_last() {
    let rows = [
        student(1, |t| t.level = None),
        student(2, |t| t.level = Some(1)),
        student(3, |_| {}),
    ];
    let sorted = sort_students(&rows, StudentColumn::default());
    assert_eq!(student_ids(&sorted), [3, 2, 1]);
}

#[test]
fn sort_students_orders_by_a_chosen_column_highest_first_the_default_order_breaking_ties() {
    let rows = [
        student(1, |t| t.weapon = Some(Some(weapon(1, 30)))),
        student(2, |t| t.weapon = Some(None)),
        student(3, |t| t.weapon = None),
        student(4, |t| t.weapon = Some(Some(weapon(4, 50)))),
        student(5, |t| {
            t.level = Some(87);
            t.weapon = Some(Some(weapon(4, 60)));
        }),
        student(6, |_| {}),
    ];
    let sorted = sort_students(&rows, StudentColumn::Star);
    assert_eq!(student_ids(&sorted), [6, 5, 4, 1, 2, 3]);
}

#[test]
fn sort_students_adds_up_the_skills_the_equipment_tiers_and_the_ability_releases() {
    let rows = [
        student(1, |t| t.skills = [Some(5), Some(10), Some(10), Some(9)]),
        student(2, |t| t.skills = [Some(3), Some(10), Some(10), Some(10)]),
        student(3, |t| t.skills = [Some(5), Some(10), None, Some(10)]),
    ];
    let sorted = sort_students(&rows, StudentColumn::Skills);
    assert_eq!(student_ids(&sorted), [1, 2, 3]);
    let tiers = |first: u8| [first, 10, 10].map(|tier| Some(Gear { tier }));
    let geared = [
        student(1, |t| t.equipment = tiers(9)),
        student(2, |t| t.equipment = tiers(8)),
        student(3, |_| {}),
    ];
    let sorted = sort_students(&geared, StudentColumn::Equipment);
    assert_eq!(student_ids(&sorted), [3, 1, 2]);
    let released = [
        student(1, |t| t.ability = [Some(25), Some(0), Some(0)]),
        student(2, |t| t.ability = [Some(25), Some(0), Some(25)]),
    ];
    let sorted = sort_students(&released, StudentColumn::Ability);
    assert_eq!(student_ids(&sorted), [2, 1]);
}

// Shops: `shop_stale` takes the shop's `resets` and `read_at`, and the time
// now as the backend writes it (the JS took a `Date`).

fn good(id: i64, name: &str, offers: Vec<Option<OfferRow>>) -> GoodRow {
    GoodRow {
        id,
        name: name.to_owned(),
        icon: String::new(),
        offers,
        sold: false,
        rarity: None,
        eleph: false,
    }
}

fn offer(left: u32, price: Option<Price>) -> OfferRow {
    OfferRow {
        left,
        price,
        shown: None,
        coin: 0,
    }
}

fn terms(price: u32, amount: u32, limit: u32) -> Price {
    Price {
        price,
        amount,
        limit,
    }
}

fn coin_row(name: &str, held: Option<Count>) -> CoinRow {
    CoinRow {
        name: name.to_owned(),
        icon: format!("icon_{name}"),
        held,
        at: 0,
    }
}

fn shop_row(name: &str, coins: &[&str], goods: Vec<GoodRow>) -> ShopRow {
    ShopRow {
        name: name.to_owned(),
        read_at: String::new(),
        resets: true,
        coins: coins
            .iter()
            .map(|&coin| coin_row(coin, Some(Count::Exact(50))))
            .collect(),
        goods,
    }
}

#[test]
fn shop_stale_tells_a_shop_that_resets_read_in_an_earlier_month() {
    let now = "2026-11-01 09:00:00";
    assert!(shop_stale(true, "2026-10-31 23:00:00", now));
    assert!(!shop_stale(true, "2026-11-01 08:00:00", now));
    // カケラ(文字) does not reset.
    assert!(!shop_stale(false, "2026-09-01 08:00:00", now));
}

#[test]
fn shop_stale_starts_a_shops_month_on_the_1st_at_4_00() {
    let october = "2026-10-31 23:00:00";
    assert!(!shop_stale(true, october, "2026-11-01 03:59:00"));
    assert!(shop_stale(true, october, "2026-11-01 04:00:00"));
    // Read in the 1st's first hours, it is the month before's.
    let early = "2026-11-01 03:30:00";
    assert!(!shop_stale(true, early, "2026-11-01 03:50:00"));
    assert!(shop_stale(true, early, "2026-11-01 04:00:00"));
    // New Year's first hours are December's.
    let december = "2025-12-31 22:00:00";
    assert!(!shop_stale(true, december, "2026-01-01 03:00:00"));
    assert!(shop_stale(true, december, "2026-01-01 04:00:00"));
}

#[test]
fn good_tile_writes_what_each_coin_buys_the_times_left_at_its_price_or_sold_out() {
    let text = |text: &str, sold| {
        Some(OfferText {
            text: text.to_owned(),
            sold,
        })
    };
    let eleph = good(
        10_019,
        "アズサの神名文字",
        vec![
            Some(offer(20, Some(terms(50, 5, 20)))),
            Some(offer(0, Some(terms(50, 5, 10)))),
        ],
    );
    assert_eq!(
        good_tile(&eleph),
        GoodTile {
            amount: "×5".to_owned(),
            offers: vec![text("20回@50", false), text("売切れ", true)],
        }
    );
    let bd = good(
        3000,
        "初級戦術教育BD（百鬼夜行）",
        vec![Some(offer(1200, Some(terms(1800, 1, 6)))), None],
    );
    assert_eq!(
        good_tile(&bd),
        GoodTile {
            amount: "×1".to_owned(),
            offers: vec![text("1,200回@1,800", false), None],
        }
    );
    // Without terms, the price is unknown and so is how many come.
    assert_eq!(
        good_tile(&good(1, "?", vec![Some(offer(3, None))])),
        GoodTile {
            amount: String::new(),
            offers: vec![text("3回@?", false)],
        }
    );
}

#[test]
fn good_face_paints_a_good_on_its_rarity_a_students_eleph_on_its_own_and_one_unknown_on_none() {
    assert_eq!(
        good_face(false, Some(Rarity::SR)),
        Some(Face::Rarity(Rarity::SR))
    );
    assert_eq!(good_face(true, Some(Rarity::N)), Some(Face::Eleph));
    assert_eq!(good_face(false, None), None);
}

#[test]
fn filter_shops_narrows_each_shops_goods_by_name_and_leaves_out_a_shop_with_none() {
    let coins = ["総力戦レアコイン", "総力戦コイン"];
    let shops = vec![
        shop_row(
            "総力戦",
            &coins,
            vec![
                good(1, "秘伝ノート", vec![]),
                good(2, "エーテルの粉", vec![]),
            ],
        ),
        shop_row("指名手配", &coins, vec![good(3, "エーテルの欠片", vec![])]),
    ];
    let narrowed = ShopRow {
        goods: vec![shops[0].goods[0].clone()],
        ..shops[0].clone()
    };
    assert_eq!(filter_shops(&shops, "秘伝"), [narrowed]);
    assert_eq!(filter_shops(&shops, ""), shops);
}

// The plan

fn need(kind: Kind, name: &str, need: u64, have: Option<Held>) -> NeedRow {
    NeedRow {
        kind,
        id: 0,
        name: name.to_owned(),
        icon: String::new(),
        family: None,
        need,
        have,
    }
}

#[test]
fn need_cells_tells_what_is_short_against_what_is_held() {
    let notes = need(
        Kind::Item,
        "初級技術ノート（ゲヘナ）",
        30,
        Some(Held::Exact(12)),
    );
    assert_eq!(
        need_cells(&notes),
        NeedCells {
            name: "初級技術ノート（ゲヘナ）".to_owned(),
            need: "30".to_owned(),
            have: "12".to_owned(),
            short: "不足 18".to_owned(),
            enough: false,
        }
    );
    let credits = need(Kind::Credits, "", 1_500_000, Some(Held::Exact(244_654_006)));
    assert_eq!(
        need_cells(&credits),
        NeedCells {
            name: "クレジット".to_owned(),
            need: "1.5M".to_owned(),
            have: "244.7M".to_owned(),
            short: "足りる".to_owned(),
            enough: true,
        }
    );
    let short = need(Kind::Credits, "", 28_431_500, Some(Held::Exact(12_000_000)));
    assert_eq!(need_cells(&short).short, "不足 16.4M");
    let reports = need(Kind::ReportExp, "", 60_000, Some(Held::About(51_000)));
    assert_eq!(
        need_cells(&reports),
        NeedCells {
            name: "レポート（経験値）".to_owned(),
            need: "60,000".to_owned(),
            have: "約51,000".to_owned(),
            short: "不足 約9,000".to_owned(),
            enough: false,
        }
    );
    assert_eq!(need_cells(&need(Kind::OrbExp, "", 5, None)).short, "?");
}

#[test]
fn group_plan_puts_the_four_tiers_of_a_kind_on_one_line_the_rest_apart() {
    let line = |id: i64, family| NeedRow {
        id,
        family,
        ..need(Kind::Item, &id.to_string(), 1, None)
    };
    let rows = [
        need(Kind::Credits, "", 5, None),
        line(4002, Some((4000, 3))),
        line(3000, Some((3000, 1))),
        line(4000, Some((4000, 1))),
        line(9999, None),
    ];
    assert_eq!(
        group_plan(&rows),
        PlanGroups {
            families: vec![
                Family {
                    id: 3000,
                    tiers: [Some(&rows[2]), None, None, None],
                },
                Family {
                    id: 4000,
                    tiers: [Some(&rows[3]), None, Some(&rows[1]), None],
                },
            ],
            rest: vec![&rows[0], &rows[4]],
        }
    );
}

#[test]
fn tier_cell_writes_a_tier_as_needed_against_held() {
    assert_eq!(
        tier_cell(30, Some(Held::Exact(12))),
        TierCell {
            text: "30 / 12".to_owned(),
            enough: false,
        }
    );
    assert_eq!(
        tier_cell(3, Some(Held::About(51_000))),
        TierCell {
            text: "3 / 約51,000".to_owned(),
            enough: true,
        }
    );
}

#[test]
fn blueprint_cells_writes_a_slots_blueprints_by_tier_and_how_they_are_paid() {
    let tier = |tier: u8, need: u64, have: u64| TierBlueprint {
        tier,
        id: 100_999 + i64::from(tier),
        name: format!("T{tier}の設計図"),
        icon: format!("t{tier}"),
        need,
        have: Held::Exact(have),
    };
    let universal = TierBlueprint {
        tier: 0,
        id: 501_000,
        name: "帽子の万能設計図".to_owned(),
        icon: "u".to_owned(),
        need: 1200,
        have: Held::About(1000),
    };
    let row = BlueprintRow {
        name: "帽子".to_owned(),
        icon: "hat".to_owned(),
        tiers: vec![tier(2, 15, 10), tier(3, 0, 30), tier(10, 60, 17)],
        universal: universal.clone(),
        short: 43,
    };
    let cell = |tier: u8, text: &str, tone| BlueprintTier {
        key: 100_999 + i64::from(tier),
        label: format!("T{tier}"),
        name: format!("T{tier}の設計図"),
        icon: format!("t{tier}"),
        text: text.to_owned(),
        tone,
    };
    assert_eq!(
        blueprint_cells(&row),
        BlueprintCells {
            name: "帽子".to_owned(),
            icon: "hat".to_owned(),
            tiers: vec![
                // Below T10 the universal ones pay the rest: no colour.
                cell(2, "15 / 10", None),
                // Not needed: nothing to show.
                cell(3, "", None),
                cell(10, "60 / 17", Some(Tone::Short)),
            ],
            // What T2 to T9 take of the universal ones over those held, coloured like T10.
            universal: UniversalCell {
                name: "帽子の万能設計図".to_owned(),
                icon: "u".to_owned(),
                text: "1,200 / 約1,000".to_owned(),
                tone: Tone::Short,
            },
        }
    );
    let enough = blueprint_cells(&BlueprintRow {
        tiers: vec![tier(10, 60, 60)],
        universal: TierBlueprint {
            need: 10,
            ..universal
        },
        ..row
    });
    assert_eq!(
        (enough.tiers[0].tone, enough.universal.tone),
        (Some(Tone::Enough), Tone::Enough)
    );
}

#[test]
fn star_name_names_a_star_the_students_1_to_5_then_the_unique_weapons() {
    // A teacher calls ★5 by its weapon's first star.
    assert_eq!(
        [1, 4, 5, 6, 8].map(star_name),
        ["★1", "★4", "固有1", "固有2", "固有4"]
    );
}

#[test]
fn star_marks_draws_a_star_as_the_game_does_the_students_in_yellow_the_unique_weapons_in_blue() {
    let marks = |kind, places, label: &str| StarMarks {
        kind,
        places,
        label: label.to_owned(),
    };
    assert_eq!(star_marks(3), marks(StarKind::Student, 0..3, "★3"));
    let four = star_marks(4);
    assert_eq!((four.kind, four.places), (StarKind::Student, 0..4));
    // 固有1 (the student's ★5) to 固有4 as the weapon's stars.
    assert_eq!(star_marks(5), marks(StarKind::Weapon, 0..1, "固有1"));
    assert_eq!(star_marks(7), marks(StarKind::Weapon, 0..3, "固有3"));
    assert_eq!(star_marks(8).places.len(), 4);
}

#[test]
fn star_choices_offers_every_star_as_a_target_none_below_the_students_now() {
    assert_eq!(
        star_choices(Some(3)).map(|c| (c.star, c.disabled)),
        [
            (1, true),
            (2, true),
            (3, false),
            (4, false),
            (5, false),
            (6, false),
            (7, false),
            (8, false),
        ]
    );
    // A star never read takes any.
    assert!(star_choices(None).iter().all(|c| !c.disabled));
}

#[test]
fn menu_place_opens_a_menu_under_its_button_over_it_where_the_window_ends_first() {
    let window = Extent {
        width: 720.0,
        height: 800.0,
    };
    let menu = Extent {
        width: 100.0,
        height: 250.0,
    };
    let button = Anchor {
        top: 100.0,
        bottom: 130.0,
        left: 40.0,
    };
    let at = |left, top| MenuAt { left, top };
    assert_eq!(menu_place(button, window, menu), at(40.0, 134.0));
    // 310 + 4 + 250 runs past 400: above the button instead.
    let low = Anchor {
        top: 280.0,
        bottom: 310.0,
        left: 40.0,
    };
    let lower = Extent {
        height: 400.0,
        ..window
    };
    assert_eq!(menu_place(low, lower, menu), at(40.0, 26.0));
    // Nowhere whole: below, from the window's top at the least.
    let short = Extent {
        width: 720.0,
        height: 200.0,
    };
    let top = Anchor {
        top: 20.0,
        bottom: 50.0,
        left: 0.0,
    };
    assert_eq!(menu_place(top, short, menu), at(4.0, 4.0));
    // A button by the window's right edge: the menu kept inside it.
    let right = Anchor {
        top: 100.0,
        bottom: 130.0,
        left: 300.0,
    };
    let narrow = Extent {
        width: 360.0,
        height: 800.0,
    };
    assert_eq!(menu_place(right, narrow, menu), at(256.0, 134.0));
}

#[test]
fn star_of_counts_a_students_star_on_into_the_unique_weapons() {
    let at = |rarity, weapon| {
        star_of(&Training {
            rarity,
            weapon,
            ..unread()
        })
    };
    assert_eq!(
        [
            at(Some(3), Some(None)),
            at(Some(5), Some(Some(weapon(2, 40)))),
            at(Some(5), None),
            at(None, Some(None)),
        ],
        [Some(3), Some(6), None, None]
    );
}

#[test]
fn unknown_text_names_the_parts_that_could_not_be_counted_student_by_student() {
    let unknown = |id, name: &str, part| UnknownRow {
        id,
        name: name.to_owned(),
        part,
    };
    assert_eq!(
        unknown_text(&[
            unknown(-1, "ツムギ", Part::Skill(0)),
            unknown(-1, "ツムギ", Part::Equipment(2)),
            unknown(10, "アル", Part::Level),
        ]),
        ["ツムギ: EX・装備3", "アル: Lv"]
    );
}

// The work tab follows the game: the month-end shops by name alone.

fn month_end() -> Vec<PlanShop> {
    ["総力戦", "大決戦", "合同火力演習", "指名手配", "熟達証書"]
        .map(|name| PlanShop {
            name: name.to_owned(),
            read_at: None,
            coins: vec![],
            cards: vec![],
        })
        .into()
}

#[test]
fn work_fold_opens_the_fold_of_the_screen_or_the_month_end_shop_the_game_shows() {
    let shops = month_end();
    let fold = |place| work_fold(Some(place), &shops);
    assert_eq!(
        fold(Place::Screen("学園交流会")).as_deref(),
        Some("work:学園交流会")
    );
    // The 指名手配 stages' screen, told by its title: the daily fold.
    assert_eq!(
        fold(Place::Screen("指名手配")).as_deref(),
        Some("work:指名手配")
    );
    // The 任務 screen showing the Normal campaign: the 任務 fold (Normal's).
    assert_eq!(
        fold(Place::Screen("任務 Normal")).as_deref(),
        Some("work:任務")
    );
    assert_eq!(
        fold(Place::Screen("特別依頼")).as_deref(),
        Some("work:特別依頼")
    );
    // The shop screen with 通常アイテム chosen: the daily fold.
    assert_eq!(
        fold(Place::Screen("ショップ: 通常アイテム")).as_deref(),
        Some("work:通常アイテム")
    );
    assert_eq!(
        fold(Place::Shop("総力戦")).as_deref(),
        Some("work:月末:総力戦")
    );
    assert_eq!(
        fold(Place::Shop("熟達証書")).as_deref(),
        Some("work:月末:熟達証書")
    );
    // The 指名手配 shop, not the daily 指名手配 stages.
    assert_eq!(
        fold(Place::Shop("指名手配")).as_deref(),
        Some("work:月末:指名手配")
    );
}

#[test]
fn work_fold_opens_nothing_where_the_work_tab_has_no_fold_for_it() {
    let shops = month_end();
    assert_eq!(work_fold(Some(Place::Shop("カケラ(文字)")), &shops), None);
    assert_eq!(work_fold(Some(Place::Screen("任務")), &shops), None);
    assert_eq!(work_fold(None, &shops), None);
}

// Behind the plan

#[test]
fn behind_text_tells_when_the_plans_reads_were_and_what_changed_since_the_held_last() {
    let behind = Behind {
        since: Some("2026-11-03 14:37:12".to_owned()),
        students: vec!["ラナ".to_owned()],
        shops: vec!["カケラ(文字)".to_owned()],
        held: true,
    };
    assert_eq!(
        behind_text(Some(&behind)),
        "11/03 14:37 の読み取りで計算中。ラナ・カケラ(文字)・所持数が変わっています。再読込で数え直します（払った分は取り消されます）"
    );
    let held = Behind {
        since: None,
        students: vec![],
        shops: vec![],
        held: true,
    };
    assert_eq!(
        behind_text(Some(&held)),
        "所持数が変わっています。再読込で数え直します（払った分は取り消されます）"
    );
}

#[test]
fn behind_text_tells_nothing_while_the_plan_counts_alike() {
    assert_eq!(behind_text(None), "");
}

// Procurement: the JS `{ shift, right }` is a `Click`, the payments a
// `BTreeMap` by the same "line|source" keys.

const CLICK: Click = Click {
    right: false,
    shift: false,
};
const SHIFT: Click = Click {
    right: false,
    shift: true,
};
const RIGHT: Click = Click {
    right: true,
    shift: false,
};
const RIGHT_SHIFT: Click = Click {
    right: true,
    shift: true,
};

fn choice_box() -> Source {
    Source::Box {
        id: 150_003,
        name: "最上級技術ノート選択ボックス".to_owned(),
        icon: "b".to_owned(),
        held: 24,
        choices: 12,
    }
}

/// The 総力戦 shop's good at `price` coins, `held` of them read.
fn plain_at(price: u32, held: Option<u64>) -> Source {
    Source::Shop {
        shop: "総力戦".to_owned(),
        coin: "総力戦コイン".to_owned(),
        icon: "c".to_owned(),
        held,
        price,
        gives: 1,
        left: 20,
    }
}

fn plain() -> Source {
    plain_at(25, Some(5940))
}

fn oopart() -> Source {
    Source::Shop {
        shop: "合同火力演習".to_owned(),
        coin: "火力演習コイン".to_owned(),
        icon: "c".to_owned(),
        held: Some(5940),
        price: 2,
        gives: 2,
        left: 20,
    }
}

fn kakera() -> Source {
    Source::Fragments {
        name: "神名のカケラ".to_owned(),
        icon: "k".to_owned(),
        held: 2941,
        rate: 1,
        left: 586,
    }
}

fn universal() -> Source {
    Source::Universal {
        id: 501_000,
        name: "帽子の万能設計図".to_owned(),
        icon: "u".to_owned(),
        held: 4825,
        rate: 50,
    }
}

fn line(
    kind: Kind,
    id: i64,
    name: &str,
    icon: &str,
    short: u64,
    sources: Vec<Source>,
) -> ProcureLine {
    ProcureLine {
        kind,
        id,
        name: name.to_owned(),
        icon: icon.to_owned(),
        short,
        sources,
    }
}

fn notes() -> ProcureLine {
    line(
        Kind::Item,
        4033,
        "最上級技術ノート（ゲヘナ）",
        "n",
        29,
        vec![choice_box(), plain()],
    )
}

fn payments(paid: &[(&str, u64)]) -> BTreeMap<String, u64> {
    paid.iter().map(|&(key, n)| (key.to_owned(), n)).collect()
}

#[test]
fn settle_pays_t10_blueprints_from_the_universal_ones_50_each() {
    let hat = line(
        Kind::Equipment,
        101_009,
        "帽子のT10設計図",
        "t10",
        60,
        vec![universal()],
    );
    assert_eq!(step(&universal(), 0, 60, SHIFT), 60);
    let settled = settle(
        &[hat],
        &payments(&[("equipment-101009|universal:501000", 3)]),
    );
    assert_eq!((settled.lines[0].given, settled.lines[0].left), (3, 57));
    assert_eq!(
        settled.resources,
        [Resource {
            key: "universal:501000".to_owned(),
            name: "帽子の万能設計図".to_owned(),
            icon: "u".to_owned(),
            held: Some(4825),
            spent: 150,
            after: Some(4675),
        }]
    );
    let cells = procure_cells(&settled);
    let chip = &cells.lines[0].chips[0];
    assert_eq!(
        (chip.text.as_str(), chip.given.as_str(), chip.out),
        ("4,675", "+3", false)
    );
    assert_eq!(
        chip.title,
        "帽子の万能設計図: T10設計図 1 枚に 50 枚（T2〜T9 に使った残り 4,825 枚）"
    );
}

#[test]
fn fragment_cost_prices_elephs_in_fragments_20_at_each_rate_up_to_4_then_5_each() {
    assert_eq!(
        [0, 10, 30, 100].map(|n| fragment_cost(1, n)),
        [0, 10, 40, 300]
    );
    assert_eq!(fragment_cost(4, 30), 80 + 50);
    assert_eq!(fragment_cost(5, 3), 15);
}

#[test]
fn step_pays_one_a_click_takes_one_back_a_right_click_all_or_none_with_shift() {
    let (choice, plain) = (choice_box(), plain());
    // 29 lacking: one more, or as many as make it up.
    assert_eq!(step(&choice, 0, 29, CLICK), 1);
    assert_eq!(step(&choice, 0, 29, SHIFT), 29);
    // A box may be opened past those held: the rest can be tailor-made.
    assert_eq!(step(&choice, 30, 5, SHIFT), 35);
    // A shop sells no more than it has left this month.
    assert_eq!(step(&plain, 0, 29, SHIFT), 20);
    assert_eq!(step(&plain, 20, 9, CLICK), 20);
    // Two ooparts a purchase.
    assert_eq!(step(&oopart(), 0, 5, SHIFT), 3);
    // A click pays one even once the line is made up; shift adds none then.
    assert_eq!(step(&choice, 3, 0, CLICK), 4);
    assert_eq!(step(&choice, 3, 0, SHIFT), 3);
    assert_eq!(step(&choice, 3, 0, RIGHT), 2);
    assert_eq!(step(&choice, 0, 5, RIGHT), 0);
    assert_eq!(step(&choice, 7, 5, RIGHT_SHIFT), 0);
}

#[test]
fn settle_settles_the_payments_what_each_line_still_lacks_and_what_each_resource_has_left() {
    let bd = line(
        Kind::Item,
        3032,
        "上級戦術教育BD（ゲヘナ）",
        "d",
        6,
        vec![plain_at(50, Some(5940))],
    );
    let mutsuki = line(Kind::Item, 20_059, "ムツキ", "m", 386, vec![kakera()]);
    let paid = payments(&[
        ("item-4033|box:150003", 30),
        ("item-4033|shop:総力戦:総力戦コイン", 2),
        ("item-3032|shop:総力戦:総力戦コイン", 6),
        ("item-20059|fragments", 30),
    ]);
    let result = settle(&[notes(), bd.clone(), mutsuki], &paid);
    let settled: Vec<_> = result
        .lines
        .iter()
        .map(|l| (l.key.as_str(), l.given, l.left))
        .collect();
    assert_eq!(
        settled,
        [
            ("item-4033", 32, 0),
            ("item-3032", 6, 0),
            ("item-20059", 30, 356)
        ]
    );
    // A chip shows how much of its resource is left after every line's payments.
    let chips: Vec<_> = result.lines[0]
        .chips
        .iter()
        .map(|c| (c.key.as_str(), c.n, c.gives, c.after))
        .collect();
    assert_eq!(
        chips,
        [
            ("box:150003", 30, 30, Some(-6)),
            ("shop:総力戦:総力戦コイン", 2, 2, Some(5940 - 50 - 300)),
        ]
    );
    let resource = |key: &str, name: &str, icon: &str, held, spent, after| Resource {
        key: key.to_owned(),
        name: name.to_owned(),
        icon: icon.to_owned(),
        held: Some(held),
        spent,
        after: Some(after),
    };
    assert_eq!(
        result.resources,
        [
            resource(
                "box:150003",
                "最上級技術ノート選択ボックス",
                "b",
                24,
                30,
                -6
            ),
            resource("coin:総力戦コイン", "総力戦コイン", "c", 5940, 350, 5590),
            resource("fragments", "神名のカケラ", "k", 2941, 40, 2941 - 40),
        ]
    );
    // A coin not read is "?" after too.
    let unread = settle(
        &[ProcureLine {
            sources: vec![plain_at(50, None)],
            ..bd
        }],
        &BTreeMap::new(),
    );
    assert_eq!(unread.resources[0].after, None);
}

/// The credits short, made up from the 熟達証書 shop.
fn credits_line() -> ProcureLine {
    let certificates = Source::Shop {
        shop: "熟達証書".to_owned(),
        coin: "熟達証書".to_owned(),
        icon: "j".to_owned(),
        held: Some(5009),
        price: 1500,
        gives: 5_000_000,
        left: 4,
    };
    line(Kind::Credits, 0, "", "", 7_000_000, vec![certificates])
}

#[test]
fn procure_cells_writes_the_procurement_each_line_its_chips_and_the_resources_after() {
    let credits = credits_line();
    let cells = procure_cells(&settle(
        &[credits.clone(), notes()],
        &payments(&[
            ("credits-0|shop:熟達証書:熟達証書", 1),
            ("item-4033|box:150003", 30),
        ]),
    ));
    let lines: Vec<_> = cells
        .lines
        .iter()
        .map(|l| {
            let text = [&l.key, &l.name, &l.icon, &l.short, &l.left].map(String::as_str);
            (text, l.done)
        })
        .collect();
    assert_eq!(
        lines,
        [
            (["credits-0", "クレジット", "", "7M", "2M"], false),
            (
                ["item-4033", "最上級技術ノート（ゲヘナ）", "n", "29", "0"],
                true
            ),
        ]
    );
    assert_eq!(
        cells.lines[0].chips,
        [ChipCells {
            key: "shop:熟達証書:熟達証書".to_owned(),
            icon: "j".to_owned(),
            text: "3,509".to_owned(),
            negative: false,
            given: "+5M".to_owned(),
            out: false,
            title: "熟達証書: 熟達証書 1,500 で ×5,000,000、今月あと 4 回".to_owned(),
        }]
    );
    let chip = &cells.lines[1].chips[0];
    assert_eq!(
        (
            chip.text.as_str(),
            chip.negative,
            chip.given.as_str(),
            chip.title.as_str()
        ),
        (
            "-6",
            true,
            "+30",
            "最上級技術ノート選択ボックス（12 種から 1 つ）"
        )
    );
    // The settlement lists what is paid from, the resources left untouched aside:
    // held, spent and what is left, above none green and below red.
    assert_eq!(
        cells.resources,
        [
            ResourceCells {
                key: "coin:熟達証書".to_owned(),
                name: "熟達証書".to_owned(),
                icon: "j".to_owned(),
                held: "5,009".to_owned(),
                spent: "-1,500".to_owned(),
                after: "3,509".to_owned(),
                tone: Some(Tone::Plus),
            },
            ResourceCells {
                key: "box:150003".to_owned(),
                name: "最上級技術ノート選択ボックス".to_owned(),
                icon: "b".to_owned(),
                held: "24".to_owned(),
                spent: "-30".to_owned(),
                after: "-6".to_owned(),
                tone: Some(Tone::Minus),
            },
        ]
    );
    // A shop that sells no more this month says so; a box never runs out.
    let sold = procure_cells(&settle(
        &[credits],
        &payments(&[("credits-0|shop:熟達証書:熟達証書", 4)]),
    ));
    let out = &sold.lines[0].chips[0];
    assert_eq!(
        (out.text.as_str(), out.out, out.given.as_str()),
        ("売切れ", true, "+20M")
    );
    assert!(!cells.lines[1].chips[0].out);
    let even = procure_cells(&settle(
        &[notes()],
        &payments(&[("item-4033|box:150003", 24)]),
    ));
    assert_eq!(
        (even.resources[0].after.as_str(), even.resources[0].tone),
        ("0", None)
    );
}

// Work

#[test]
fn stock_cells_writes_how_many_pieces_a_slots_blueprints_make_short_of_one_red_of_the_golden_ratio_orange()
 {
    let stock = |pieces| Stock {
        held: 270,
        about: false,
        need: 60,
        pieces,
    };
    assert_eq!(
        stock_cells(&stock(4.5), 3),
        StockCells {
            pieces: "4.5個分".to_owned(),
            held: "270 / 60".to_owned(),
            tone: None,
        }
    );
    assert_eq!(stock_cells(&stock(2.96), 3).tone, Some(Tone::Low));
    assert_eq!(stock_cells(&stock(0.9), 3).tone, Some(Tone::Short));
    // Short of the ratio never shows as reaching it: 238 / 60 is 3.96, cut to a tenth.
    let cut = stock_cells(
        &Stock {
            held: 238,
            about: false,
            need: 60,
            pieces: 238.0 / 60.0,
        },
        4,
    );
    assert_eq!(
        (cut.pieces.as_str(), cut.tone),
        ("3.9個分", Some(Tone::Low))
    );
    let about = Stock {
        held: 2150,
        about: true,
        need: 430,
        pieces: 5.0,
    };
    assert_eq!(stock_cells(&about, 4).held, "約2,150 / 430");
}

#[test]
fn golden_title_tells_why_a_slots_golden_ratio_is_so_many_its_share_of_every_students_slots_in_24()
{
    let golden = Golden {
        students: 3,
        slots: 103,
        all: 834,
        total: 24,
    };
    assert_eq!(
        golden_title(&golden),
        lines(&[
            "黄金比 3人",
            "全実装生徒の装備欄 834 のうち 103",
            "103 ÷ 834 × 24 = 2.96（0.25以上は切り上げ）",
        ])
    );
    // Cut to two decimals as the rounding goes by: 2.129… shows 2.12, never 2.13.
    let cut = Golden {
        students: 2,
        slots: 74,
        ..golden
    };
    assert!(golden_title(&cut).contains("= 2.12（"));
}

#[test]
fn held_tone_tells_a_drop_held_short_of_one_students_need_of_two_or_enough_for_two() {
    assert_eq!(
        held_tone(Some(Count::Exact(30)), Some(75)),
        Some(Tone::Short)
    );
    assert_eq!(
        held_tone(Some(Count::Exact(100)), Some(75)),
        Some(Tone::Fair)
    );
    assert_eq!(
        held_tone(Some(Count::Exact(150)), Some(75)),
        Some(Tone::Enough)
    );
    assert_eq!(
        held_tone(Some(Count::Thousands(2)), Some(75)),
        Some(Tone::Enough)
    );
    assert_eq!(held_tone(None, Some(75)), Some(Tone::Short));
    assert_eq!(held_tone(Some(Count::Exact(3)), None), None);
}

#[test]
fn farm_cells_writes_the_farm_to_the_golden_ratio_a_line_a_mission_and_the_missions_past_it() {
    let names = |names: &[&str]| names.iter().map(|&name| name.to_owned()).collect();
    let farm = Farm {
        multiplier: 2,
        runs: 71,
        ap: 710,
        stages: vec![
            FarmStage {
                name: "29-1".to_owned(),
                runs: 60,
                ap: 600,
                slots: names(&["お守り", "時計"]),
            },
            FarmStage {
                name: "25-1".to_owned(),
                runs: 11,
                ap: 110,
                slots: names(&["帽子"]),
            },
        ],
        next: vec![NextStage {
            name: "29-2".to_owned(),
            slots: names(&["ヘアピン", "シューズ", "腕時計"]),
        }],
    };
    let farm_line = |name: &str, runs: &str, ap: &str, slots: &str| FarmLine {
        name: name.to_owned(),
        runs: runs.to_owned(),
        ap: ap.to_owned(),
        slots: slots.to_owned(),
    };
    assert_eq!(
        farm_cells(&farm),
        FarmCells {
            lines: vec![
                farm_line("29-1", "60回", "600AP", "お守り・時計"),
                farm_line("25-1", "11回", "110AP", "帽子"),
            ],
            total: FarmTotal {
                runs: "71回".to_owned(),
                ap: "710AP".to_owned(),
            },
            next: vec![NextLine {
                name: "29-2".to_owned(),
                slots: "ヘアピン・シューズ・腕時計".to_owned(),
            }],
        }
    );
}

#[test]
fn place_name_names_a_place_without_the_kind_of_play_it_is() {
    assert_eq!(
        [
            "指名手配：ハイウェイ",
            "学園交流会 - トリニティ",
            "拠点防衛"
        ]
        .map(place_name),
        ["ハイウェイ", "トリニティ", "拠点防衛"]
    );
}

/// A kind's four tiers from `id` on, as the enhancement materials show them: the
/// JS cells carried only an id and a count; a `HeldCell` is named and needs none here.
fn tiers_from(id: i64) -> [Option<HeldCell>; 4] {
    std::array::from_fn(|t| {
        Some(HeldCell {
            id: id + t as i64,
            name: String::new(),
            icon: String::new(),
            held: Some(Count::Exact(t as u32)),
            need: None,
        })
    })
}

/// The commissions' haul, the reports `about` or not. The month-end tests' JS
/// row had no report tiers; a `CommissionRow` always carries them.
fn commissions(about: bool) -> CommissionRow {
    CommissionRow {
        credits_name: "クレジット回収".to_owned(),
        credits: Some(248_955_686),
        credits_each: 41_500_000,
        credit_students: Some(5.999),
        reports_name: "拠点防衛".to_owned(),
        report_tiers: tiers_from(10),
        report_exp: 6_245_925,
        report_about: about,
        exp_each: 1_249_185,
        report_students: 5.0,
    }
}

/// A row of the work tab's counts, untoned and without a why.
fn raise_row(
    key: &str,
    title: &str,
    held: RaiseHeld,
    students: &str,
    numerator: &str,
    denominator: &str,
) -> RaiseRow {
    RaiseRow {
        key: key.to_owned(),
        title: title.to_owned(),
        held,
        students: students.to_owned(),
        numerator: numerator.to_owned(),
        denominator: denominator.to_owned(),
        tone: None,
        why: None,
    }
}

fn credits_held(text: &str) -> RaiseHeld {
    RaiseHeld::Credits(text.to_owned())
}

#[test]
fn commission_rows_counts_the_students_each_commissions_haul_raises_the_reports_first_the_fewer_marked()
 {
    // A commission's name over its haul held: the reports by tier, the credits as a
    // count beside their icon; the students raised and how at the right.
    assert_eq!(
        commission_rows(&commissions(true)),
        [
            RaiseRow {
                tone: Some(Tone::Fewer),
                ..raise_row(
                    "reports",
                    "拠点防衛",
                    RaiseHeld::Tiers(tiers_from(10).to_vec()),
                    "5.0人分",
                    "約6.2M EXP",
                    "/ 1.2M EXP",
                )
            },
            RaiseRow {
                tone: Some(Tone::More),
                ..raise_row(
                    "credits",
                    "クレジット回収",
                    credits_held("248,955,686"),
                    "5.9人分",
                    "249M",
                    "/ 41.5M",
                )
            },
        ]
    );
    // Credits never read: no count, and neither raises fewer.
    let unread = CommissionRow {
        credits: None,
        credit_students: None,
        ..commissions(true)
    };
    let [reports, credits] = commission_rows(&unread);
    assert_eq!(reports.tone, None);
    assert_eq!(
        credits,
        raise_row(
            "credits",
            "クレジット回収",
            credits_held("—"),
            "?",
            "?",
            "/ 41.5M"
        )
    );
}

#[test]
fn currency_rows_lists_the_credits_then_the_reports_as_the_commissions_count_them() {
    let row = commissions(false);
    let [reports, credits] = commission_rows(&row);
    assert_eq!(
        currency_rows(&row),
        [
            RaiseRow {
                title: "クレジット".to_owned(),
                ..credits
            },
            RaiseRow {
                title: "レポート".to_owned(),
                ..reports
            },
        ]
    );
}

#[test]
fn currency_coins_lists_the_coins_held_then_the_shops_coins_the_item_list_lacks_each_once() {
    let coin = |name: &str, held| HeldCell {
        id: 1,
        name: name.to_owned(),
        icon: format!("icon_{name}"),
        held,
        need: None,
    };
    let coins = [
        coin("総力戦コイン", Some(Count::Exact(1160))),
        coin("戦術コイン", None),
    ];
    let shops = [
        shop_row("", &["総力戦レアコイン", "総力戦コイン"], vec![]),
        shop_row("", &["熟達証書"], vec![]),
        shop_row("", &["熟達証書"], vec![]),
    ];
    let listed = currency_coins(&coins, &shops);
    let named: Vec<_> = listed.iter().map(|c| (c.name.as_str(), c.held)).collect();
    assert_eq!(
        named,
        [
            ("総力戦コイン", Some(Count::Exact(1160))),
            ("戦術コイン", None),
            ("総力戦レアコイン", Some(Count::Exact(50))),
            ("熟達証書", Some(Count::Exact(50))),
        ]
    );
}

#[test]
fn scrimmage_rows_names_each_schools_scrimmage_over_its_parts_held_the_first_to_run_out_marked() {
    let places = [
        PlaceRow {
            name: "学園交流会 - トリニティ".to_owned(),
            kinds: vec![tiers_from(10)],
        },
        PlaceRow {
            name: "学園交流会 - ゲヘナ".to_owned(),
            kinds: vec![tiers_from(20)],
        },
    ];
    let school = SchoolRow {
        exp: 220_280,
        about: false,
        rate: 1.5,
        exp_each: 49_605,
        students: 6.661,
        share: 0.4593,
        first: true,
    };
    let schools = [
        school.clone(),
        SchoolRow {
            students: 7.0,
            first: false,
            share: 0.26,
            ..school
        },
    ];
    let rows = scrimmage_rows(&places, &schools);
    let visible: Vec<_> = rows
        .iter()
        .map(|r| RaiseRow {
            why: None,
            ..r.clone()
        })
        .collect();
    assert_eq!(
        visible,
        [
            RaiseRow {
                tone: Some(Tone::First),
                ..raise_row(
                    "学園交流会 - トリニティ",
                    "トリニティ",
                    RaiseHeld::Tiers(tiers_from(10).to_vec()),
                    "6.6人分",
                    "330.4K EXP",
                    "/ 49.6K EXP",
                )
            },
            raise_row(
                "学園交流会 - ゲヘナ",
                "ゲヘナ",
                RaiseHeld::Tiers(tiers_from(20).to_vec()),
                "7.0人分",
                "330.4K EXP",
                "/ 49.6K EXP",
            ),
        ]
    );
    // Why on hover, as `school_cells` tells it.
    assert_eq!(rows[0].why, Some(school_cells(&schools[0]).why));
}

// Month-end and daily shops

#[test]
fn normal_item_rows_counts_the_students_the_credits_and_the_orbs_held_raise_the_fewer_marked() {
    // The orbs as the equipment tab's enhancement materials show them.
    let orbs = Enhancement {
        category: "Exp".to_owned(),
        tiers: tiers_from(1),
        raise: Raise {
            exp: 5_700_780,
            about: true,
            rate: 1.0,
            exp_each: 741_690,
            students: 7.686,
        },
    };
    assert_eq!(
        normal_item_rows(&commissions(false), Some(&orbs)),
        [
            RaiseRow {
                tone: Some(Tone::Fewer),
                ..raise_row(
                    "credits",
                    "クレジット",
                    credits_held("248,955,686"),
                    "5.9人分",
                    "249M",
                    "/ 41.5M",
                )
            },
            RaiseRow {
                tone: Some(Tone::More),
                ..raise_row(
                    "orbs",
                    "強化珠",
                    RaiseHeld::Tiers(tiers_from(1).to_vec()),
                    "7.6人分",
                    "約5.7M EXP",
                    "/ 0.74M EXP",
                )
            },
        ]
    );
    let unread = CommissionRow {
        credits: None,
        credit_students: None,
        ..commissions(false)
    };
    let tones: Vec<_> = normal_item_rows(&unread, Some(&orbs))
        .iter()
        .map(|r| r.tone)
        .collect();
    assert_eq!(tones, [None, None]);
    // Before the equipment is counted by kind, the credits alone.
    let alone: Vec<_> = normal_item_rows(&commissions(false), None)
        .into_iter()
        .map(|r| (r.key, r.tone))
        .collect();
    assert_eq!(alone, [("credits".to_owned(), None)]);
}

fn card() -> PlanCard {
    PlanCard {
        id: 0,
        name: String::new(),
        icon: String::new(),
        coin: 0,
        left: 16,
        price: Some(50),
        amount: Some(5),
        reserved: false,
        barred: false,
        held: None,
        eleph: false,
        rarity: None,
    }
}

#[test]
fn plan_card_tells_a_cards_times_left_at_its_price_bought_up_or_its_student_done() {
    let cells = |amount: &str, text: &str, star| PlanCardCells {
        amount: amount.to_owned(),
        text: text.to_owned(),
        star,
    };
    assert_eq!(plan_card(&card()), cells("×5", "16回@50", None));
    let unpriced = PlanCard {
        price: None,
        amount: None,
        ..card()
    };
    assert_eq!(plan_card(&unpriced), cells("", "16回@?", None));
    let bought = PlanCard { left: 0, ..card() };
    assert_eq!(plan_card(&bought), cells("×5", "売切れ", None));
    // The student at 固有4: the weapon's four stars in the place of the times left.
    let barred = PlanCard {
        left: 0,
        barred: true,
        ..card()
    };
    assert_eq!(plan_card(&barred), cells("×5", "", Some(8)));
}

/// アズサ at 固有2 and ハナコ at ★3.
fn azusa_and_hanako() -> [StudentRow; 2] {
    [
        StudentRow {
            id: 10_019,
            name: "アズサ".to_owned(),
            training: Training {
                rarity: Some(5),
                weapon: Some(Some(weapon(2, 40))),
                ..unread()
            },
        },
        StudentRow {
            id: 10_020,
            name: "ハナコ".to_owned(),
            training: Training {
                rarity: Some(3),
                weapon: Some(None),
                ..unread()
            },
        },
    ]
}

#[test]
fn plan_title_tells_on_hover_how_many_of_a_cards_good_are_held_and_an_elephs_students_star() {
    let students = azusa_and_hanako();
    let azusa = PlanCard {
        id: 10_019,
        name: "アズサの神名文字".to_owned(),
        held: Some(Count::Exact(120)),
        eleph: true,
        ..card()
    };
    assert_eq!(
        plan_title(&azusa, &students),
        lines(&["アズサの神名文字", "星: 固有2", "所持: 120"])
    );
    let hanako = PlanCard {
        id: 10_020,
        name: "ハナコの神名文字".to_owned(),
        ..azusa.clone()
    };
    assert_eq!(
        plan_title(&hanako, &students),
        lines(&["ハナコの神名文字", "星: ★3", "所持: 120"])
    );
    let unread = PlanCard {
        id: 10_099,
        held: None,
        ..azusa.clone()
    };
    assert_eq!(
        plan_title(&unread, &students),
        lines(&["アズサの神名文字", "星: 未読み取り", "所持: ?"])
    );
    let note = PlanCard {
        id: 9999,
        name: "秘伝ノート".to_owned(),
        held: Some(Count::Thousands(2)),
        eleph: false,
        ..azusa
    };
    assert_eq!(
        plan_title(&note, &students),
        lines(&["秘伝ノート", "所持: 2K"])
    );
}

#[test]
fn plan_coin_tells_the_coins_left_once_the_cards_reserved_are_paid_marked_while_any_is_reserved() {
    let coin = PlanCoin {
        name: "総力戦コイン".to_owned(),
        icon: String::new(),
        held: Some(Count::Exact(600)),
        need: 200,
        short: false,
    };
    assert_eq!(
        plan_coin(&coin),
        PlanCoinCells {
            left: "400".to_owned(),
            reserved: true,
            short: false,
            title: "総力戦コイン: 所持 600 − 予約 200".to_owned(),
        }
    );
    // Nothing reserved: the count held, unmarked.
    let free = plan_coin(&PlanCoin {
        need: 0,
        ..coin.clone()
    });
    assert_eq!((free.left.as_str(), free.reserved), ("600", false));
    let over = plan_coin(&PlanCoin {
        need: 1700,
        short: true,
        ..coin.clone()
    });
    assert_eq!((over.left.as_str(), over.short), ("-1,100", true));
    let thousands = PlanCoin {
        held: Some(Count::Thousands(3)),
        need: 500,
        ..coin.clone()
    };
    assert_eq!(plan_coin(&thousands).left, "2,500");
    assert_eq!(
        plan_coin(&PlanCoin { held: None, ..coin }),
        PlanCoinCells {
            left: "?".to_owned(),
            reserved: true,
            short: false,
            title: "総力戦コイン: 所持 ? − 予約 200".to_owned(),
        }
    );
}

// Item tiles: `tile_title` takes the good's id, name and whether it is an
// eleph (the JS took the good).

#[test]
fn tile_title_tells_on_hover_an_elephs_students_star_any_other_item_by_its_name() {
    let students = azusa_and_hanako();
    assert_eq!(
        tile_title(10_019, "アズサの神名文字", true, &students),
        lines(&["アズサの神名文字", "星: 固有2"])
    );
    assert_eq!(
        tile_title(10_020, "ハナコの神名文字", true, &students),
        lines(&["ハナコの神名文字", "星: ★3"])
    );
    assert_eq!(
        tile_title(10_099, "ミカの神名文字", true, &students),
        lines(&["ミカの神名文字", "星: 未読み取り"])
    );
    assert_eq!(
        tile_title(9999, "秘伝ノート", false, &students),
        "秘伝ノート"
    );
}

#[test]
fn held_text_writes_a_count_held_a_dash_where_its_list_never_read_it() {
    assert_eq!(held_text(Some(Count::Exact(1234))), "1,234");
    assert_eq!(held_text(Some(Count::Thousands(51))), "51K");
    assert_eq!(held_text(None), "—");
}

// Hard missions

fn hard_card() -> HardCard {
    HardCard {
        id: 10_000,
        name: "アルの神名文字".to_owned(),
        icon: String::new(),
        stages: vec!["18-3".to_owned()],
        ap: 60,
        star: Some(7),
        held: Some(Count::Exact(80)),
        done: false,
        reserved: false,
    }
}

#[test]
fn hard_tile_tells_a_students_card_the_missions_it_is_dropped_at_the_star_and_the_elephs_held() {
    // The user's アル: 148 held at 固有3 reads "固有3 148文字", never what 固有4 still takes (52).
    let aru = HardCard {
        held: Some(Count::Exact(148)),
        ..hard_card()
    };
    assert_eq!(
        hard_tile(&aru),
        HardTile {
            amount: "1個所".to_owned(),
            star: Some(7),
            held: "148文字".to_owned(),
            title: lines(&[
                "アルの神名文字",
                "星: 固有3",
                "所持: 148",
                "任務 Hard: 18-3",
                "1日 60 AP",
            ]),
        }
    );
    let yuuka = hard_tile(&HardCard {
        stages: ["1-1", "4-3", "12-2"].map(str::to_owned).into(),
        ap: 180,
        star: Some(2),
        held: Some(Count::Exact(1280)),
        ..hard_card()
    });
    assert_eq!(
        (yuuka.amount.as_str(), yuuka.star, yuuka.held.as_str()),
        ("3個所", Some(2), "1,280文字")
    );
    assert!(yuuka.title.contains("任務 Hard: 1-1、4-3、12-2"));
    // A student never read, the item list never read; one at 固有4.
    let unread = hard_tile(&HardCard {
        star: None,
        held: None,
        ..hard_card()
    });
    assert_eq!((unread.star, unread.held.as_str()), (None, "?文字"));
    assert!(unread.title.contains("星: 未読み取り\n所持: ?"));
    let done = hard_tile(&HardCard {
        star: Some(8),
        done: true,
        ..hard_card()
    });
    assert_eq!((done.star, done.held.as_str()), (Some(8), "80文字"));
}

#[test]
fn hard_cost_counts_a_days_ap_of_the_students_reserved_those_at_the_top_star_too() {
    let reserved = |ap, done| HardCard {
        ap,
        done,
        reserved: true,
        ..hard_card()
    };
    let other = HardCard {
        ap: 120,
        ..hard_card()
    };
    assert_eq!(
        hard_cost(&[reserved(60, false), reserved(180, false), other]),
        240
    );
    // Elephs farmed past 固有4 wait for a 固有5 the game hints at (the user's of 2026-10-22).
    assert_eq!(hard_cost(&[reserved(60, false), reserved(120, true)]), 180);
    assert_eq!(hard_cost(&[]), 0);
}

// Enhancement materials and schools

#[test]
fn enhancement_cells_tells_how_many_students_a_kind_of_enhancement_material_raises_as_the_work_tab_counts_it()
 {
    let kind = |category: &str, raise| Enhancement {
        category: category.to_owned(),
        tiers: [None, None, None, None],
        raise,
    };
    let cells = |students: &str, numerator: &str, denominator: &str| RaiseCells {
        students: students.to_owned(),
        numerator: numerator.to_owned(),
        denominator: denominator.to_owned(),
    };
    let orbs = Raise {
        exp: 5_700_780,
        about: true,
        rate: 1.0,
        exp_each: 741_690,
        students: 7.686,
    };
    // The orbs in millions, as the work tab's normal items shop: the count, the exp held
    // over the exp one student takes (the user's of 2026-10-29: three lines).
    assert_eq!(
        enhancement_cells(&kind("Exp", orbs)),
        cells("7.6人分", "約5.7M EXP", "/ 0.74M EXP")
    );
    // A kind of parts with its weapons' rate taken in (the user's 「x1.5倍は計算してしまう」),
    // thousands as K rounded to a tenth (49,605 → 49.6K).
    let parts = Raise {
        exp: 220_280,
        about: false,
        rate: 1.5,
        exp_each: 49_605,
        students: 6.661,
    };
    assert_eq!(
        enhancement_cells(&kind("WeaponExpGrowthA", parts)),
        cells("6.6人分", "330.4K EXP", "/ 49.6K EXP")
    );
    // Rounded half up, not cut: 172,650 is 172.7K; below a thousand as it is, a million on in M.
    let at = |exp, rate| {
        enhancement_cells(&kind("WeaponExpGrowthZ", Raise { exp, rate, ..parts })).numerator
    };
    assert_eq!(
        [at(172_650, 1.0), at(999, 1.0), at(1_234_567, 1.0)],
        ["172.7K EXP", "999 EXP", "1.2M EXP"]
    );
    assert_eq!(at(129_050, 1.5), "193.6K EXP");
    // Always to a tenth, as the counts of students: 258,975 is 259.0K.
    assert_eq!(at(172_650, 1.5), "259.0K EXP");
}

#[test]
fn school_cells_counts_the_weapons_a_schools_parts_raise_and_how_the_first_to_run_out_marked() {
    let row = SchoolRow {
        exp: 220_280,
        about: false,
        rate: 1.5,
        exp_each: 49_605,
        students: 6.661,
        share: 0.4593,
        first: true,
    };
    assert_eq!(
        school_cells(&row),
        SchoolCells {
            students: "6.6人分".to_owned(),
            numerator: "330.4K EXP".to_owned(),
            denominator: "/ 49.6K EXP".to_owned(),
            first: true,
            // Why it is red, on hover: the students its parts suit, and per them.
            why: lines(&[
                "素材が合う生徒: 46%",
                "6.6人分 ÷ 46% = 14.5",
                "最も少ないので先に回します",
            ]),
        }
    );
    let later = school_cells(&SchoolRow {
        about: true,
        first: false,
        ..row
    });
    assert_eq!(
        (later.numerator.as_str(), later.first, later.why),
        (
            "約330.4K EXP",
            false,
            lines(&["素材が合う生徒: 46%", "6.6人分 ÷ 46% = 14.5"])
        )
    );
}

// boxTotals: the summed box is the first set's `HeldCell` and its parts
// (the JS spread the cell's fields beside `parts`).

fn cell(id: i64, held: Option<Count>) -> HeldCell {
    HeldCell {
        id,
        name: format!("箱{id}"),
        icon: format!("icon_{id}"),
        held,
        need: None,
    }
}

fn set(bd: [Option<HeldCell>; 4], notes: [Option<HeldCell>; 4]) -> SkillBoxes {
    SkillBoxes {
        all: false,
        schools: vec![],
        bd,
        notes,
    }
}

fn total(cell: HeldCell, parts: &str) -> BoxTotal {
    BoxTotal {
        cell,
        parts: parts.to_owned(),
    }
}

#[test]
fn box_totals_adds_every_sets_boxes_tier_by_tier_and_says_how_much_each_holds() {
    let exact = |n| Some(Count::Exact(n));
    let sets = [
        set(
            [
                Some(cell(1, exact(429))),
                Some(cell(2, None)),
                None,
                Some(cell(4, exact(9000))),
            ],
            [Some(cell(5, None)), None, None, None],
        ),
        set(
            [
                Some(cell(11, exact(5))),
                Some(cell(12, None)),
                None,
                Some(cell(14, exact(2000))),
            ],
            [Some(cell(15, None)), None, None, None],
        ),
        set(
            [
                None,
                Some(cell(22, None)),
                None,
                Some(cell(24, Some(Count::Thousands(12)))),
            ],
            [None, None, None, None],
        ),
    ];
    let totals = box_totals(&sets).unwrap();
    // A set without the tier's box, or holding none, counts 0.
    assert_eq!(
        totals.bd[0],
        Some(total(cell(1, exact(434)), "429 + 5 + 0"))
    );
    // None held anywhere stays unknown, as one box would.
    assert_eq!(totals.bd[1], Some(total(cell(2, None), "0 + 0 + 0")));
    assert_eq!(totals.bd[2], None);
    // From 10,000 in thousands, as the game writes it.
    assert_eq!(
        totals.bd[3],
        Some(total(
            cell(4, Some(Count::Thousands(23))),
            "9,000 + 2,000 + 12K"
        ))
    );
    assert_eq!(
        totals.notes,
        [Some(total(cell(5, None), "0 + 0 + 0")), None, None, None]
    );
}

#[test]
fn box_totals_has_nothing_without_sets() {
    assert_eq!(box_totals(&[]), None);
}

// Updates: the JS `{ state, ... }` objects the page built are `UpdateState`s.

#[test]
fn update_text_tells_where_checking_for_an_update_stands() {
    let text = |update| update_text(&update);
    assert_eq!(text(UpdateState::Checking), "アップデートを確認しています…");
    assert_eq!(
        text(UpdateState::Latest {
            current: "v0.1.29".to_owned()
        }),
        "最新のバージョンです（v0.1.29）"
    );
    assert_eq!(
        text(UpdateState::Available {
            current: "v0.1.29".to_owned(),
            latest: "v0.1.30".to_owned()
        }),
        "v0.1.30 があります（今 v0.1.29）"
    );
    assert_eq!(
        text(UpdateState::Installing {
            latest: "v0.1.30".to_owned()
        }),
        "v0.1.30 に更新しています…"
    );
    assert_eq!(
        text(UpdateState::Failed {
            error: "通信できません".to_owned()
        }),
        "アップデートできませんでした: 通信できません"
    );
}

// The phone

#[test]
fn phone_line_tells_where_the_link_to_a_phone_stands() {
    let device = |name: &str| name.to_owned();
    assert_eq!(
        phone_line(&Link::Unpaired),
        "スマホとはまだつないでいません"
    );
    assert_eq!(
        phone_line(&Link::Pairing { qr: vec![] }),
        "この QR コードを読み取るか、USB ケーブルでつないでください"
    );
    assert_eq!(
        phone_line(&Link::Searching {
            device: device("Pixel 9")
        }),
        "Pixel 9 を探しています…"
    );
    assert_eq!(
        phone_line(&Link::Allowing),
        "スマホに出た「USB デバッグを許可しますか？」で許可を押してください"
    );
    assert_eq!(
        phone_line(&Link::Streaming {
            device: device("Pixel 9"),
            via: Via::Wifi
        }),
        "Pixel 9 の画面を Wi-Fi で受け取っています"
    );
    assert_eq!(
        phone_line(&Link::Streaming {
            device: device("TB322FC"),
            via: Via::Usb
        }),
        "TB322FC の画面を USB で受け取っています"
    );
    assert_eq!(
        phone_line(&Link::Stopped {
            device: device("Pixel 9")
        }),
        "Pixel 9 とは切断しています"
    );
    assert_eq!(
        phone_line(&Link::Failed {
            message: "ペア設定できませんでした".to_owned()
        }),
        "ペア設定できませんでした"
    );
}

/// `qrPath` drew an SVG path; `qr_squares` gives the squares it drew.
#[test]
fn qr_squares_draws_a_qr_codes_dark_modules_inside_its_quiet_zone() {
    let modules = [vec![true, false], vec![false, true]];
    // Four modules of quiet zone around the code ("M4 4h1v1h-1z M5 5h1v1h-1z").
    assert_eq!(qr_squares(&modules), [(4, 4), (5, 5)]);
}

// The streaming app

fn waiting(problem: Option<&str>) -> Cast {
    Cast::Waiting {
        url: "rtmp://192.168.1.20:1935/aoi/Ab3".to_owned(),
        moblin: vec![],
        plain: vec![],
        problem: problem.map(str::to_owned),
    }
}

fn receiving(sideways: bool) -> Cast {
    Cast::Receiving {
        from: "192.168.1.20".to_owned(),
        sideways,
    }
}

#[test]
fn cast_line_tells_where_waiting_for_a_streaming_app_stands() {
    assert_eq!(
        cast_line(&Cast::Off),
        "待ち受けると、スマホの配信アプリ（iPhone は Moblin）から画面を受け取れます"
    );
    assert_eq!(
        cast_line(&waiting(None)),
        "配信アプリでこの QR コードを読み取り、配信を始めてください"
    );
    assert_eq!(
        cast_line(&receiving(false)),
        "192.168.1.20 から画面を受け取っています"
    );
    assert_eq!(
        cast_line(&Cast::Failed {
            message: "ポート 1935 で待ち受けられません".to_owned()
        }),
        "ポート 1935 で待ち受けられません"
    );
}

#[test]
fn cast_note_adds_what_went_wrong_with_the_last_stream_or_how_to_send_the_screen_larger() {
    assert_eq!(cast_note(&waiting(None)), None);
    assert_eq!(
        cast_note(&waiting(Some("キーの違う配信が届いたので断りました"))).as_deref(),
        Some("キーの違う配信が届いたので断りました")
    );
    assert_eq!(cast_note(&receiving(false)), None);
    assert_eq!(
        cast_note(&receiving(true)).as_deref(),
        Some(
            "画面が横倒しで小さく届いています。配信アプリで縦向き（Portrait）の配信にすると大きく届きます"
        )
    );
    assert_eq!(cast_note(&Cast::Off), None);
}

/// A line of the plan for an item: `need` against `have` held (none: never read).
fn need_line(id: i64, name: &str, need: u64, have: Option<u64>) -> crate::plan::NeedRow {
    crate::plan::NeedRow {
        kind: crate::plan::Kind::Item,
        id,
        name: name.to_owned(),
        icon: format!("icon_{id}"),
        family: None,
        need,
        have: have.map(crate::plan::Held::Exact),
    }
}

/// A bounty dropping the items `ids`, a kind each.
fn bounty(name: &str, ids: &[i64]) -> crate::work::PlaceRow {
    let cell = |id: i64| crate::work::HeldCell {
        id,
        name: format!("item {id}"),
        icon: format!("icon_{id}"),
        held: None,
        need: None,
    };
    crate::work::PlaceRow {
        name: name.to_owned(),
        kinds: ids
            .iter()
            .map(|&id| [Some(cell(id)), None, None, None])
            .collect(),
    }
}

#[test]
fn bounty_candidates_name_the_bounties_dropping_what_falls_short() {
    let plan = [
        need_line(4033, "最上級技術ノート（ゲヘナ）", 30, Some(12)),
        need_line(3013, "上級戦術教育BD（ゲヘナ）", 10, Some(4)),
    ];
    let bounties = [
        bounty("指名手配：ハイウェイ", &[4033]),
        bounty("指名手配：砂漠の鉄道", &[3013, 4033]),
        bounty("指名手配：教室", &[9999]),
    ];
    let found = bounty_candidates(&plan, &bounties);
    let seen: Vec<(i64, &str, Vec<&str>)> = found
        .iter()
        .map(|c| {
            (
                c.id,
                c.short.as_str(),
                c.places.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            (4033, "18", vec!["砂漠の鉄道", "ハイウェイ"]),
            (3013, "6", vec!["砂漠の鉄道"]),
        ],
        "each line short, in the plan's order, with the bounties that drop it, the one dropping more lines first"
    );
}

#[test]
fn bounty_candidates_leave_out_what_is_enough_or_dropped_nowhere() {
    let plan = [
        need_line(4033, "enough", 30, Some(30)),
        need_line(8000, "nowhere", 5, Some(0)),
    ];
    let bounties = [bounty("指名手配：ハイウェイ", &[4033])];
    assert!(
        bounty_candidates(&plan, &bounties).is_empty(),
        "nothing short and dropped by a bounty"
    );
}

#[test]
fn bounty_candidates_keep_what_was_never_read_its_shortfall_unknown() {
    let plan = [need_line(3013, "unread", 10, None)];
    let bounties = [bounty("指名手配：ハイウェイ", &[3013])];
    let found = bounty_candidates(&plan, &bounties);
    assert_eq!(found.len(), 1, "a line never read may fall short");
    assert_eq!(found[0].short, "?", "how short is not known");
}

#[test]
fn bounty_candidates_put_first_the_place_making_up_the_most_shortfalls() {
    let plan = [
        need_line(4033, "notes", 30, Some(12)),
        need_line(3013, "bd", 10, Some(4)),
        need_line(9001, "oopart", 4, Some(0)),
    ];
    let places = [
        bounty("指名手配：ハイウェイ", &[4033]),
        bounty("指名手配：砂漠の鉄道", &[4033, 3013, 9001]),
        bounty("学園交流会：トリニティ", &[9001]),
    ];
    let found = bounty_candidates(&plan, &places);
    let first: Vec<&str> = found.iter().map(|c| c.places[0].as_str()).collect();
    assert_eq!(
        first,
        ["砂漠の鉄道", "砂漠の鉄道", "砂漠の鉄道"],
        "the place dropping three lines short comes first for each"
    );
    let oopart = &found[2];
    assert_eq!(
        oopart.places,
        ["砂漠の鉄道", "トリニティ"],
        "a scrimmage counts as a daily place too"
    );
}

#[test]
fn bounty_candidates_take_any_line_a_place_drops_whatever_its_kind() {
    let mut line = need_line(5000, "a piece", 3, Some(1));
    line.kind = crate::plan::Kind::Equipment;
    let found = bounty_candidates(&[line], &[bounty("指名手配：教室", &[5000])]);
    assert_eq!(found.len(), 1, "a line of equipment a place drops counts");
}

#[test]
fn count_lines_tells_each_good_of_a_cell_and_its_count_a_line_each() {
    let cell = |name: &str, held| HeldCell {
        id: 1,
        name: name.to_owned(),
        icon: String::new(),
        held,
        need: None,
    };
    let (bd, note) = (
        cell("初級戦術教育BD（ゲヘナ）", Some(Count::Exact(677))),
        cell("初級技術ノート（ゲヘナ）", None),
    );
    assert_eq!(
        count_lines(&[Some(&bd), Some(&note)]),
        "初級戦術教育BD（ゲヘナ）: 677\n初級技術ノート（ゲヘナ）: —"
    );
    assert_eq!(
        count_lines(&[None, Some(&bd)]),
        "初級戦術教育BD（ゲヘナ）: 677",
        "a tier the kind lacks tells nothing"
    );
}
