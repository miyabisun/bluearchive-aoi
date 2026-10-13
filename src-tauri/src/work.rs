//! The work tab: what the daily plays give, against what is held. The
//! blueprints of the normal missions by equipment slot, the drops of the
//! bounties and of the scrimmages, and how many students the credits and the
//! reports raise.

use std::{collections::HashMap, hash::BuildHasher};

use serde::{Deserialize, Serialize};

use crate::{
    Item,
    icon::Rarity,
    plan::{Grower, Growth, Piece},
    quantity::Count,
};

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
    pub farm: Option<Farm>,
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
/// its T10 piece, how many students' pieces from T1 to T10 the golden ratio
/// keeps of it, and what the blueprints held make of pieces (the T10
/// blueprints, and apart the universal blueprints with those of T9 and below
/// counted at `RATE`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotRow {
    pub name: String,
    pub icon: String,
    pub golden: u32,
    pub top: Stock,
    pub universal: Stock,
}

/// Blueprints held against what one piece from T1 to T10 takes, and how many
/// pieces they make.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stock {
    pub held: u64,
    pub about: bool,
    pub need: u64,
    pub pieces: f64,
}

const UNIVERSAL: &str = "の万能設計図";
/// The universal blueprints one blueprint of each tier is made from (by tier,
/// T2 to T10; the Blue Archive wiki as schale-calc's equipment-upgrade.md has
/// it, 2026-10-13).
const RATE: [u64; 11] = [0, 0, 2, 3, 5, 7, 10, 15, 20, 30, 50];
/// Blueprint and universal blueprint ids start here; below are the pieces.
const BLUEPRINTS: i64 = 100_000;
/// The golden ratio (オニア先生の黄金比, as schale-calc's onia-golden-ratio.md
/// has it): how many students' pieces from T1 to T10 to keep of each slot.
const GOLDEN: [(&str, u32); 9] = [
    ("Hat", 3),
    ("Gloves", 2),
    ("Shoes", 4),
    ("Bag", 2),
    ("Badge", 2),
    ("Hairpin", 4),
    ("Charm", 1),
    ("Watch", 4),
    ("Necklace", 2),
];

/// A slot's blueprints: its universal one, and each tier's with what one piece
/// from T1 to T10 takes of it.
struct Slot {
    name: String,
    icon: String,
    golden: u32,
    universal: i64,
    prints: Vec<(i64, u8, u64)>,
}

/// The slots in the order of their T10 pieces.
fn slot_data(pieces: &[Piece], equipment: &[Item]) -> Vec<Slot> {
    let tops = pieces.iter().filter(|p| p.id < BLUEPRINTS && p.tier == 10);
    tops.filter_map(|top| {
        let slot = |p: &&Piece| p.category == top.category;
        let crafted = pieces.iter().filter(slot).filter(|p| p.id < BLUEPRINTS);
        let recipes: Vec<&(i64, u64)> = crafted.flat_map(|p| &p.recipe).collect();
        let blueprints = pieces.iter().filter(slot).filter(|p| p.id >= BLUEPRINTS);
        let universal = blueprints.clone().find(|p| p.tier == 0)?.id;
        let prints = blueprints
            .filter(|p| (2..=10).contains(&p.tier))
            .map(|p| {
                let per: u64 = recipes.iter().filter(|r| r.0 == p.id).map(|r| r.1).sum();
                (p.id, p.tier, per)
            })
            .collect();
        let entry = |id: i64| equipment.iter().find(|e| e.id == id);
        Some(Slot {
            name: entry(universal)?.name.strip_suffix(UNIVERSAL)?.to_owned(),
            icon: entry(top.id)?.icon.clone(),
            golden: GOLDEN
                .iter()
                .find(|g| g.0 == top.category)
                .map_or(0, |g| g.1),
            universal,
            prints,
        })
    })
    .collect()
}

/// The slots in the order of their T10 pieces, each as `SlotRow` tells.
pub fn slots<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    held: &HashMap<i64, Count, S>,
) -> Vec<SlotRow> {
    slot_data(pieces, equipment)
        .into_iter()
        .map(|slot| {
            let (top, lower): (Vec<&(i64, u8, u64)>, Vec<_>) =
                slot.prints.iter().partition(|p| p.1 == 10);
            // Each blueprint counted in universal ones, the T10 ones in themselves.
            let stock = |worth: &[(i64, u64, u64)]| {
                let (held, about) =
                    worth
                        .iter()
                        .fold((0, false), |(sum, about), &(id, each, _)| {
                            let (n, a) = amount(held.get(&id).copied());
                            (sum + n * each, about || a)
                        });
                let need: u64 = worth.iter().map(|&(_, each, per)| each * per).sum();
                Stock {
                    held,
                    about,
                    need,
                    pieces: if need == 0 {
                        0.0
                    } else {
                        held as f64 / need as f64
                    },
                }
            };
            let top: Vec<(i64, u64, u64)> = top.iter().map(|p| (p.0, 1, p.2)).collect();
            let lower: Vec<(i64, u64, u64)> = std::iter::once((slot.universal, 1, 0))
                .chain(lower.iter().map(|p| (p.0, RATE[usize::from(p.1)], p.2)))
                .collect();
            SlotRow {
                name: slot.name,
                icon: slot.icon,
                golden: slot.golden,
                top: stock(&top),
                universal: stock(&lower),
            }
        })
        .collect()
}

/// The normal missions to play to bring every slot up to the golden ratio,
/// what they give `multiplier` times over (a campaign doubles them).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Farm {
    pub multiplier: u32,
    pub runs: u32,
    pub ap: u32,
    pub stages: Vec<FarmStage>,
}

/// A mission of the farm: its runs and their AP, and the slots it serves.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FarmStage {
    pub name: String,
    pub runs: u32,
    pub ap: u32,
    pub slots: Vec<String>,
}

/// The fewest AP of normal missions that, with the blueprints held, make every
/// slot's golden-ratio pieces from T1 to T10, any tier's blueprints made from
/// the universal ones at `RATE` (a linear program, its runs rounded up, as
/// schale-calc's plan-onia-golden.js plans it); `None` where none can.
pub fn farm<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    missions: &[Mission],
    held: &HashMap<i64, Count, S>,
    multiplier: u32,
) -> Option<Farm> {
    use microlp::{ComparisonOp, OptimizationDirection, Problem};
    let slots = slot_data(pieces, equipment);
    let mut problem = Problem::new(OptimizationDirection::Minimize);
    let runs: Vec<_> = missions
        .iter()
        .map(|m| problem.add_var(f64::from(m.ap), (0.0, f64::INFINITY)))
        .collect();
    let gives = |id: i64| -> Vec<(microlp::Variable, f64)> {
        missions
            .iter()
            .zip(&runs)
            .filter_map(|(m, &run)| {
                let each: f64 = m.drops.iter().filter(|d| d.0 == id).map(|d| d.1).sum();
                (each > 0.0).then_some((run, each * f64::from(multiplier)))
            })
            .collect()
    };
    let has = |id: i64| amount(held.get(&id).copied()).0 as f64;
    for slot in &slots {
        // What the universal blueprints make, against what they hold and drop.
        let mut universal: Vec<(microlp::Variable, f64)> = gives(slot.universal)
            .into_iter()
            .map(|(run, n)| (run, -n))
            .collect();
        for &(id, tier, per) in &slot.prints {
            let made = problem.add_var(0.0, (0.0, f64::INFINITY));
            let mut supply = gives(id);
            supply.push((made, 1.0));
            let want = f64::from(slot.golden) * per as f64 - has(id);
            problem.add_constraint(&supply, ComparisonOp::Ge, want);
            universal.push((made, RATE[usize::from(tier)] as f64));
        }
        problem.add_constraint(&universal, ComparisonOp::Le, has(slot.universal));
    }
    let solution = problem.solve().ok()?.into_solution().ok()?;
    let stages: Vec<FarmStage> = missions
        .iter()
        .zip(&runs)
        .filter_map(|(m, &run)| {
            let count = (solution.var_value(run) - 1e-6).ceil().max(0.0) as u32;
            (count > 0).then(|| FarmStage {
                name: m.name.clone(),
                runs: count,
                ap: count * m.ap,
                slots: slots
                    .iter()
                    .filter(|s| {
                        let ids = s.prints.iter().map(|p| p.0).chain([s.universal]);
                        ids.into_iter().any(|id| m.drops.iter().any(|d| d.0 == id))
                    })
                    .map(|s| s.name.clone())
                    .collect(),
            })
        })
        .collect();
    Some(Farm {
        multiplier,
        runs: stages.iter().map(|s| s.runs).sum(),
        ap: stages.iter().map(|s| s.ap).sum(),
        stages,
    })
}

/// What one student takes of each item at most a full raise of their skills
/// and potential: the EX skill's levels, three skills' and three potentials'
/// (HP, attack, healing); over the students that take an item, the median.
pub fn person_needs(growers: &[Grower], growth: &Growth) -> HashMap<i64, u64> {
    let mut each: HashMap<i64, Vec<u64>> = HashMap::new();
    for grower in growers {
        let mut takes: HashMap<i64, u64> = HashMap::new();
        for &(id, n) in grower.ex.iter().flatten() {
            *takes.entry(id).or_default() += n;
        }
        for &(id, n) in grower.skills.iter().flatten() {
            *takes.entry(id).or_default() += 3 * n;
        }
        if let Some(first) = grower.potential {
            for &(tier, n) in &growth.potential {
                *takes.entry(first + i64::from(tier)).or_default() += 3 * n;
            }
        }
        for (id, n) in takes {
            each.entry(id).or_default().push(n);
        }
    }
    each.into_iter()
        .map(|(id, mut takes)| {
            takes.sort_unstable();
            (id, takes[takes.len() / 2])
        })
        .collect()
}

/// A bounty or a scrimmage: its name and each kind it drops, by tier.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaceRow {
    pub name: String,
    pub kinds: Vec<[Option<HeldCell>; 4]>,
}

/// A good dropped, how many are held, and what one student takes (none for a
/// good no student does).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeldCell {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub held: Option<Count>,
    pub need: Option<u64>,
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
/// `kind` (none for a good of no such kind), in the order first dropped, with
/// what one student `needs`.
pub fn places<S: BuildHasher, T: BuildHasher>(
    places: &[Place],
    entries: &[Item],
    kind: impl Fn(&Item) -> Option<String>,
    held: &HashMap<i64, Count, S>,
    needs: &HashMap<i64, u64, T>,
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
                    need: needs.get(&entry.id).copied(),
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
        let held = HashMap::from([
            (101_009, Count::Exact(270)),
            (101_001, Count::Exact(5)),
            (501_000, Count::Thousands(1)),
        ]);
        let rows = slots(&hat_pieces(), &hat_entries(), &held);
        assert_eq!(rows.len(), 1);
        let hat = &rows[0];
        assert_eq!(
            (hat.name.as_str(), hat.icon.as_str(), hat.golden),
            ("帽子", "icon_1009", 3)
        );
        assert_eq!((hat.top.held, hat.top.need, hat.top.pieces), (270, 60, 4.5));
        // 1,000 universal (about) and 5 of T2's, 2 universal each, against
        // T2's 15 and T10's 20 of T2's.
        let u = &hat.universal;
        assert_eq!((u.held, u.about, u.need), (1010, true, 70));
        assert!((u.pieces - 1010.0 / 70.0).abs() < 1e-9);
    }

    /// A hat's pieces: T2 takes 15 of its own; T10 60 of its own and 20 of T2's.
    fn hat_pieces() -> [Piece; 5] {
        [
            piece(1001, 2, &[(101_001, 15)]),
            piece(1009, 10, &[(101_009, 60), (101_001, 20)]),
            piece(101_001, 2, &[]),
            piece(101_009, 10, &[]),
            piece(501_000, 0, &[]),
        ]
    }

    fn hat_entries() -> [Item; 2] {
        [
            item(1009, "ゲーミングヘルメット", Rarity::N),
            item(501_000, "帽子の万能設計図", Rarity::N),
        ]
    }

    #[test]
    fn the_farm_plays_the_fewest_ap_to_the_golden_ratio() {
        // The golden ratio keeps 3 hats: 180 T10 blueprints and 105 of T2's.
        let missions = [
            Mission {
                name: "29-1".into(),
                ap: 10,
                drops: vec![(101_009, 0.5)],
            },
            Mission {
                name: "25-1".into(),
                ap: 10,
                drops: vec![(501_000, 10.0)],
            },
            Mission {
                name: "4-1".into(),
                ap: 10,
                drops: vec![(501_000, 1.0)],
            },
        ];
        let held = HashMap::from([(101_009, Count::Exact(120))]);
        let farm = farm(&hat_pieces(), &hat_entries(), &missions, &held, 2).unwrap();
        // 60 T10 blueprints at 1 a run, cheaper than 3,000 universal ones; the
        // 105 of T2's from 210 universal ones, 20 a run.
        let runs: Vec<(&str, u32)> = farm
            .stages
            .iter()
            .map(|s| (s.name.as_str(), s.runs))
            .collect();
        assert_eq!(runs, [("29-1", 60), ("25-1", 11)]);
        assert_eq!((farm.runs, farm.ap), (71, 710));
        assert_eq!(farm.stages[0].slots, ["帽子"]);
        // Enough held, nothing to play.
        let held = HashMap::from([(101_009, Count::Exact(180)), (101_001, Count::Exact(105))]);
        assert!(farm_of(&missions, &held).stages.is_empty());
    }

    fn farm_of(missions: &[Mission], held: &HashMap<i64, Count>) -> Farm {
        farm(&hat_pieces(), &hat_entries(), missions, held, 2).unwrap()
    }

    #[test]
    fn a_person_takes_the_median_raise_of_the_students_that_take_an_item() {
        let grower = |id, ex: Vec<(i64, u64)>, skills: Vec<(i64, u64)>, potential| Grower {
            id,
            equipment: Vec::new(),
            ex: vec![ex],
            skills: vec![skills],
            potential,
        };
        let growth = Growth {
            potential: vec![(0, 10), (1, 6)],
            ..crate::plan::tests::growth()
        };
        let growers = [
            // Three skills of 1 note; three potentials of 10 and 6 ooparts.
            grower(1, vec![(3030, 2)], vec![(4030, 1)], Some(150)),
            grower(2, Vec::new(), vec![(150, 5)], Some(150)),
            grower(3, Vec::new(), vec![(4030, 1)], None),
        ];
        let needs = person_needs(&growers, &growth);
        assert_eq!(needs[&4030], 3);
        assert_eq!(needs[&3030], 2);
        // 30 and 45 of the first tier, 18 of the second.
        assert_eq!((needs[&150], needs[&151]), (45, 18));
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
        let needs = HashMap::from([(3030, 40)]);
        let rows = places(&[place], &entries, kind, &held, &needs);
        assert_eq!(rows[0].kinds[1][0].as_ref().unwrap().need, Some(40));
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
