//! The work tab: what the daily plays give, against what is held. The
//! blueprints of the normal missions by equipment slot, the drops of the
//! bounties and of the scrimmages, and how many students the credits and the
//! reports raise.

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

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
    #[serde(default)]
    pub hard: Vec<HardMission>,
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

/// A hard mission: its name ("18-3"), its AP and the items a run may drop,
/// a student's eleph among them.
#[derive(Debug, Clone, Deserialize)]
pub struct HardMission {
    pub name: String,
    pub ap: u32,
    pub drops: Vec<i64>,
}

/// The hard missions' reservations as `strategy::Reserve` keeps them: their
/// shop and their coin (the user's of 2026-10-22: 「予約中情報をYAMLに追加」).
pub const HARD: &str = "任務 Hard";
pub const AP: &str = "AP";

/// How many times a day a hard mission may be played (the game's rule).
pub const HARD_RUNS: u32 = 3;

/// A student as the hard missions' cards order them: their name, the stars
/// they come with (1 to 3) and their star now (`plan::star_of`, `None` unread).
pub struct HardStudent {
    pub name: String,
    pub first: u8,
    pub now: Option<u8>,
}

/// A student's eleph the hard missions drop, as the work tab's card shows it:
/// the eleph, the missions dropping it, a day's AP playing each of them
/// `HARD_RUNS` times, the student's star now (`None` unread), how many are
/// held, whether the weapon's fourth is reached and whether the card is
/// reserved.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HardCard {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub stages: Vec<String>,
    pub ap: u32,
    pub star: Option<u8>,
    pub held: Option<Count>,
    pub done: bool,
    pub reserved: bool,
}

/// The `elephs` the hard `missions` drop, a card a student (`student` by the
/// eleph's id, which is the student's), in the user's order of 2026-10-22:
/// those who come with three stars first, those with one or two after, each by
/// name, and those at the weapon's fourth last, greyed.
pub fn hard<'a, S: BuildHasher>(
    missions: &[HardMission],
    eleph: impl Fn(i64) -> Option<&'a Item>,
    student: impl Fn(i64) -> HardStudent,
    held: &HashMap<i64, Count, S>,
    reserved: impl Fn(i64) -> bool,
) -> Vec<HardCard> {
    // Each card with what orders it: done, not of three stars, the name.
    let mut cards: Vec<(HardCard, (bool, bool, String))> = Vec::new();
    for mission in missions {
        let day = mission.ap * HARD_RUNS;
        for entry in mission.drops.iter().filter_map(|&id| eleph(id)) {
            if let Some((card, _)) = cards.iter_mut().find(|(c, _)| c.id == entry.id) {
                card.stages.push(mission.name.clone());
                card.ap += day;
                continue;
            }
            let who = student(entry.id);
            let count = held.get(&entry.id).copied();
            let done = who.now == Some(crate::plan::TOP_STAR);
            let card = HardCard {
                id: entry.id,
                name: entry.name.clone(),
                icon: entry.icon.clone(),
                stages: vec![mission.name.clone()],
                ap: day,
                star: who.now,
                held: count,
                done,
                reserved: reserved(entry.id),
            };
            cards.push((card, (done, who.first != 3, who.name)));
        }
    }
    cards.sort_by(|a, b| a.1.cmp(&b.1));
    cards.into_iter().map(|(card, _)| card).collect()
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
/// keeps of it (and why), and what the blueprints held make of pieces (the T10
/// blueprints, and apart the universal blueprints with those of T9 and below
/// counted at `RATE`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotRow {
    pub name: String,
    pub icon: String,
    pub golden: Golden,
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

/// How many students' pieces the golden ratio keeps in all, as オニア先生's
/// (schale-calc's onia-golden-ratio.md) did.
const GOLDEN_STUDENTS: u64 = 24;

/// The golden ratio, by slot (its category): how many students' pieces from
/// T1 to T10 to keep, the slot's share of the slots of every student released
/// (`growers`) in `GOLDEN_STUDENTS` students (the user's of 2026-10-17: 「黄金比
/// は全実装生徒の装備の割合を元に算出した数値」), in whole students (see
/// `whole_students`).
pub fn golden(growers: &[Grower]) -> BTreeMap<String, Golden> {
    let mut slots: BTreeMap<String, u64> = BTreeMap::new();
    for slot in growers.iter().flat_map(|g| &g.equipment) {
        *slots.entry(slot.clone()).or_default() += 1;
    }
    let all = slots.values().sum();
    slots
        .into_iter()
        .map(|(slot, slots)| {
            let whole = whole_students(slots * GOLDEN_STUDENTS, all);
            let golden = Golden {
                students: u32::try_from(whole).unwrap_or(u32::MAX),
                slots,
                all,
                total: GOLDEN_STUDENTS,
            };
            (slot, golden)
        })
        .collect()
}

/// A slot's golden ratio: how many students' pieces to keep, and why: the
/// slot is `slots` of the `all` slots of every student, in `total` students.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Golden {
    pub students: u32,
    pub slots: u64,
    pub all: u64,
    pub total: u64,
}

/// `part / all` students, rounded up from a quarter on (the user's of
/// 2026-10-17: 「0.24以下: 切り捨てる」「0.25以上: 切り上げる」): a stock short of
/// a student fails, and a stock of 2.5 students' blueprints is not enough.
fn whole_students(part: u64, all: u64) -> u64 {
    if all == 0 {
        return 0;
    }
    part / all + u64::from(part % all * 4 >= all)
}

/// A slot's blueprints: its universal one, and each tier's with what one piece
/// from T1 to T10 takes of it; the slot's category ("Hat") and name ("帽子"),
/// and the icon of its T10 piece.
pub(crate) struct Slot {
    pub(crate) category: String,
    pub(crate) name: String,
    pub(crate) icon: String,
    golden: Golden,
    pub(crate) universal: i64,
    pub(crate) prints: Vec<(i64, u8, u64)>,
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
pub(crate) fn slot_data(
    pieces: &[Piece],
    equipment: &[Item],
    golden: &BTreeMap<String, Golden>,
) -> Vec<Slot> {
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
            category: top.category.clone(),
            name: entry(universal)?.name.strip_suffix(UNIVERSAL)?.to_owned(),
            icon: entry(top.id)?.icon.clone(),
            golden: golden.get(&top.category).copied().unwrap_or_default(),
            universal,
            prints,
        })
    })
    .collect()
}

/// The slots in the order of their T10 pieces, each as `SlotRow` tells, of the
/// `golden` ratio (see `golden`).
pub fn slots<S: BuildHasher>(
    pieces: &[Piece],
    equipment: &[Item],
    golden: &BTreeMap<String, Golden>,
    held: &HashMap<i64, Count, S>,
) -> Vec<SlotRow> {
    slot_data(pieces, equipment, golden)
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
    golden: &BTreeMap<String, Golden>,
    missions: &[Mission],
    held: &HashMap<i64, Count, S>,
    multiplier: u32,
) -> Option<Farm> {
    let mut farming = Farming {
        slots: slot_data(pieces, equipment, golden),
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
            tops.map(move |&(id, _, per)| (id, f64::from(slot.golden.students) * per as f64))
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
                let want = f64::from(slot.golden.students) * per as f64 - self.has(id);
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

impl HeldCell {
    /// `entry` with how many are held and what one student `needs` of it.
    pub fn of<S: BuildHasher, T: BuildHasher>(
        entry: &Item,
        held: &HashMap<i64, Count, S>,
        needs: &HashMap<i64, u64, T>,
    ) -> Self {
        Self {
            id: entry.id,
            name: entry.name.clone(),
            icon: entry.icon.clone(),
            held: held.get(&entry.id).copied(),
            need: needs.get(&entry.id).copied(),
        }
    }
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

/// The `entries` of the kinds that come in tiers grouped by `kind` (none for
/// an entry of no such kind), in the order first seen, each with its tiers by
/// rarity: how many are held and what one student `needs`.
pub fn by_tier<'a, S: BuildHasher, T: BuildHasher>(
    entries: impl IntoIterator<Item = &'a Item>,
    kind: impl Fn(&Item) -> Option<String>,
    held: &HashMap<i64, Count, S>,
    needs: &HashMap<i64, u64, T>,
) -> Vec<(String, [Option<HeldCell>; 4])> {
    let mut kinds: Vec<(String, [Option<HeldCell>; 4])> = Vec::new();
    for entry in entries {
        let (Some(key), Some(rarity)) = (kind(entry), entry.rarity) else {
            continue;
        };
        let found = kinds.iter().position(|(k, _)| *k == key);
        let at = found.unwrap_or_else(|| {
            kinds.push((key, Default::default()));
            kinds.len() - 1
        });
        kinds[at].1[tier(rarity)] = Some(HeldCell::of(entry, held, needs));
    }
    kinds
}

/// The places with what they drop of the kinds that come in tiers, grouped by
/// `kind` (see `by_tier`), in the order first dropped.
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
            let dropped = place
                .drops
                .iter()
                .filter_map(|id| entries.iter().find(|e| e.id == *id));
            PlaceRow {
                name: place.name.clone(),
                kinds: by_tier(dropped, &kind, held, needs)
                    .into_iter()
                    .map(|(_, tiers)| tiers)
                    .collect(),
            }
        })
        .collect()
}

/// The commissions against what raising a student from Lv1 takes: the
/// credits held over a whole training's, and the exp of the reports held over
/// Lv1 to 90's; the reports held by tier, as the enhancement materials show
/// theirs (the user's of 2026-10-29: 「レポートを強化素材風の表記にする」).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommissionRow {
    pub credits_name: String,
    pub credits: Option<u64>,
    pub credits_each: u64,
    pub credit_students: Option<f64>,
    pub reports_name: String,
    pub report_tiers: [Option<HeldCell>; 4],
    pub report_exp: u64,
    pub report_about: bool,
    pub exp_each: u64,
    pub report_students: f64,
}

/// The commissions as `CommissionRow` tells, the reports' exp each by id and
/// their tiers among `entries`.
pub fn commissions<S: BuildHasher>(
    names: &Commissions,
    credits: Option<u64>,
    (credits_each, exp_each): (u64, u64),
    reports: &[(i64, u64)],
    entries: &[Item],
    held: &HashMap<i64, Count, S>,
) -> CommissionRow {
    let (report_exp, report_about) = exp_held(reports, held);
    let report_tiers = by_tier(
        entries
            .iter()
            .filter(|e| reports.iter().any(|(id, _)| *id == e.id)),
        |_| Some(String::new()),
        held,
        &HashMap::<i64, u64>::new(),
    )
    .into_iter()
    .next()
    .map(|(_, tiers)| tiers)
    .unwrap_or_default();
    CommissionRow {
        credits_name: names.credits.clone(),
        credits,
        credits_each,
        credit_students: credits.map(|c| per(c, credits_each)),
        reports_name: names.reports.clone(),
        report_tiers,
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
/// weapon takes from Lv1 to its cap and how many weapons the parts raise; the
/// share of students whose weapons they suit, and whether the school's parts
/// run out first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SchoolRow {
    pub exp: u64,
    pub about: bool,
    pub rate: f64,
    pub exp_each: u64,
    pub students: f64,
    pub share: f64,
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
            let raised = raise(&dropped, school.rate, each, held);
            SchoolRow {
                exp: raised.exp,
                about: raised.about,
                rate: raised.rate,
                exp_each: raised.exp_each,
                students: raised.students,
                share: school.share,
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

/// Materials held against what one student takes of them: their exp, whether
/// only about, the rate it is taken at, the exp a student takes and how many
/// students the materials raise.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Raise {
    pub exp: u64,
    pub about: bool,
    pub rate: f64,
    pub exp_each: u64,
    pub students: f64,
}

/// The `materials` (`(id, exp of one)`) held as `Raise` tells, taken at
/// `rate`, a student taking `each`.
pub fn raise<S: BuildHasher>(
    materials: &[(i64, u64)],
    rate: f64,
    each: u64,
    held: &HashMap<i64, Count, S>,
) -> Raise {
    let (exp, about) = exp_held(materials, held);
    Raise {
        exp,
        about,
        rate,
        exp_each: each,
        students: per(exp, each) * rate,
    }
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
        let rows = slots(&hat_pieces(), &hat_entries(), &ratio(), &held);
        assert_eq!(rows.len(), 1);
        let hat = &rows[0];
        assert_eq!(
            (hat.name.as_str(), hat.icon.as_str(), hat.golden.students),
            ("帽子", "icon_1009", 3)
        );
        assert_eq!((hat.top.held, hat.top.need, hat.top.pieces), (270, 60, 4.5));
        // 1,000 universal (about) and 5 of T2's, 2 universal each, against
        // T2's 15 and T10's 20 of T2's.
        let u = &hat.universal;
        assert_eq!((u.held, u.about, u.need), (1010, true, 70));
        assert!((u.pieces - 1010.0 / 70.0).abs() < 1e-9);
    }

    /// A golden ratio of 3 hats, 2 bags and 4 shoes.
    fn ratio() -> BTreeMap<String, Golden> {
        let students = |students| Golden {
            students,
            ..Golden::default()
        };
        BTreeMap::from([
            ("Hat".to_owned(), students(3)),
            ("Bag".to_owned(), students(2)),
            ("Shoes".to_owned(), students(4)),
        ])
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
        let farm = farm(&hat_pieces(), &hat_entries(), &ratio(), &missions, &held, 2).unwrap();
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
        let farm = farm(&pieces, &entries, &ratio(), &missions, &held, 2).unwrap();
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
        let farm = farm(&pieces, &entries, &ratio(), &missions, &held, 2).unwrap();
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
        farm(&hat_pieces(), &hat_entries(), &ratio(), missions, held, 2).unwrap()
    }

    /// A share is rounded up from a quarter on, down below it.
    #[test]
    fn a_share_of_students_rounds_up_from_a_quarter() {
        assert_eq!(whole_students(624, 100), 6);
        assert_eq!(whole_students(625, 100), 7);
        assert_eq!(whole_students(600, 100), 6);
        assert_eq!(whole_students(699, 100), 7);
    }

    /// Each slot keeps its share of the students' slots in 24 students: of 10
    /// students' 30 slots, 10 hats are 8, 6 shoes 4.8 (5), 9 watches 7.2 (7)
    /// and 5 bags 4.
    #[test]
    fn the_golden_ratio_is_each_slots_share_of_every_students_in_24() {
        let student = |slots: [&str; 3]| Grower {
            id: 0,
            equipment: slots.iter().map(|&s| s.to_owned()).collect(),
            ex: Vec::new(),
            skills: Vec::new(),
            potential: None,
            weapon: None,
            star: 3,
        };
        let growers: Vec<Grower> = [
            [["Hat", "Shoes", "Watch"]; 5].to_vec(),
            [["Hat", "Bag", "Watch"]; 4].to_vec(),
            vec![["Hat", "Shoes", "Bag"]],
        ]
        .concat()
        .into_iter()
        .map(student)
        .collect();
        let ratio = |students, slots| Golden {
            students,
            slots,
            all: 30,
            total: 24,
        };
        assert_eq!(
            golden(&growers),
            BTreeMap::from([
                ("Bag".to_owned(), ratio(4, 5)),
                ("Hat".to_owned(), ratio(8, 10)),
                ("Shoes".to_owned(), ratio(5, 6)),
                ("Watch".to_owned(), ratio(7, 9)),
            ])
        );
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
            star: 3,
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
        let reports = [
            item(10, "初級レポート", Rarity::N),
            item(11, "中級レポート", Rarity::R),
            item(13, "最上級レポート", Rarity::SSR),
            item(20, "錆びたスプリング", Rarity::N),
        ];
        let row = commissions(
            &names,
            Some(250_000_000),
            (50_000_000, 1_000_000),
            &[(10, 50), (11, 500), (13, 10_000)],
            &reports,
            &held,
        );
        assert_eq!(row.credit_students, Some(5.0));
        // 1,000 × 50 + 2,000 × 10,000 (about).
        assert_eq!((row.report_exp, row.report_about), (20_050_000, true));
        assert!((row.report_students - 20.05).abs() < 1e-9);
        // The reports by tier as the scrimmages' parts: held, or none read.
        let tiers: Vec<Option<(i64, Option<Count>)>> = row
            .report_tiers
            .iter()
            .map(|c| c.as_ref().map(|c| (c.id, c.held)))
            .collect();
        assert_eq!(
            tiers,
            [
                Some((10, Some(Count::Exact(1000)))),
                Some((11, None)),
                None,
                Some((13, Some(Count::Thousands(2)))),
            ]
        );
        assert_eq!(
            commissions(&names, None, (1, 1), &[], &reports, &held).credit_students,
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
        assert_eq!(
            (rows[0].rate, rows[0].exp_each, rows[0].share),
            (1.5, 1000, 0.5)
        );
    }

    /// A student's three slots each level every tier from Lv1 to its top, fed
    /// in orbs; the orbs held count as so many students.
    #[test]
    fn the_hard_missions_show_each_students_eleph_three_stars_first_and_the_done_last() {
        let mission = |name: &str, drops: &[i64]| HardMission {
            name: name.into(),
            ap: 20,
            drops: drops.to_vec(),
        };
        let missions = [
            mission("1-1", &[2, 13010]),
            mission("1-3", &[10010]),
            mission("4-2", &[10010]),
            mission("12-1", &[10010]),
            mission("18-3", &[10000]),
            mission("20-1", &[16000]),
        ];
        let elephs = [
            item(10000, "アルの神名文字", Rarity::N),
            item(10010, "ユウカの神名文字", Rarity::N),
            item(13010, "ジュンコの神名文字", Rarity::N),
            item(16000, "ハルカの神名文字", Rarity::N),
        ];
        // アル comes with three stars and is at the weapon's third, with 80 elephs held;
        // ユウカ at ★2; ジュンコ never read; ハルカ done.
        let student = |id| {
            let (name, first, now) = match id {
                10000 => ("アル", 3, Some(7)),
                10010 => ("ユウカ", 1, Some(2)),
                13010 => ("ジュンコ", 2, None),
                _ => ("ハルカ", 3, Some(8)),
            };
            HardStudent {
                name: name.into(),
                first,
                now,
            }
        };
        let held = HashMap::from([(10000, Count::Exact(80))]);
        let eleph = |id| elephs.iter().find(|e| e.id == id);
        let cards = hard(&missions, eleph, student, &held, |id| id == 10010);
        let order: Vec<i64> = cards.iter().map(|c| c.id).collect();
        assert_eq!(order, [10000, 13010, 10010, 16000]);
        let aru = &cards[0];
        assert_eq!(
            (aru.stages.clone(), aru.ap, aru.star, aru.held, aru.done),
            (
                vec!["18-3".to_owned()],
                60,
                Some(7),
                Some(Count::Exact(80)),
                false
            )
        );
        let yuuka = &cards[2];
        assert_eq!(yuuka.stages, ["1-3", "4-2", "12-1"]);
        assert_eq!((yuuka.ap, yuuka.held, yuuka.reserved), (180, None, true));
        assert_eq!(cards[1].star, None);
        assert!(cards[3].done && !cards[0].reserved);
        assert_eq!(aru.icon, "icon_10000");
    }

    #[test]
    fn a_raise_counts_the_students_the_exp_held_raises_at_its_rate() {
        // Two titanium springs (1,000 exp each) and about 1,000 rusty ones (10 each),
        // taken at 1.5 by a weapon that takes 4,000.
        let held = HashMap::from([(13, Count::Exact(2)), (10, Count::Thousands(1))]);
        let row = raise(&[(10, 10), (13, 1000)], 1.5, 4000, &held);
        assert_eq!(
            (row.exp, row.about, row.rate, row.exp_each),
            (12_000, true, 1.5, 4000)
        );
        assert!((row.students - 4.5).abs() < 1e-9, "{}", row.students);
        assert!(raise(&[(10, 10)], 1.5, 0, &held).students.abs() < 1e-9);
    }

    #[test]
    fn a_student_takes_the_orbs_of_three_slots_from_t1_to_their_tops() {
        // Tier 1 takes 15 exp to its top, tier 2 50, fed 10 at a time.
        let each = orbs_each(
            &[vec![0, 5, 15], vec![0, 20, 50]],
            &[3, 3],
            &[(1, 10), (2, 40)],
        );
        assert_eq!(each, 3 * (20 + 50));
    }
}
