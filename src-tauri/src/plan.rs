//! The training plan: what the students marked for training take to reach
//! their targets (levels, skills and equipment tiers), and how that compares
//! with what is held.

use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

use serde::{Deserialize, Serialize};

use crate::{Item, quantity::Count, student::Training};

/// The universal blueprints one blueprint of each tier is made from (by tier,
/// T2 to T10; the Blue Archive wiki as schale-calc's equipment-upgrade.md has
/// it, 2026-10-13).
pub const RATE: [u64; 11] = [0, 0, 2, 3, 5, 7, 10, 15, 20, 30, 50];
/// What a universal blueprint's name ends with, after its slot's ("帽子").
pub const UNIVERSAL: &str = "の万能設計図";
/// Blueprint and universal blueprint ids start here; below are the pieces.
pub const BLUEPRINTS: i64 = 100_000;

/// How far a student is to be trained: level, the four skills (EX first) and
/// the three equipment slots' tiers. What a target leaves out is the default's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Target {
    pub level: u8,
    pub skills: [u8; 4],
    pub equipment: [u8; 3],
    /// The star (see `star`); `None` leaves it as it is.
    pub star: Option<u8>,
}

impl Default for Target {
    /// The user's starting target of 2026-10-12: Lv90, skills 5/10/10/10,
    /// equipment T10/10/10.
    fn default() -> Self {
        Self {
            level: 90,
            skills: [5, 10, 10, 10],
            equipment: [10, 10, 10],
            star: None,
        }
    }
}

/// A student's growth data from the snapshot: the categories of the three
/// equipment slots, and the materials of each skill level as `(item, amount)`:
/// EX from Lv2 to 5, the other skills from Lv2 to 9.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Grower {
    pub id: i64,
    pub equipment: Vec<String>,
    #[serde(default)]
    pub ex: Vec<Vec<(i64, u64)>>,
    #[serde(default)]
    pub skills: Vec<Vec<(i64, u64)>>,
    /// The ooparts (their first tier) the student's potential takes.
    #[serde(default)]
    pub potential: Option<i64>,
}

/// A piece of equipment of the snapshot: what it is made from (blueprints and
/// credits, from the tier below) and how far it levels.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Piece {
    pub id: i64,
    pub category: String,
    pub tier: u8,
    pub max_level: u8,
    #[serde(default)]
    pub recipe: Vec<(i64, u64)>,
    #[serde(default)]
    pub recipe_cost: u64,
}

/// The growth tables (`data/schaledb/growth.json`): the exp from Lv1 to each
/// level of a student and, tier by tier, of equipment; the credits each exp
/// costs; the exp of each report and enhancement orb; the credits of each
/// skill level (EX Lv2 to 5, the others Lv2 to 10); the note Lv10 takes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Growth {
    pub level_exp: Vec<u64>,
    pub level_credits: u64,
    pub reports: Vec<(i64, u64)>,
    pub equipment_exp: Vec<Vec<u64>>,
    pub equipment_credits: u64,
    pub orbs: Vec<(i64, u64)>,
    pub ex_credits: Vec<u64>,
    pub skill_credits: Vec<u64>,
    pub secret_note: i64,
    /// Elephs and credits of each star step: the student's from 2 to 5, the
    /// unique weapon's from 2 to 4.
    pub star_up: Vec<(u64, u64)>,
    pub weapon_up: Vec<(u64, u64)>,
    /// The unique weapon's exp from Lv1 to each level, and the credits each
    /// exp costs.
    pub weapon_exp: Vec<u64>,
    pub weapon_credits: u64,
    /// Each level of a potential (HP, attack or healing, Lv1 to 25): the tier
    /// of its ooparts (0 the first) and how many.
    #[serde(default)]
    pub potential: Vec<(u8, u64)>,
}

/// A part of a student's training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Level,
    Skill(usize),
    Equipment(usize),
    Star,
}

/// What a plan takes: items and equipment (blueprints) by id, credits, the exp
/// of reports and of enhancement orbs; and the parts that could not be
/// counted, by student (the student or the current value unknown).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs {
    pub items: BTreeMap<i64, u64>,
    pub equipment: BTreeMap<i64, u64>,
    pub credits: u64,
    pub report_exp: u64,
    pub orb_exp: u64,
    pub unknown: Vec<(i64, Part)>,
}

impl Needs {
    /// Adds what `other` takes.
    pub fn add(&mut self, other: Needs) {
        for (id, amount) in other.items {
            *self.items.entry(id).or_default() += amount;
        }
        for (id, amount) in other.equipment {
            *self.equipment.entry(id).or_default() += amount;
        }
        self.credits += other.credits;
        self.report_exp += other.report_exp;
        self.orb_exp += other.orb_exp;
        self.unknown.extend(other.unknown);
    }
}

/// An amount held: exact, or about as much where a count is shown in
/// thousands ("51K").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Held {
    Exact(u64),
    About(u64),
}

impl Held {
    /// How many, whether exact or about.
    pub fn amount(self) -> u64 {
        match self {
            Held::Exact(n) | Held::About(n) => n,
        }
    }
}

/// What `counts` hold of the ids in `worth`, each counted at its worth (the
/// exp of a report, or 1 for an item); an id not counted is none.
pub fn held<S: BuildHasher>(counts: &HashMap<i64, Count, S>, worth: &[(i64, u64)]) -> Held {
    let (mut sum, mut about) = (0, false);
    for &(id, each) in worth {
        let count = match counts.get(&id) {
            Some(Count::Exact(n)) => u64::from(*n),
            Some(Count::Thousands(k)) => {
                about = true;
                u64::from(*k) * 1000
            }
            None => 0,
        };
        sum += count * each;
    }
    if about {
        Held::About(sum)
    } else {
        Held::Exact(sum)
    }
}

/// The growth data a plan is counted with.
pub struct Planner {
    pub growth: Growth,
    pub growers: Vec<Grower>,
    pub pieces: Vec<Piece>,
}

impl Planner {
    /// What the `targets` take from the students' training as last read
    /// (`trained`); a target whose student was never read counts nothing.
    pub fn needs(
        &self,
        targets: &BTreeMap<i64, Target>,
        trained: &BTreeMap<i64, Training>,
    ) -> Needs {
        let mut total = Needs::default();
        for (&id, target) in targets {
            if let Some(now) = trained.get(&id) {
                let grower = self.growers.iter().find(|g| g.id == id);
                total.add(needs(id, grower, now, target, &self.growth, &self.pieces));
            }
        }
        total
    }
}

/// What a line of the plan counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Credits,
    /// The exp of activity reports.
    ReportExp,
    /// The exp of enhancement orbs.
    OrbExp,
    Item,
    Equipment,
}

/// A line of the plan for the window: what is needed against what is held
/// (`None`: not known, as the credits before they are read).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NeedRow {
    pub kind: Kind,
    pub id: i64,
    pub name: String,
    /// The icon's name (see `crate::icon`); none for credits and exp.
    pub icon: String,
    /// For the kinds that come in four tiers, the family (its first tier's id)
    /// and the tier, the window showing a family on one line.
    pub family: Option<(i64, u8)>,
    pub need: u64,
    pub have: Option<Held>,
}

/// A slot's blueprints a plan takes, for the window: the slot, named as its
/// universal blueprints are ("帽子") and shown by its T1 piece; its blueprints
/// of T2 to T10; and how they are paid: the blueprints held first (`paid` of
/// them), then the rest of T2 to T9 from the universal ones at `RATE` (their
/// `need`), the rest of T10 (`short`) left for the player to make up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlueprintRow {
    pub name: String,
    pub icon: String,
    pub tiers: Vec<TierBlueprint>,
    pub paid: u64,
    pub universal: TierBlueprint,
    pub short: u64,
}

/// A tier's blueprints (0 the universal ones): needed and held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TierBlueprint {
    pub tier: u8,
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub need: u64,
    pub have: Held,
}

/// The slots whose blueprints `needs` takes, in the order of their T1 pieces,
/// each as `BlueprintRow` tells; the `entries` name the equipment.
pub fn blueprints<S: BuildHasher>(
    needs: &BTreeMap<i64, u64>,
    pieces: &[Piece],
    entries: &[Item],
    counts: &HashMap<i64, Count, S>,
) -> Vec<BlueprintRow> {
    let entry = |id: i64| entries.iter().find(|e| e.id == id);
    let blueprint = |p: &Piece| {
        let e = entry(p.id)?;
        Some(TierBlueprint {
            tier: p.tier,
            id: p.id,
            name: e.name.clone(),
            icon: e.icon.clone(),
            need: needs.get(&p.id).copied().unwrap_or(0),
            have: held(counts, &[(p.id, 1)]),
        })
    };
    let firsts = pieces.iter().filter(|p| p.id < BLUEPRINTS && p.tier == 1);
    firsts
        .filter_map(|first| {
            let slot = |p: &&Piece| p.category == first.category && p.id >= BLUEPRINTS;
            let tiers = pieces.iter().filter(slot).filter(|p| p.tier >= 2);
            let mut tiers: Vec<TierBlueprint> = tiers.map(blueprint).collect::<Option<_>>()?;
            if tiers.iter().all(|t| t.need == 0) {
                return None;
            }
            tiers.sort_by_key(|t| t.tier);
            let mut universal = blueprint(pieces.iter().filter(slot).find(|p| p.tier == 0)?)?;
            let (mut paid, mut short) = (0, 0);
            for t in &tiers {
                let own = t.need.min(t.have.amount());
                paid += own;
                if t.tier == 10 {
                    short = t.need - own;
                } else {
                    universal.need += (t.need - own) * RATE[usize::from(t.tier)];
                }
            }
            Some(BlueprintRow {
                name: universal.name.strip_suffix(UNIVERSAL)?.to_owned(),
                icon: entry(first.id)?.icon.clone(),
                tiers,
                paid,
                universal,
                short,
            })
        })
        .collect()
}

/// A part of a target's training that could not be counted, for the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnknownRow {
    pub id: i64,
    pub name: String,
    pub part: Part,
}

/// A student's star on one scale: 1 to 5 their own (5 with the unique
/// weapon's first), then 6 to 8 the weapon's 2 to 4.
pub fn star_of(now: &Training) -> Option<u8> {
    match (now.rarity?, now.weapon) {
        (5, Some(Some(weapon))) => Some(4 + weapon.stars),
        (5, _) => None,
        (rarity, _) => Some(rarity),
    }
}

/// What the student `id` takes from `now` to `target`; `grower` is `None` for
/// a student the snapshot lacks, whose skills and equipment are unknown.
pub fn needs(
    id: i64,
    grower: Option<&Grower>,
    now: &Training,
    target: &Target,
    growth: &Growth,
    pieces: &[Piece],
) -> Needs {
    let mut n = Needs::default();
    // The level: its exp in reports, and the credits that exp costs.
    match now.level {
        Some(level) if level < target.level => {
            let exp = growth.level_exp[usize::from(target.level) - 1]
                - growth.level_exp[usize::from(level) - 1];
            n.report_exp += exp;
            n.credits += exp * growth.level_credits;
        }
        Some(_) => {}
        None => n.unknown.push((id, Part::Level)),
    }
    for (skill, (&level, &to)) in now.skills.iter().zip(&target.skills).enumerate() {
        // A skill not yet unlocked (0) takes from Lv1 on what the others do.
        let from = level.map(|l| l.max(1));
        match (from, grower) {
            (Some(from), _) if from >= to => {}
            (Some(from), Some(grower)) => skill_needs(&mut n, grower, skill, from, to, growth),
            _ => n.unknown.push((id, Part::Skill(skill))),
        }
    }
    if let Some(to) = target.star {
        match star_of(now) {
            Some(star) if star >= to => {}
            // A student's elephs are the item of their id.
            Some(star) if id >= 0 => {
                for step in star + 1..=to {
                    let (elephs, credits) = if step <= 5 {
                        growth.star_up[usize::from(step) - 2]
                    } else {
                        growth.weapon_up[usize::from(step) - 6]
                    };
                    *n.items.entry(id).or_default() += elephs;
                    n.credits += credits;
                }
            }
            _ => n.unknown.push((id, Part::Star)),
        }
    }
    for (slot, (&gear, &to)) in now.equipment.iter().zip(&target.equipment).enumerate() {
        let category = grower.and_then(|g| g.equipment.get(slot));
        match (gear, category) {
            (Some(gear), _) if gear.tier >= to => {}
            // Nothing is asked of a slot left at the first tier.
            (None, _) if to <= 1 => {}
            (Some(gear), Some(category)) => {
                equipment_needs(&mut n, category, gear.tier, gear.level, to, growth, pieces);
            }
            _ => n.unknown.push((id, Part::Equipment(slot))),
        }
    }
    n
}

/// Skill `skill` (0 the EX) from Lv`from` to Lv`to`.
fn skill_needs(n: &mut Needs, grower: &Grower, skill: usize, from: u8, to: u8, growth: &Growth) {
    for level in usize::from(from) + 1..=usize::from(to) {
        let (materials, credits) = if skill == 0 {
            (grower.ex.get(level - 2), growth.ex_credits[level - 2])
        } else {
            (
                grower.skills.get(level - 2),
                growth.skill_credits[level - 2],
            )
        };
        n.credits += credits;
        let note = [(growth.secret_note, 1)];
        // Lv10 takes the note the material lists stop short of.
        let materials = materials.map_or(&note[..], Vec::as_slice);
        for &(item, amount) in materials {
            *n.items.entry(item).or_default() += amount;
        }
    }
}

/// A slot of `category` from tier `tier` at Lv`level` to tier `to`: each tier
/// levelled to its top with enhancement orbs, then made into the next.
fn equipment_needs(
    n: &mut Needs,
    category: &str,
    tier: u8,
    level: u8,
    to: u8,
    growth: &Growth,
    pieces: &[Piece],
) {
    let piece = |tier: u8| {
        pieces
            .iter()
            .find(|p| p.category == category && p.tier == tier)
    };
    for t in tier..to {
        let from = if t == tier { level } else { 1 };
        let top = piece(t).map_or(from, |p| p.max_level);
        let exp = &growth.equipment_exp[usize::from(t) - 1];
        let gained = exp[usize::from(top) - 1].saturating_sub(exp[usize::from(from) - 1]);
        n.orb_exp += gained;
        n.credits += gained * growth.equipment_credits;
        if let Some(next) = piece(t + 1) {
            for &(blueprint, amount) in &next.recipe {
                *n.equipment.entry(blueprint).or_default() += amount;
            }
            n.credits += next.recipe_cost;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::student::{Gear, Weapon};

    /// Small tables: Lv1 to 4, two equipment tiers of three levels.
    pub(crate) fn growth() -> Growth {
        Growth {
            level_exp: vec![0, 10, 30, 60],
            level_credits: 7,
            reports: vec![(10, 50)],
            equipment_exp: vec![vec![0, 5, 15], vec![0, 20, 50]],
            equipment_credits: 4,
            orbs: vec![(1, 90)],
            ex_credits: vec![100, 200, 300, 400],
            skill_credits: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            secret_note: 9999,
            star_up: vec![(30, 10), (80, 40), (100, 200), (120, 1000)],
            weapon_up: vec![(120, 1500), (180, 2000), (200, 2500)],
            weapon_exp: Vec::new(),
            weapon_credits: 0,
            potential: Vec::new(),
        }
    }

    fn grower() -> Grower {
        Grower {
            id: 1,
            equipment: vec!["Hat".into(), "Bag".into(), "Charm".into()],
            ex: vec![
                vec![(3030, 2)],
                vec![(3031, 3)],
                vec![(3032, 4)],
                vec![(3033, 5), (150, 1)],
            ],
            skills: (2..=9).map(|lv| vec![(4030, lv)]).collect(),
            potential: None,
        }
    }

    fn pieces() -> Vec<Piece> {
        let piece = |id, tier, max_level, recipe: Vec<(i64, u64)>, recipe_cost| Piece {
            id,
            category: "Hat".into(),
            tier,
            max_level,
            recipe,
            recipe_cost,
        };
        vec![
            piece(1000, 1, 3, vec![], 0),
            piece(1001, 2, 3, vec![(101_001, 15)], 1500),
            piece(1002, 3, 3, vec![(101_002, 20), (101_001, 5)], 3000),
        ]
    }

    fn now(level: u8, skills: [u8; 4], hat: Option<Gear>) -> Training {
        Training {
            level: Some(level),
            rarity: Some(3),
            bond: Some(1),
            skills: skills.map(Some),
            weapon: Some(None),
            equipment: [hat, None, None],
            gear: Some(0),
            ability: [Some(0); 3],
        }
    }

    /// A target that asks nothing beyond `now`.
    fn reached(t: &Training) -> Target {
        Target {
            level: t.level.unwrap(),
            skills: t.skills.map(Option::unwrap),
            equipment: t.equipment.map(|g| g.map_or(1, |g| g.tier)),
            star: None,
        }
    }

    #[test]
    fn stars_take_the_students_elephs_up_to_the_weapons_fourth() {
        // ★3 to the weapon's ★3: ★4, ★5 (the weapon's ★1 with it), weapon ★2, ★3.
        let student = now(4, [1, 1, 1, 1], Some(Gear { tier: 1, level: 1 }));
        let target = Target {
            star: Some(7),
            ..reached(&student)
        };
        let n = needs(
            10000,
            Some(&grower()),
            &student,
            &target,
            &growth(),
            &pieces(),
        );
        assert_eq!(n.items, BTreeMap::from([(10000, 100 + 120 + 120 + 180)]));
        assert_eq!(n.credits, 200 + 1000 + 1500 + 2000);
        // Where the weapon is, the steps go on from it.
        let mut five = student;
        five.rarity = Some(5);
        five.weapon = Some(Some(Weapon {
            level: 30,
            stars: 2,
        }));
        assert_eq!(star_of(&five), Some(6));
        let n = needs(
            10000,
            None,
            &five,
            &Target {
                star: Some(8),
                ..target
            },
            &growth(),
            &pieces(),
        );
        assert_eq!(n.items, BTreeMap::from([(10000, 180 + 200)]));
        // A target at or below the star asks nothing; a student SchaleDB lacks
        // has no eleph to count.
        assert_eq!(
            needs(
                10000,
                None,
                &five,
                &Target {
                    star: Some(5),
                    ..target
                },
                &growth(),
                &pieces()
            )
            .items,
            BTreeMap::new()
        );
        let n = needs(-1, None, &student, &target, &growth(), &pieces());
        assert_eq!(n.unknown, [(-1, Part::Star)]);
    }

    #[test]
    fn plans_add_up() {
        let mut total = Needs {
            items: BTreeMap::from([(1, 2)]),
            credits: 10,
            unknown: vec![(1, Part::Level)],
            ..Needs::default()
        };
        total.add(Needs {
            items: BTreeMap::from([(1, 3), (2, 1)]),
            equipment: BTreeMap::from([(5, 1)]),
            credits: 5,
            report_exp: 7,
            orb_exp: 9,
            unknown: vec![(2, Part::Skill(0))],
        });
        assert_eq!(
            total,
            Needs {
                items: BTreeMap::from([(1, 5), (2, 1)]),
                equipment: BTreeMap::from([(5, 1)]),
                credits: 15,
                report_exp: 7,
                orb_exp: 9,
                unknown: vec![(1, Part::Level), (2, Part::Skill(0))],
            }
        );
    }

    #[test]
    fn what_is_held_counts_its_exp_and_is_about_where_a_count_is_in_thousands() {
        let reports = [(10, 50), (11, 500)];
        let counts = HashMap::from([(10, Count::Exact(3)), (11, Count::Exact(2))]);
        assert_eq!(held(&counts, &reports), Held::Exact(1150));
        let counts = HashMap::from([(10, Count::Thousands(12)), (11, Count::Exact(2))]);
        assert_eq!(held(&counts, &reports), Held::About(601_000));
        assert_eq!(held(&HashMap::new(), &[(4030, 1)]), Held::Exact(0));
    }

    #[test]
    fn levels_take_reports_exp_and_credits_for_it() {
        let student = now(2, [1, 1, 1, 1], Some(Gear { tier: 1, level: 1 }));
        let target = Target {
            level: 4,
            ..reached(&student)
        };
        let n = needs(1, Some(&grower()), &student, &target, &growth(), &pieces());
        assert_eq!((n.report_exp, n.credits), (50, 350));
        assert!(n.items.is_empty() && n.equipment.is_empty() && n.unknown.is_empty());
    }

    #[test]
    fn a_target_reached_takes_nothing() {
        let student = now(4, [5, 10, 10, 10], Some(Gear { tier: 3, level: 3 }));
        let n = needs(
            1,
            Some(&grower()),
            &student,
            &reached(&student),
            &growth(),
            &pieces(),
        );
        assert_eq!(n, Needs::default());
    }

    #[test]
    fn skills_take_their_levels_materials_credits_and_the_note_for_lv10() {
        let student = now(4, [3, 8, 1, 10], Some(Gear { tier: 1, level: 1 }));
        let target = Target {
            skills: [5, 10, 2, 10],
            ..reached(&student)
        };
        let n = needs(1, Some(&grower()), &student, &target, &growth(), &pieces());
        // EX Lv4 and 5; normal Lv9 and 10 (the note); passive Lv2.
        assert_eq!(
            n.items,
            BTreeMap::from([(3032, 4), (3033, 5), (150, 1), (4030, 9 + 2), (9999, 1)])
        );
        assert_eq!(n.credits, 300 + 400 + 8 + 9 + 1);
    }

    #[test]
    fn equipment_levels_to_the_top_of_each_tier_then_takes_the_next_ones_recipe() {
        let student = now(4, [1, 1, 1, 1], Some(Gear { tier: 1, level: 2 }));
        let target = Target {
            equipment: [3, 1, 1],
            ..reached(&student)
        };
        let n = needs(1, Some(&grower()), &student, &target, &growth(), &pieces());
        // Tier 1 from Lv2 to 3 (10 exp), tier 2 from Lv1 to 3 (50 exp).
        assert_eq!(n.orb_exp, 60);
        assert_eq!(n.equipment, BTreeMap::from([(101_001, 20), (101_002, 20)]));
        assert_eq!(n.credits, 60 * 4 + 1500 + 3000);
    }

    #[test]
    fn a_slots_blueprints_pay_from_themselves_then_below_t10_from_the_universal_ones() {
        let (pieces, entries) = hat_blueprints();
        let needs = BTreeMap::from([(101_001, 15), (101_002, 20), (101_009, 60)]);
        let held = HashMap::from([
            (101_001, Count::Exact(10)),
            (101_002, Count::Exact(30)),
            (101_009, Count::Exact(17)),
            (501_000, Count::Thousands(1)),
        ]);
        let rows = blueprints(&needs, &pieces, &entries, &held);
        assert_eq!(rows.len(), 1);
        let hat = &rows[0];
        assert_eq!(
            (hat.name.as_str(), hat.icon.as_str()),
            ("帽子", "icon_1000")
        );
        let tiers: Vec<(u8, u64)> = hat.tiers.iter().map(|t| (t.tier, t.need)).collect();
        let mut want = vec![(2, 15), (3, 20)];
        want.extend((4..=9).map(|tier| (tier, 0)));
        want.push((10, 60));
        assert_eq!(tiers, want);
        // 10 of T2's own, 20 of T3's and 17 of T10's; T2's other 5 from 10
        // universal ones; T10's other 43 left short.
        let u = &hat.universal;
        assert_eq!(
            (hat.paid, u.need, u.have, hat.short),
            (47, 10, Held::About(1000), 43)
        );
        assert_eq!(u.name, "帽子の万能設計図");
        // A slot nothing is asked of is left out.
        assert!(blueprints(&BTreeMap::new(), &pieces, &entries, &held).is_empty());
    }

    /// A hat's T1 piece, its blueprints of T2 to T10 and its universal ones.
    fn hat_blueprints() -> (Vec<Piece>, Vec<crate::Item>) {
        let mut ids = vec![(1000, 1, "無地のキャップ".to_owned())];
        ids.extend(
            (2..=10).map(|tier| (100_999 + i64::from(tier), tier, format!("T{tier}の設計図"))),
        );
        ids.push((501_000, 0, "帽子の万能設計図".to_owned()));
        let pieces = ids
            .iter()
            .map(|(id, tier, _)| Piece {
                id: *id,
                category: "Hat".into(),
                tier: *tier,
                max_level: 1,
                recipe: Vec::new(),
                recipe_cost: 0,
            })
            .collect();
        let entries = ids
            .into_iter()
            .map(|(id, tier, name)| crate::Item {
                id,
                name,
                icon: format!("icon_{id}"),
                category: "Hat".into(),
                tier,
                rarity: None,
            })
            .collect();
        (pieces, entries)
    }

    #[test]
    fn what_cannot_be_counted_is_told_by_part() {
        let mut student = now(2, [1, 1, 1, 1], Some(Gear { tier: 1, level: 1 }));
        student.skills[2] = None;
        let target = Target {
            level: 3,
            skills: [1, 1, 2, 1],
            equipment: [2, 2, 1],
            star: None,
        };
        let n = needs(1, Some(&grower()), &student, &target, &growth(), &pieces());
        assert_eq!(n.unknown, [(1, Part::Skill(2)), (1, Part::Equipment(1))]);
        // A student the snapshot lacks: the level counts, the rest is unknown.
        let n = needs(-1, None, &student, &target, &growth(), &pieces());
        assert_eq!(n.report_exp, 20);
        assert_eq!(
            n.unknown,
            [
                (-1, Part::Skill(2)),
                (-1, Part::Equipment(0)),
                (-1, Part::Equipment(1))
            ]
        );
    }
}
