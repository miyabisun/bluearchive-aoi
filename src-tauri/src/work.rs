//! The work tab: what the daily plays give, against what is held. The
//! blueprints of the normal missions by equipment slot, the drops of the
//! bounties and of the scrimmages, and how many students the credits and the
//! reports raise.

use std::{collections::HashMap, hash::BuildHasher};

use serde::{Deserialize, Serialize};

use crate::{Item, icon::Rarity, plan::Piece, quantity::Count};

/// The daily stages (`data/schaledb/stages.json`, written by
/// `tools/schaledb/stages.ps1`).
#[derive(Debug, Clone, Deserialize)]
pub struct Stages {
    pub missions: Vec<Mission>,
    pub bounties: Vec<Place>,
    pub scrimmages: Vec<Place>,
    pub commissions: Commissions,
}

/// A normal mission: its name ("25-1"), its AP and the blueprints a run gives
/// on average.
#[derive(Debug, Clone, Deserialize)]
pub struct Mission {
    pub name: String,
    pub ap: u32,
    pub drops: Vec<(i64, f64)>,
}

/// A bounty or a scrimmage: its kind (`ChaserA`), its name and what any of
/// its stages drops.
#[derive(Debug, Clone, Deserialize)]
pub struct Place {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub drops: Vec<i64>,
}

/// The commissions' names: the credits' and the reports'.
#[derive(Debug, Clone, Deserialize)]
pub struct Commissions {
    #[serde(rename = "FindGift")]
    pub credits: String,
    #[serde(rename = "Blood")]
    pub reports: String,
}

/// The work tab's rows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkRows {
    pub slots: Vec<SlotRow>,
    pub bounties: Vec<PlaceRow>,
    pub scrimmages: Vec<PlaceRow>,
    pub commissions: CommissionRow,
}

/// How much a count holds, and whether only about (a count in thousands).
fn amount(count: Option<Count>) -> (u64, bool) {
    match count {
        Some(Count::Exact(n)) => (u64::from(n), false),
        Some(Count::Thousands(k)) => (u64::from(k) * 1000, true),
        None => (0, false),
    }
}

/// An equipment slot as the normal missions serve it: its name, the icon of
/// its T10 piece, and what the blueprints held make of pieces from T1 to T10
/// (the T10 blueprints, and apart the universal blueprints with those of T9
/// and below counted at `RATE`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotRow {
    pub name: String,
    pub icon: String,
    pub top: Stock,
    pub universal: Stock,
}

/// Blueprints held against what one piece from T1 to T10 takes, how many
/// pieces they make, and the mission that gives the most of them per AP.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stock {
    pub held: u64,
    pub about: bool,
    pub need: u64,
    pub pieces: f64,
    pub best: Option<Best>,
}

/// A mission and what a run of it gives on average.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Best {
    pub name: String,
    pub per_run: f64,
    pub ap: u32,
}

const UNIVERSAL: &str = "の万能設計図";
/// The universal blueprints one blueprint of each tier is made from (by tier,
/// T2 to T10; the Blue Archive wiki as schale-calc's equipment-upgrade.md has
/// it, 2026-10-13).
const RATE: [u64; 11] = [0, 0, 2, 3, 5, 7, 10, 15, 20, 30, 50];
/// Blueprint and universal blueprint ids start here; below are the pieces.
const BLUEPRINTS: i64 = 100_000;

/// The slots in the order of their T10 pieces, each as `SlotRow` tells.
pub fn slots<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    missions: &[Mission],
    held: &HashMap<i64, Count, S>,
) -> Vec<SlotRow> {
    let tops = pieces.iter().filter(|p| p.id < BLUEPRINTS && p.tier == 10);
    tops.filter_map(|top| {
        let slot = |p: &&Piece| p.category == top.category;
        let blueprints: Vec<&Piece> = pieces
            .iter()
            .filter(slot)
            .filter(|p| p.id >= BLUEPRINTS)
            .collect();
        let top_print = blueprints.iter().find(|p| p.tier == 10)?.id;
        let universal = blueprints.iter().find(|p| p.tier == 0)?.id;
        // Each blueprint counted in universal ones, the T10 ones in themselves.
        let lower: Vec<(i64, u64)> = blueprints
            .iter()
            .filter(|p| (2..=9).contains(&p.tier))
            .map(|p| (p.id, RATE[usize::from(p.tier)]))
            .collect();
        let crafted = pieces.iter().filter(slot).filter(|p| p.id < BLUEPRINTS);
        let recipes: Vec<&(i64, u64)> = crafted.flat_map(|p| &p.recipe).collect();
        let need = |worth: &[(i64, u64)]| -> u64 {
            recipes
                .iter()
                .filter_map(|(id, n)| worth.iter().find(|w| w.0 == *id).map(|w| n * w.1))
                .sum()
        };
        let stock = |worth: &[(i64, u64)], need: u64| {
            let (held, about) = worth.iter().fold((0, false), |(sum, about), &(id, each)| {
                let (n, a) = amount(held.get(&id).copied());
                (sum + n * each, about || a)
            });
            Stock {
                held,
                about,
                need,
                pieces: if need == 0 {
                    0.0
                } else {
                    held as f64 / need as f64
                },
                best: best(missions, worth),
            }
        };
        let lower_with_universal: Vec<(i64, u64)> = std::iter::once((universal, 1))
            .chain(lower.iter().copied())
            .collect();
        let entry = |id: i64| equipment.iter().find(|e| e.id == id);
        Some(SlotRow {
            name: entry(universal)?.name.strip_suffix(UNIVERSAL)?.to_owned(),
            icon: entry(top.id)?.icon.clone(),
            top: stock(&[(top_print, 1)], need(&[(top_print, 1)])),
            universal: stock(&lower_with_universal, need(&lower)),
        })
    })
    .collect()
}

/// The mission that gives the most of `worth` per AP, each id at its worth.
fn best(missions: &[Mission], worth: &[(i64, u64)]) -> Option<Best> {
    missions
        .iter()
        .map(|m| {
            let per_run: f64 = m
                .drops
                .iter()
                .filter_map(|(id, n)| worth.iter().find(|w| w.0 == *id).map(|w| n * w.1 as f64))
                .sum();
            (m, per_run)
        })
        .filter(|(m, per_run)| *per_run > 0.0 && m.ap > 0)
        .max_by(|a, b| (a.1 / f64::from(a.0.ap)).total_cmp(&(b.1 / f64::from(b.0.ap))))
        .map(|(m, per_run)| Best {
            name: m.name.clone(),
            per_run,
            ap: m.ap,
        })
}

/// A bounty or a scrimmage: its name and each kind it drops, by tier.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaceRow {
    pub name: String,
    pub kinds: Vec<[Option<HeldCell>; 4]>,
}

/// A good dropped and how many are held.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeldCell {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub held: Option<Count>,
}

/// The tier a rarity stands for (N 1 … SSR 4).
fn tier(rarity: Rarity) -> usize {
    match rarity {
        Rarity::N => 0,
        Rarity::R => 1,
        Rarity::SR => 2,
        Rarity::SSR => 3,
    }
}

/// The places with what they drop of the kinds that come in tiers, grouped by
/// `kind` (none for a good of no such kind), in the order first dropped.
pub fn places<S: BuildHasher>(
    places: &[Place],
    entries: &[Item],
    kind: impl Fn(&Item) -> Option<String>,
    held: &HashMap<i64, Count, S>,
) -> Vec<PlaceRow> {
    places
        .iter()
        .map(|place| {
            let mut kinds: Vec<(String, [Option<HeldCell>; 4])> = Vec::new();
            for entry in place
                .drops
                .iter()
                .filter_map(|id| entries.iter().find(|e| e.id == *id))
            {
                let (Some(key), Some(rarity)) = (kind(entry), entry.rarity) else {
                    continue;
                };
                let found = kinds.iter().position(|(k, _)| *k == key);
                let at = found.unwrap_or_else(|| {
                    kinds.push((key, Default::default()));
                    kinds.len() - 1
                });
                kinds[at].1[tier(rarity)] = Some(HeldCell {
                    id: entry.id,
                    name: entry.name.clone(),
                    icon: entry.icon.clone(),
                    held: held.get(&entry.id).copied(),
                });
            }
            PlaceRow {
                name: place.name.clone(),
                kinds: kinds.into_iter().map(|(_, tiers)| tiers).collect(),
            }
        })
        .collect()
}

/// The commissions against what raising a student from Lv1 takes: the
/// credits held over a whole training's, and the exp of the reports held over
/// Lv1 to 90's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommissionRow {
    pub credits_name: String,
    pub credits: Option<u64>,
    pub credits_each: u64,
    pub credit_students: Option<f64>,
    pub reports_name: String,
    pub report_exp: u64,
    pub report_about: bool,
    pub exp_each: u64,
    pub report_students: f64,
}

/// The commissions as `CommissionRow` tells, the reports' exp each by id.
pub fn commissions<S: BuildHasher>(
    names: &Commissions,
    credits: Option<u64>,
    (credits_each, exp_each): (u64, u64),
    reports: &[(i64, u64)],
    held: &HashMap<i64, Count, S>,
) -> CommissionRow {
    let (report_exp, report_about) = reports.iter().fold((0, false), |(sum, about), &(id, exp)| {
        let (n, a) = amount(held.get(&id).copied());
        (sum + n * exp, about || a)
    });
    let per = |held: u64, each: u64| {
        if each == 0 {
            0.0
        } else {
            held as f64 / each as f64
        }
    };
    CommissionRow {
        credits_name: names.credits.clone(),
        credits,
        credits_each,
        credit_students: credits.map(|c| per(c, credits_each)),
        reports_name: names.reports.clone(),
        report_exp,
        report_about,
        exp_each,
        report_students: per(report_exp, exp_each),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each kind's tiers as the id and the count held.
    type Cells = Vec<Vec<Option<(i64, Option<Count>)>>>;

    fn piece(id: i64, tier: u8, recipe: &[(i64, u64)]) -> Piece {
        Piece {
            id,
            category: "Hat".into(),
            tier,
            max_level: 1,
            recipe: recipe.to_vec(),
            recipe_cost: 0,
        }
    }

    fn item(id: i64, name: &str, rarity: Rarity) -> Item {
        Item {
            id,
            name: name.into(),
            icon: format!("icon_{id}"),
            category: "Hat".into(),
            tier: 0,
            rarity: Some(rarity),
        }
    }

    #[test]
    fn a_slot_counts_its_t10_blueprints_and_apart_the_universal_with_t9_and_below() {
        // T2 takes 15 of its own; T10 60 of its own and 20 of T2's.
        let pieces = [
            piece(1001, 2, &[(101_001, 15)]),
            piece(1009, 10, &[(101_009, 60), (101_001, 20)]),
            piece(101_001, 2, &[]),
            piece(101_009, 10, &[]),
            piece(501_000, 0, &[]),
        ];
        let equipment = [
            item(1009, "ゲーミングヘルメット", Rarity::N),
            item(501_000, "帽子の万能設計図", Rarity::N),
        ];
        let missions = [
            Mission {
                name: "25-1".into(),
                ap: 10,
                drops: vec![(101_009, 0.3), (501_000, 13.0)],
            },
            Mission {
                name: "29-5".into(),
                ap: 10,
                drops: vec![(101_009, 0.4), (101_001, 1.0)],
            },
        ];
        let held = HashMap::from([
            (101_009, Count::Exact(270)),
            (101_001, Count::Exact(5)),
            (501_000, Count::Thousands(1)),
        ]);
        let rows = slots(&pieces, &equipment, &missions, &held);
        assert_eq!(rows.len(), 1);
        let hat = &rows[0];
        assert_eq!(
            (hat.name.as_str(), hat.icon.as_str()),
            ("帽子", "icon_1009")
        );
        assert_eq!((hat.top.held, hat.top.need, hat.top.pieces), (270, 60, 4.5));
        let best = hat.top.best.as_ref().unwrap();
        assert_eq!((best.name.as_str(), best.per_run), ("29-5", 0.4));
        // 1,000 universal (about) and 5 of T2's, 2 universal each, against
        // T2's 15 and T10's 20 of T2's.
        let u = &hat.universal;
        assert_eq!((u.held, u.about, u.need), (1010, true, 70));
        assert!((u.pieces - 1010.0 / 70.0).abs() < 1e-9);
        assert_eq!(u.best.as_ref().unwrap().name, "25-1");
    }

    #[test]
    fn a_place_shows_each_kind_it_drops_by_tier_with_what_is_held() {
        let entries = [
            item(3030, "初級BD（ゲヘナ）", Rarity::N),
            item(3031, "中級BD（ゲヘナ）", Rarity::R),
            item(110, "円盤の欠片", Rarity::N),
            item(61, "指名手配コイン", Rarity::N),
        ];
        let place = Place {
            kind: "ChaserA".into(),
            name: "ハイウェイ".into(),
            drops: vec![61, 110, 3030, 3031],
        };
        let kind = |e: &Item| (e.id != 61).then(|| (e.id / 10).to_string());
        let held = HashMap::from([(3031, Count::Exact(12))]);
        let rows = places(&[place], &entries, kind, &held);
        assert_eq!(rows[0].name, "ハイウェイ");
        let tiers: Cells = rows[0]
            .kinds
            .iter()
            .map(|k| {
                k.iter()
                    .map(|c| c.as_ref().map(|c| (c.id, c.held)))
                    .collect()
            })
            .collect();
        assert_eq!(
            tiers,
            [
                vec![Some((110, None)), None, None, None],
                vec![
                    Some((3030, None)),
                    Some((3031, Some(Count::Exact(12)))),
                    None,
                    None
                ],
            ]
        );
    }

    #[test]
    fn the_commissions_count_the_students_the_credits_and_the_reports_raise() {
        let names = Commissions {
            credits: "クレジット回収".into(),
            reports: "拠点防衛".into(),
        };
        let held = HashMap::from([(10, Count::Exact(1000)), (13, Count::Thousands(2))]);
        let row = commissions(
            &names,
            Some(250_000_000),
            (50_000_000, 1_000_000),
            &[(10, 50), (13, 10_000)],
            &held,
        );
        assert_eq!(row.credit_students, Some(5.0));
        // 1,000 × 50 + 2,000 × 10,000 (about).
        assert_eq!((row.report_exp, row.report_about), (20_050_000, true));
        assert!((row.report_students - 20.05).abs() < 1e-9);
        assert_eq!(
            commissions(&names, None, (1, 1), &[], &held).credit_students,
            None
        );
    }
}
