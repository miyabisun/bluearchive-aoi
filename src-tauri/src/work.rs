//! The work tab: what the daily plays give, against what is held. The
//! blueprints of the normal missions by equipment slot, the drops of the
//! bounties and of the scrimmages, and how many students the credits and the
//! reports raise.

use std::{collections::HashMap, hash::BuildHasher};

use serde::{Deserialize, Serialize};

use crate::{
    Item,
    icon::Rarity,
    plan::{BLUEPRINTS, Grower, Growth, Piece, RATE, UNIVERSAL},
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
    pub orbs: OrbRow,
    /// The scrimmages' schools, as `scrimmages` lists them.
    pub schools: Vec<SchoolRow>,
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

impl Slot {
    /// The two columns: the T10 blueprints, and apart the universal ones with
    /// those of T9 and below counted at `RATE`; each blueprint with what it
    /// counts for and what one piece from T1 to T10 takes of it.
    fn columns(&self) -> [Vec<(i64, u64, u64)>; 2] {
        let (top, lower): (Vec<&(i64, u8, u64)>, Vec<_>) =
            self.prints.iter().partition(|p| p.1 == 10);
        [
            top.iter().map(|p| (p.0, 1, p.2)).collect(),
            std::iter::once((self.universal, 1, 0))
                .chain(lower.iter().map(|p| (p.0, RATE[usize::from(p.1)], p.2)))
                .collect(),
        ]
    }
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
            let [top, lower] = slot.columns();
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

/// How many missions the farm ranks to play past the ratio.
const NEXT: usize = 3;

/// The normal missions to play to bring every slot up to the golden ratio,
/// what they give `multiplier` times over (a campaign doubles them); and the
/// missions to play past it, best first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Farm {
    pub multiplier: u32,
    pub runs: u32,
    pub ap: u32,
    pub stages: Vec<FarmStage>,
    pub next: Vec<NextStage>,
}

/// A mission of the farm: its runs and their AP, and the slots it serves, the
/// one it drops most of first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FarmStage {
    pub name: String,
    pub runs: u32,
    pub ap: u32,
    pub slots: Vec<String>,
}

/// A mission to play past the ratio, and the slots it serves, the one it
/// drops most of first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NextStage {
    pub name: String,
    pub slots: Vec<String>,
}

/// The normal missions to play, in the user's three steps of 2026-10-14: the
/// fewest AP to the golden ratio's T10 blueprints, of those the runs worth
/// most (see `Farming::worth`); then the fewest AP to T9 and below's, the
/// universal blueprints making any of those tiers at `RATE`; and past the
/// ratio, the missions dropping T10 blueprints worth most, to even the stock
/// out. Linear programs, the runs rounded up (as schale-calc's
/// plan-onia-golden.js plans them); `None` where none can.
pub fn farm<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    missions: &[Mission],
    held: &HashMap<i64, Count, S>,
    multiplier: u32,
) -> Option<Farm> {
    let mut farming = Farming {
        slots: slot_data(pieces, equipment),
        missions,
        stock: held
            .iter()
            .map(|(&id, &count)| (id, amount(Some(count)).0 as f64))
            .collect(),
        multiplier,
    };
    let top = farming.least(true)?;
    farming.add(&top);
    let lower = farming.least(false)?;
    farming.add(&lower);
    let stages: Vec<FarmStage> = missions
        .iter()
        .zip(top.iter().zip(&lower).map(|(a, b)| a + b))
        .filter(|(_, count)| *count > 0)
        .map(|(m, count)| FarmStage {
            name: m.name.clone(),
            runs: count,
            ap: count * m.ap,
            slots: served(&farming.slots, m),
        })
        .collect();
    let mut next: Vec<(f64, &Mission)> = missions
        .iter()
        .map(|m| (farming.worth(m), m))
        .filter(|(worth, _)| *worth > 0.0)
        .collect();
    next.sort_by(|a, b| b.0.total_cmp(&a.0));
    Some(Farm {
        multiplier,
        runs: stages.iter().map(|s| s.runs).sum(),
        ap: stages.iter().map(|s| s.ap).sum(),
        stages,
        next: next
            .into_iter()
            .take(NEXT)
            .map(|(_, m)| NextStage {
                name: m.name.clone(),
                slots: served(&farming.slots, m),
            })
            .collect(),
    })
}

/// The slots `mission` drops blueprints of, the one it drops most of first (a
/// mission drops one slot a third more than the others).
fn served(slots: &[Slot], mission: &Mission) -> Vec<String> {
    let mut served: Vec<(f64, &str)> = slots
        .iter()
        .filter_map(|s| {
            let ids: Vec<i64> = s.prints.iter().map(|p| p.0).chain([s.universal]).collect();
            let drops = mission.drops.iter().filter(|d| ids.contains(&d.0));
            let total: f64 = drops.map(|d| d.1).sum();
            (total > 0.0).then_some((total, s.name.as_str()))
        })
        .collect();
    served.sort_by(|a, b| b.0.total_cmp(&a.0));
    served
        .into_iter()
        .map(|(_, name)| name.to_owned())
        .collect()
}

/// A variable of runs for each mission, each run counting as `by_run` has it
/// in the objective.
fn runs(problem: &mut microlp::Problem, by_run: &[f64]) -> Vec<microlp::Variable> {
    let var = |&cost| problem.add_var(cost, (0.0, f64::INFINITY));
    by_run.iter().map(var).collect()
}

/// The runs with what each counts for, those counting for nothing left out.
fn terms(runs: &[microlp::Variable], by_run: &[f64]) -> Vec<(microlp::Variable, f64)> {
    let all = runs.iter().zip(by_run);
    all.filter(|(_, n)| **n != 0.0)
        .map(|(&run, &n)| (run, n))
        .collect()
}

/// What the farm plans with: the slots, the missions, the blueprints held with
/// what the runs planned so far give, and how many times over the missions
/// give.
struct Farming<'a> {
    slots: Vec<Slot>,
    missions: &'a [Mission],
    stock: HashMap<i64, f64>,
    multiplier: u32,
}

impl Farming<'_> {
    fn has(&self, id: i64) -> f64 {
        self.stock.get(&id).copied().unwrap_or(0.0)
    }

    /// What a run of each mission gives of `id`.
    fn each(&self, id: i64) -> Vec<f64> {
        let drops = |m: &Mission| {
            m.drops
                .iter()
                .filter(|d| d.0 == id)
                .map(|d| d.1)
                .sum::<f64>()
        };
        let times = f64::from(self.multiplier);
        self.missions.iter().map(|m| drops(m) * times).collect()
    }

    /// Adds to the stock what `counts` runs of each mission give.
    fn add(&mut self, counts: &[u32]) {
        let times = f64::from(self.multiplier);
        for (mission, &count) in self.missions.iter().zip(counts) {
            for &(id, rate) in &mission.drops {
                *self.stock.entry(id).or_default() += rate * times * f64::from(count);
            }
        }
    }

    /// What a run of `mission` is worth to the T10 blueprints: each it drops
    /// over how many times its slot has reached the ratio (at least once), so
    /// one twice over the ratio counts half (the user's of 2026-10-14).
    fn worth(&self, mission: &Mission) -> f64 {
        let times = f64::from(self.multiplier);
        let tops = self.slots.iter().flat_map(|slot| {
            let tops = slot.prints.iter().filter(|p| p.1 == 10);
            tops.map(move |&(id, _, per)| (id, f64::from(slot.golden) * per as f64))
        });
        tops.filter(|&(_, need)| need > 0.0)
            .map(|(id, need)| {
                let rate: f64 = mission
                    .drops
                    .iter()
                    .filter(|d| d.0 == id)
                    .map(|d| d.1)
                    .sum();
                rate * times / (self.has(id) / need).max(1.0)
            })
            .sum()
    }

    /// The fewest runs to the ratio's T10 blueprints (`top`), of those the
    /// runs worth most; or to T9 and below's. Rounded up.
    fn least(&self, top: bool) -> Option<Vec<u32>> {
        use microlp::{
            ComparisonOp::Le,
            OptimizationDirection::{Maximize, Minimize},
            Problem,
        };
        let ap: Vec<f64> = self.missions.iter().map(|m| f64::from(m.ap)).collect();
        let mut least = Problem::new(Minimize);
        let now = runs(&mut least, &ap);
        self.reach(&mut least, &now, top);
        let solution = least.solve().ok()?.into_solution().ok()?;
        let mut counts: Vec<f64> = now.iter().map(|&run| solution.var_value(run)).collect();
        if top {
            let worth: Vec<f64> = self.missions.iter().map(|m| self.worth(m)).collect();
            let mut best = Problem::new(Maximize);
            let now = runs(&mut best, &worth);
            let most = solution.objective();
            best.add_constraint(terms(&now, &ap), Le, most + 1e-9 * most.max(1.0));
            self.reach(&mut best, &now, true);
            if let Some(solution) = best.solve().ok().and_then(|s| s.into_solution().ok()) {
                counts = now.iter().map(|&run| solution.var_value(run)).collect();
            }
        }
        let round = |count: f64| (count - 1e-4).ceil().max(0.0) as u32;
        Some(counts.into_iter().map(round).collect())
    }

    /// Asks that the stock and what `runs` give make the ratio's T10
    /// blueprints (`top`), or those of T9 and below, the universal ones making
    /// any of them.
    fn reach(&self, problem: &mut microlp::Problem, runs: &[microlp::Variable], top: bool) {
        use microlp::ComparisonOp::{Ge, Le};
        for slot in &self.slots {
            // What the universal blueprints make, against what they hold and drop.
            let given: Vec<f64> = self.each(slot.universal).iter().map(|n| -n).collect();
            let mut universal = terms(runs, &given);
            for &(id, tier, per) in slot.prints.iter().filter(|p| (p.1 == 10) == top) {
                let mut supply = terms(runs, &self.each(id));
                if !top {
                    let made = problem.add_var(0.0, (0.0, f64::INFINITY));
                    supply.push((made, 1.0));
                    universal.push((made, RATE[usize::from(tier)] as f64));
                }
                let want = f64::from(slot.golden) * per as f64 - self.has(id);
                problem.add_constraint(supply, Ge, want);
            }
            if !top {
                problem.add_constraint(universal, Le, self.has(slot.universal));
            }
        }
    }
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
    let (report_exp, report_about) = exp_held(reports, held);
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

/// The exp of the `kinds` (item, exp of one) held, and whether only about.
fn exp_held<S: BuildHasher>(kinds: &[(i64, u64)], held: &HashMap<i64, Count, S>) -> (u64, bool) {
    kinds.iter().fold((0, false), |(sum, about), &(id, exp)| {
        let (n, a) = amount(held.get(&id).copied());
        (sum + n * exp, about || a)
    })
}

/// How many `each` make of `held`.
fn per(held: u64, each: u64) -> f64 {
    if each == 0 {
        0.0
    } else {
        held as f64 / each as f64
    }
}

/// A school's scrimmage: the unique weapon parts it drops, how much more exp
/// they give the weapons they suit (1.5 for 15,000 per 10,000), and the share
/// of students whose weapons they suit.
pub struct School {
    pub drops: Vec<i64>,
    pub rate: f64,
    pub share: f64,
}

/// A school's parts held against the unique weapons they raise: their exp
/// (whether only about), the rate the weapons they suit take it at, the exp a
/// weapon takes from Lv1 to its cap and how many weapons the parts raise; and
/// whether the school's parts run out first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SchoolRow {
    pub exp: u64,
    pub about: bool,
    pub rate: f64,
    pub exp_each: u64,
    pub students: f64,
    pub first: bool,
}

/// The `schools`' parts (`parts`, `(id, exp of one)`) as `SchoolRow` tells, a
/// weapon taking `each`; first the school with the fewest weapons for its
/// share of students (the user's of 2026-10-16: 「一番優先順位の高い学園を赤字で
/// 知らせる」).
pub fn school_rows<S: BuildHasher>(
    schools: &[School],
    parts: &[(i64, u64)],
    each: u64,
    held: &HashMap<i64, Count, S>,
) -> Vec<SchoolRow> {
    let mut rows: Vec<SchoolRow> = schools
        .iter()
        .map(|school| {
            let dropped: Vec<(i64, u64)> = parts
                .iter()
                .copied()
                .filter(|(id, _)| school.drops.contains(id))
                .collect();
            let (exp, about) = exp_held(&dropped, held);
            SchoolRow {
                exp,
                about,
                rate: school.rate,
                exp_each: each,
                students: per(exp, each) * school.rate,
                first: false,
            }
        })
        .collect();
    let first = rows
        .iter()
        .zip(schools)
        .enumerate()
        .filter(|(_, (_, school))| school.share > 0.0)
        .min_by(|(_, (a, x)), (_, (b, y))| {
            (a.students / x.share).total_cmp(&(b.students / y.share))
        })
        .map(|(i, _)| i);
    if let Some(i) = first {
        rows[i].first = true;
    }
    rows
}

/// The enhancement orbs held against what one student's equipment takes:
/// their exp, whether only about, the exp a student takes and how many
/// students the orbs raise.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OrbRow {
    pub exp: u64,
    pub about: bool,
    pub exp_each: u64,
    pub students: f64,
}

/// The orb exp a student's three slots take from T1 to T10: every tier from
/// Lv1 to its top (`tops`, tier by tier), fed in orbs (see `plan::fed`): the
/// user's measure of 2026-10-16, ten times from Lv1 to the top.
pub fn orbs_each(equipment_exp: &[Vec<u64>], tops: &[u8], orbs: &[(i64, u64)]) -> u64 {
    let slot: u64 = equipment_exp
        .iter()
        .zip(tops)
        .map(|(exp, &top)| crate::plan::fed(exp[usize::from(top) - 1], orbs))
        .sum();
    3 * slot
}

/// The orbs (`(item, exp of one)`) as `OrbRow` tells, a student taking `each`.
pub fn orbs<S: BuildHasher>(
    orbs: &[(i64, u64)],
    each: u64,
    held: &HashMap<i64, Count, S>,
) -> OrbRow {
    let (exp, about) = exp_held(orbs, held);
    OrbRow {
        exp,
        about,
        exp_each: each,
        students: per(exp, each),
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
        // 60 T10 blueprints at 1 a run; the 105 of T2's from 210 universal
        // ones, 20 a run.
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

    #[test]
    fn the_farm_keeps_the_universal_blueprints_off_t10() {
        let missions = [Mission {
            name: "29-1".into(),
            ap: 10,
            drops: vec![(101_009, 0.5)],
        }];
        // The universal ones would make 2,000 T10 blueprints; the farm plays
        // for all 180 instead.
        let held = HashMap::from([(501_000, Count::Exact(100_000))]);
        let runs: Vec<(String, u32)> = farm_of(&missions, &held)
            .stages
            .into_iter()
            .map(|s| (s.name, s.runs))
            .collect();
        assert_eq!(runs, [("29-1".to_owned(), 180)]);
    }

    #[test]
    fn of_missions_alike_the_farm_plays_the_one_whose_drops_the_ratio_lacks_most() {
        // Hats from none to 3; bags 8 times over 2, shoes just over 4.
        let (pieces, entries) = three_slots();
        let mission = |name: &str, other: i64| Mission {
            name: name.into(),
            ap: 10,
            drops: vec![(101_009, 0.5), (other, 0.5)],
        };
        let missions = [mission("28-4", 104_009), mission("29-2", 103_009)];
        let held = HashMap::from([(104_009, Count::Exact(1000)), (103_009, Count::Exact(250))]);
        let farm = farm(&pieces, &entries, &missions, &held, 2).unwrap();
        let runs: Vec<(&str, u32)> = farm
            .stages
            .iter()
            .map(|s| (s.name.as_str(), s.runs))
            .collect();
        assert_eq!(runs, [("29-2", 180)]);
    }

    #[test]
    fn past_the_ratio_the_farm_ranks_three_t10_missions_by_their_drops_over_the_ratio_reached() {
        let (pieces, entries) = three_slots();
        let mission = |name: &str, drops: &[i64]| Mission {
            name: name.into(),
            ap: 10,
            drops: drops.iter().map(|&id| (id, 0.5)).collect(),
        };
        let missions = [
            mission("28-4", &[101_009, 104_009]),
            mission("29-2", &[101_009, 103_009]),
            mission("29-8", &[104_009]),
            mission("29-9", &[104_009, 103_009]),
            // No T10 blueprints: nothing to rank.
            mission("25-1", &[501_000]),
        ];
        // Hats at the ratio, bags 8 times over it, shoes twice: a hat counts
        // whole, a shoe half, a bag an eighth.
        let held = HashMap::from([
            (101_009, Count::Exact(180)),
            (104_009, Count::Exact(1000)),
            (103_009, Count::Exact(480)),
        ]);
        let farm = farm(&pieces, &entries, &missions, &held, 2).unwrap();
        assert!(farm.stages.is_empty());
        let next: Vec<&str> = farm.next.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(next, ["29-2", "28-4", "29-9"]);
        assert_eq!(farm.next[0].slots, ["帽子", "靴"]);
    }

    /// Hats, bags and shoes, of T10 pieces only.
    fn three_slots() -> (Vec<Piece>, Vec<Item>) {
        let (mut pieces, mut entries) = (Vec::new(), Vec::new());
        for (category, name, n) in [("Hat", "帽子", 1), ("Bag", "カバン", 4), ("Shoes", "靴", 3)]
        {
            let (p, e) = t10_slot(category, name, n);
            pieces.extend(p);
            entries.extend(e);
        }
        (pieces, entries)
    }

    /// A slot's T10 piece, taking 60 of its T10 blueprints, and its universal
    /// blueprints; `n` its place among the slots (1 the hat).
    fn t10_slot(category: &str, name: &str, n: i64) -> (Vec<Piece>, Vec<Item>) {
        let top = n * 1000 + 9;
        let in_slot = |piece: Piece| Piece {
            category: category.into(),
            ..piece
        };
        let pieces = vec![
            in_slot(piece(top, 10, &[(100_000 + top, 60)])),
            in_slot(piece(100_000 + top, 10, &[])),
            in_slot(piece(500_000 + n * 1000, 0, &[])),
        ];
        let entries = vec![
            item(top, name, Rarity::N),
            item(
                500_000 + n * 1000,
                &format!("{name}の万能設計図"),
                Rarity::N,
            ),
        ];
        (pieces, entries)
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
            weapon: None,
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

    /// A school's scrimmage parts raise so many unique weapons from Lv1 to
    /// their cap, `rate` times over for the weapons they suit; the school whose
    /// parts run out first, for the share of students they suit, comes first.
    #[test]
    fn the_schools_count_the_weapons_their_parts_raise_the_first_to_run_out_marked() {
        let parts = [(10, 10), (11, 50), (20, 10), (21, 50), (30, 10)];
        let school = |drops: Vec<i64>, share: f64| School {
            drops,
            rate: 1.5,
            share,
        };
        let schools = [
            school(vec![10, 11], 0.5),
            school(vec![20, 21], 0.25),
            school(vec![30], 0.25),
        ];
        let held = HashMap::from([
            (10, Count::Exact(100)),
            (11, Count::Exact(20)),
            (20, Count::Exact(100)),
            (21, Count::Thousands(1)),
            (30, Count::Exact(150)),
        ]);
        let rows = school_rows(&schools, &parts, 1000, &held);
        let told: Vec<(u64, bool, f64, bool)> = rows
            .iter()
            .map(|r| (r.exp, r.about, r.students, r.first))
            .collect();
        // 2,000 exp × 1.5 makes 3 weapons for half the students: 6 a share;
        // the others 306 and 9 a share.
        assert_eq!(
            told,
            [
                (2_000, false, 3.0, true),
                (51_000, true, 76.5, false),
                (1_500, false, 2.25, false),
            ]
        );
        assert_eq!((rows[0].rate, rows[0].exp_each), (1.5, 1000));
    }

    /// A student's three slots each level every tier from Lv1 to its top, fed
    /// in orbs; the orbs held count as so many students.
    #[test]
    fn the_orbs_count_the_students_whose_equipment_they_raise() {
        // Tier 1 takes 15 exp to its top, tier 2 50, fed 10 at a time.
        let each = orbs_each(
            &[vec![0, 5, 15], vec![0, 20, 50]],
            &[3, 3],
            &[(1, 10), (2, 40)],
        );
        assert_eq!(each, 3 * (20 + 50));
        let held = HashMap::from([(1, Count::Exact(21)), (2, Count::Exact(10))]);
        let row = orbs(&[(1, 10), (2, 40)], each, &held);
        assert_eq!((row.exp, row.about, row.exp_each), (210 + 400, false, 210));
        assert!((row.students - 610.0 / 210.0).abs() < 1e-9);
    }
}
