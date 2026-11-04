//! What the plan counts from: what was read when the app started or the plan was last reloaded
//! (the window's 再読込), so a student trained meanwhile leaves the plan as it is; and what has
//! been read otherwise since, for the window to tell.

use std::collections::HashMap;

use serde::Serialize;

use crate::{
    Row, keep::ReadAt, plan::star_of, quantity::Count, shop::ShopRow, student::Training,
    watch::StudentRow,
};

/// What was read: the students, the items and equipment, the credits and the shops, and when the
/// newest of it was.
#[derive(Debug, Clone, Default)]
pub struct Ground {
    pub students: Vec<StudentRow>,
    pub items: Vec<Row>,
    pub equipment: Vec<Row>,
    pub credits: Option<u64>,
    pub shops: Vec<ShopRow>,
    /// Local time ("2026-11-03 14:37:12"), none before anything was read.
    pub read_at: Option<String>,
}

/// What was read since the ground and counts otherwise than it: when the ground's newest read
/// was, the marked students trained since and the shops selling otherwise, by name, and whether
/// anything held (items, equipment, credits, coins) was counted otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Behind {
    pub since: Option<String>,
    pub students: Vec<String>,
    pub shops: Vec<String>,
    pub held: bool,
}

impl Ground {
    /// How `now` differs from the ground for the plan of the `marked` students; none where it
    /// counts the same. A student the ground lacks is counted as read now, so is never behind.
    pub fn behind(&self, now: &Self, marked: &[i64]) -> Option<Behind> {
        fn find(students: &[StudentRow], id: i64) -> Option<&StudentRow> {
            students.iter().find(|s| s.id == id)
        }
        let students: Vec<String> = marked
            .iter()
            .filter_map(|&id| Some((find(&self.students, id)?, find(&now.students, id)?)))
            .filter(|(then, now)| !alike(&then.training, &now.training))
            .map(|(_, now)| now.name.clone())
            .collect();
        let shop = |name: &str| self.shops.iter().find(|s| s.name == name);
        let shops: Vec<String> = now
            .shops
            .iter()
            .filter(|s| shop(&s.name).is_none_or(|then| then.goods != s.goods))
            .map(|s| s.name.clone())
            .collect();
        let coins = |s: &ShopRow| s.coins.iter().map(|c| c.held).collect::<Vec<_>>();
        let held = counts(&self.items) != counts(&now.items)
            || counts(&self.equipment) != counts(&now.equipment)
            || self.credits != now.credits
            || now
                .shops
                .iter()
                .any(|s| shop(&s.name).is_some_and(|then| coins(then) != coins(s)));
        (!students.is_empty() || !shops.is_empty() || held).then(|| Behind {
            since: self.read_at.clone(),
            students,
            shops,
            held,
        })
    }
}

/// Whether the plan counts the trainings alike: the level, the star (see `star_of`), the
/// skills and the equipment.
fn alike(a: &Training, b: &Training) -> bool {
    (a.level, star_of(a), a.skills, a.equipment) == (b.level, star_of(b), b.skills, b.equipment)
}

/// How many of each are held.
fn counts(rows: &[Row]) -> HashMap<i64, Count> {
    rows.iter().map(|r| (r.id, r.count)).collect()
}

/// The newest of the reads `read_at` tells and the `shops`' (none before anything was read).
pub fn newest(read_at: &ReadAt, shops: &[ShopRow]) -> Option<String> {
    [
        &read_at.items,
        &read_at.equipment,
        &read_at.students,
        &read_at.credits,
    ]
    .into_iter()
    .flatten()
    .chain(shops.iter().map(|s| &s.read_at))
    .max()
    .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shop::{CoinRow, GoodRow, OfferRow},
        student::{Gear, Weapon},
    };

    fn lana(rarity: u8, weapon: Option<u8>, bond: u8) -> StudentRow {
        StudentRow {
            id: 26017,
            name: "ラナ".to_owned(),
            training: Training {
                level: Some(90),
                rarity: Some(rarity),
                bond: Some(bond),
                skills: [Some(5), Some(8), Some(7), Some(7)],
                weapon: Some(weapon.map(|stars| Weapon { level: 30, stars })),
                equipment: [Some(Gear { tier: 10 }); 3],
                gear: Some(0),
                ability: [Some(0); 3],
            },
        }
    }

    fn row(id: i64, count: u32) -> Row {
        Row {
            id,
            name: String::new(),
            icon: String::new(),
            tier: None,
            count: Count::Exact(count),
            rarity: None,
            eleph: true,
        }
    }

    /// The カケラ(文字) shop selling the elephs `ids` (rate 1, 20 left), `coins` fragments held.
    fn kakera(ids: &[i64], coins: u32) -> ShopRow {
        ShopRow {
            name: "カケラ(文字)".to_owned(),
            read_at: "2026-11-03 14:30:00".to_owned(),
            resets: false,
            coins: vec![CoinRow {
                name: "神名のカケラ".to_owned(),
                icon: String::new(),
                held: Some(Count::Exact(coins)),
                at: 0,
            }],
            goods: ids
                .iter()
                .map(|&id| GoodRow {
                    id,
                    name: String::new(),
                    icon: String::new(),
                    offers: vec![Some(OfferRow {
                        left: 20,
                        price: None,
                        shown: Some(1),
                        coin: 0,
                    })],
                    sold: false,
                    rarity: None,
                    eleph: true,
                })
                .collect(),
        }
    }

    /// Lana at ★4 with 113 elephs, the カケラ(文字) shop not yet selling hers.
    fn ground() -> Ground {
        Ground {
            students: vec![lana(4, None, 3)],
            items: vec![row(26017, 113)],
            equipment: vec![row(1000, 5)],
            credits: Some(100),
            shops: vec![kakera(&[26016], 1458)],
            read_at: Some("2026-11-03 14:37:12".to_owned()),
        }
    }

    /// What is told behind the ground.
    fn told(students: Vec<String>, shops: Vec<String>, held: bool) -> Behind {
        Behind {
            since: Some("2026-11-03 14:37:12".to_owned()),
            students,
            shops,
            held,
        }
    }

    #[test]
    fn what_was_read_again_alike_is_not_behind() {
        let mut now = ground();
        now.read_at = Some("2026-11-04 11:20:00".to_owned());
        now.shops[0].read_at = "2026-11-04 11:20:00".to_owned();
        assert_eq!(ground().behind(&now, &[26017]), None);
    }

    #[test]
    fn a_marked_student_trained_since_is_named() {
        let mut now = ground();
        now.students = vec![lana(5, Some(1), 3)];
        assert_eq!(
            ground().behind(&now, &[26017]),
            Some(told(vec!["ラナ".to_owned()], vec![], false))
        );
        // Not marked, the plan does not count them.
        assert_eq!(ground().behind(&now, &[]), None);
    }

    #[test]
    fn what_the_plan_does_not_count_of_a_student_leaves_it_alone() {
        let mut now = ground();
        now.students = vec![lana(4, None, 4)];
        assert_eq!(ground().behind(&now, &[26017]), None);
    }

    #[test]
    fn a_student_the_ground_lacks_is_counted_as_read_now() {
        let mut first = ground();
        first.students.clear();
        assert_eq!(first.behind(&ground(), &[26017]), None);
    }

    #[test]
    fn anything_held_counted_otherwise_is_told_without_a_name() {
        let mut now = ground();
        now.items = vec![row(26017, 12)];
        assert_eq!(ground().behind(&now, &[]), Some(told(vec![], vec![], true)));
        let mut now = ground();
        now.equipment = vec![row(1000, 6)];
        assert_eq!(ground().behind(&now, &[]), Some(told(vec![], vec![], true)));
        let mut now = ground();
        now.credits = Some(90);
        assert_eq!(ground().behind(&now, &[]), Some(told(vec![], vec![], true)));
        // The coins held over a shop are held too.
        let mut now = ground();
        now.shops = vec![kakera(&[26016], 1000)];
        assert_eq!(ground().behind(&now, &[]), Some(told(vec![], vec![], true)));
    }

    #[test]
    fn a_shop_selling_otherwise_or_read_first_since_is_named() {
        let mut now = ground();
        now.shops = vec![kakera(&[26016, 26017], 1458)];
        assert_eq!(
            ground().behind(&now, &[]),
            Some(told(vec![], vec!["カケラ(文字)".to_owned()], false))
        );
        let mut now = ground();
        now.shops[0].goods[0].offers[0] =
            now.shops[0].goods[0].offers[0].map(|o| OfferRow { left: 19, ..o });
        assert_eq!(
            ground().behind(&now, &[]),
            Some(told(vec![], vec!["カケラ(文字)".to_owned()], false))
        );
        let mut first = ground();
        first.shops.clear();
        assert_eq!(
            first.behind(&ground(), &[]),
            Some(told(vec![], vec!["カケラ(文字)".to_owned()], false))
        );
    }

    #[test]
    fn the_newest_read_is_the_latest_of_the_lists_and_the_shops() {
        let read_at = ReadAt {
            items: Some("2026-11-04 11:18:53".to_owned()),
            equipment: None,
            students: Some("2026-11-04 11:18:20".to_owned()),
            credits: Some("2026-11-04 11:18:21".to_owned()),
        };
        assert_eq!(
            newest(&read_at, &[kakera(&[], 0)]).as_deref(),
            Some("2026-11-04 11:18:53")
        );
        let mut late = kakera(&[], 0);
        late.read_at = "2026-11-04 11:20:53".to_owned();
        assert_eq!(
            newest(&read_at, &[late]).as_deref(),
            Some("2026-11-04 11:20:53")
        );
        assert_eq!(newest(&ReadAt::default(), &[]), None);
    }
}
