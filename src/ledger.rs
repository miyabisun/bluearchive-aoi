//! What a training or a purchase, read as it happened, takes from or gives to the lists'
//! counts before the lists are read again (the user's of 2026-11-15: 「生徒の固有武器の星が
//! 増えて神名文字がその分減っているはずなのに、まだ減っていない数字を表示している」「陳列データが
//! 売切れに変化してコインも減っている、なのにアイテムは増えていない」).
//!
//! A change is counted only where it surely came after the list's last reading: its old
//! value was read in this run, in a frame that read it, and the list it counts in was not
//! shown since. Otherwise the list may show the change already, and counting it would count
//! it twice.

use std::collections::{BTreeMap, HashMap};

use crate::{
    List,
    plan::{self, BLUEPRINTS, Kind, Part, Planner, RATE, Target, star_of},
    shop::{self, Offer},
    student::Training,
    supply::{ChoiceBox, boxes_for},
    tally::Tally,
};

/// The parts of a training whose materials are counted: the star, the skills and the
/// equipment. Not the level: which reports fed it is not told.
const PARTS: [Part; 8] = [
    Part::Star,
    Part::Skill(0),
    Part::Skill(1),
    Part::Skill(2),
    Part::Skill(3),
    Part::Equipment(0),
    Part::Equipment(1),
    Part::Equipment(2),
];

/// How far `part` of a training is, where it was read.
fn value(training: &Training, part: Part) -> Option<u8> {
    match part {
        Part::Level => None,
        Part::Star => star_of(training),
        Part::Skill(skill) => training.skills[skill],
        Part::Equipment(slot) => training.equipment[slot].map(|gear| gear.tier),
    }
}

/// The list that counts what `part` takes.
pub fn list_of(part: Part) -> List {
    match part {
        Part::Equipment(_) => List::Equipment,
        _ => List::Items,
    }
}

/// When each thing was last seen, in frames of this run.
#[derive(Default)]
pub struct Ledger {
    frame: u64,
    /// When each list was last shown.
    listed: HashMap<List, u64>,
    /// When a student's part was last read at the value known of it.
    parts: BTreeMap<(i64, Part), u64>,
    /// When a shop's offer (by item and coin) was last read with the times left known of it.
    offers: HashMap<(&'static str, i64, u8), u64>,
}

impl Ledger {
    /// A frame comes.
    pub fn frame(&mut self) {
        self.frame += 1;
    }

    /// The frame shows `list`.
    pub fn listed(&mut self, list: List) {
        self.listed.insert(list, self.frame);
    }

    /// Whether what was seen at `seen` was seen after `list` was last shown.
    fn since(&self, seen: Option<u64>, list: List) -> bool {
        seen.is_some_and(|seen| seen > self.listed.get(&list).copied().unwrap_or(0))
    }

    /// The frame reads student `id` so (`read`), `known` being what was known of them: the
    /// parts it reads as known are seen. A part it could not read is not: the roster
    /// keeps the known value for it, which tells nothing of now.
    pub fn student(&mut self, id: i64, known: Option<&Training>, read: &Training) {
        let Some(known) = known else { return };
        for part in PARTS {
            if let Some(now) = value(read, part)
                && value(known, part) == Some(now)
            {
                self.parts.insert((id, part), self.frame);
            }
        }
    }

    /// The frame takes student `id` trained from `before` to `after`: the parts gone up
    /// that are to be counted (see the module). Nothing in the game goes down: a part
    /// read lower is a misreading put right, and counts nothing.
    pub fn trained(&mut self, id: i64, before: &Training, after: &Training) -> Vec<Part> {
        let mut counted = Vec::new();
        for part in PARTS {
            let (was, now) = (value(before, part), value(after, part));
            if was == now || now.is_none() {
                continue;
            }
            // The value taken is read by this frame.
            let seen = self.parts.insert((id, part), self.frame);
            if was.is_some() && was < now && self.since(seen, list_of(part)) {
                counted.push(part);
            }
        }
        counted
    }

    /// The frame reads `shop`'s offers so (`read`), `known` being its offers as known:
    /// those read with the times left known are seen.
    pub fn offers(&mut self, shop: &'static str, known: &[Offer], read: &[Offer]) {
        for offer in read {
            let same = |k: &&Offer| (k.id, k.coin, k.left) == (offer.id, offer.coin, offer.left);
            if known.iter().any(|k| same(&k)) {
                self.offers.insert((shop, offer.id, offer.coin), self.frame);
            }
        }
    }

    /// The frame takes `shop`'s offer of `good` for `coin` bought: whether the purchase is
    /// to be counted in `list` (see the module).
    pub fn bought(&mut self, shop: &'static str, good: i64, coin: u8, list: List) -> bool {
        let seen = self.offers.insert((shop, good, coin), self.frame);
        self.since(seen, list)
    }
}

/// Takes `amount` of `id` from what `tally` counts, and what that is short of from the
/// `others` in their order, each at so many for one. One never counted is left so: how
/// many were held is not known. Where all of them are short, some of `id` were had unseen:
/// its count is forgotten and the others are left.
fn pay(tally: &mut Tally, id: i64, amount: u64, others: &[(i64, u64)]) {
    let Some(held) = tally.amount(id) else { return };
    let mut short = amount.saturating_sub(held);
    let mut taken = vec![(id, amount - short)];
    for &(other, rate) in others {
        let made_up = (tally.amount(other).unwrap_or(0) / rate.max(1)).min(short);
        taken.push((other, made_up * rate));
        short -= made_up;
    }
    if short > 0 {
        tally.forget(id);
        return;
    }
    for (id, taken) in taken {
        tally.shift(id, -i64::try_from(taken).unwrap_or(i64::MAX));
    }
}

/// The universal blueprints of `blueprint`'s slot, and how many make one of it.
fn universal(blueprint: i64, pieces: &[plan::Piece]) -> Option<(i64, u64)> {
    let print = pieces.iter().find(|p| p.id == blueprint)?;
    let rate = RATE.get(usize::from(print.tier)).copied()?;
    let of_slot = |p: &&plan::Piece| p.category == print.category && p.id >= BLUEPRINTS;
    let universal = pieces.iter().filter(of_slot).find(|p| p.tier == 0)?;
    (rate > 0).then_some((universal.id, rate))
}

/// Takes from the lists what the `parts` of student `id` took from `before` to `after`:
/// the star the student's elephs; a skill its materials, then the choice boxes giving one,
/// those of fewest choices first (the user's of 2026-11-15: 「対象の消費アイテム -> 選択ボックスを
/// 自動的に使用」「6校分のものが優先的に選択されます」); a slot's tier its blueprints, then the
/// slot's universal ones at `RATE` (「装備強化は常に現物の設計図 -> 万能設計図で補ってしまった
/// 事にしましょう」). See `pay`.
pub fn charge(
    planner: &Planner,
    boxes: &[ChoiceBox],
    id: i64,
    (before, after): (&Training, &Training),
    parts: &[Part],
    items: &mut Tally,
    equipment: &mut Tally,
) {
    // A target that asks nothing but the parts' new values.
    let mut target = Target {
        level: 0,
        skills: [0; 4],
        equipment: [0; 3],
        star: None,
    };
    for &part in parts {
        let to = value(after, part);
        match part {
            Part::Level => {}
            Part::Star => target.star = to,
            Part::Skill(skill) => target.skills[skill] = to.unwrap_or(0),
            Part::Equipment(slot) => target.equipment[slot] = to.unwrap_or(0),
        }
    }
    let grower = planner.growers.iter().find(|g| g.id == id);
    let needs = plan::needs(
        id,
        grower,
        before,
        &target,
        &planner.growth,
        &planner.pieces,
    );
    for (item, amount) in needs.items {
        // A student's elephs (the item of their id) are spent as held alone (the user's:
        // 「勝手にカケラを消費して行う事は出来ず」).
        let made_up: Vec<(i64, u64)> = if item == id {
            Vec::new()
        } else {
            let boxes = boxes_for(Kind::Item, item, boxes);
            boxes.iter().map(|b| (b.id, 1)).collect()
        };
        pay(items, item, amount, &made_up);
    }
    for (blueprint, amount) in needs.equipment {
        let made_up = universal(blueprint, &planner.pieces);
        pay(equipment, blueprint, amount, made_up.as_slice());
    }
}

/// The list a shop's good is counted in, and its id there; none for the credits, read off
/// the screen.
pub fn counted(good: i64) -> Option<(List, i64)> {
    if good >= shop::CURRENCY_GOODS {
        None
    } else if good >= shop::EQUIPMENT_GOODS {
        Some((List::Equipment, good - shop::EQUIPMENT_GOODS))
    } else {
        Some((List::Items, good))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        plan::{Grower, Piece, tests::growth},
        quantity::Count,
        student::Gear,
    };

    const STUDENT: i64 = 10_001;
    const NOTE: i64 = 4030;
    const SIX: i64 = 800;
    const EVERY: i64 = 801;
    const T2: i64 = 101_001;
    const T3: i64 = 101_002;
    const UNIVERSAL: i64 = 101_000;
    const PRINT_BOX: i64 = 802;
    const ELEPH_BOX: i64 = 803;

    /// A student whose skills take notes (as many as the level), with a hat: T2 from 15 of
    /// its blueprints, T3 from 20 of its own and 5 of T2's.
    fn planner() -> Planner {
        let piece = |id, tier, recipe: Vec<(i64, u64)>| Piece {
            id,
            category: "Hat".into(),
            tier,
            max_level: 3,
            recipe,
            recipe_cost: 0,
        };
        Planner {
            growth: growth(),
            growers: vec![Grower {
                id: STUDENT,
                equipment: vec!["Hat".into(), "Bag".into(), "Charm".into()],
                ex: Vec::new(),
                skills: (2..=9).map(|level| vec![(NOTE, level)]).collect(),
                potential: None,
                weapon: None,
                star: 3,
            }],
            pieces: vec![
                piece(1000, 1, vec![]),
                piece(1001, 2, vec![(T2, 15)]),
                piece(1002, 3, vec![(T3, 20), (T2, 5)]),
                piece(UNIVERSAL, 0, vec![]),
                piece(T2, 2, vec![]),
                piece(T3, 3, vec![]),
            ],
        }
    }

    /// The note's boxes, of six schools and of every school, the box of every school first
    /// in the data; a box of blueprints; and a box that gives the student's elephs.
    fn boxes() -> Vec<ChoiceBox> {
        let choice = |id, equipment, choices: Vec<i64>| ChoiceBox {
            id,
            equipment,
            choices,
        };
        vec![
            choice(EVERY, false, (NOTE..NOTE + 12).collect()),
            choice(SIX, false, (NOTE..NOTE + 6).collect()),
            choice(PRINT_BOX, true, vec![T2]),
            choice(ELEPH_BOX, false, vec![STUDENT]),
        ]
    }

    /// ★`rarity`, the passive skill at `skill`, a hat of `tier`.
    fn trained(rarity: u8, skill: u8, tier: u8) -> Training {
        Training {
            level: Some(1),
            rarity: Some(rarity),
            bond: Some(1),
            skills: [Some(1), Some(1), Some(skill), Some(1)],
            weapon: Some(None),
            equipment: [Some(Gear { tier }), None, None],
            gear: Some(0),
            ability: [Some(0); 3],
        }
    }

    fn tally(counts: &[(i64, Count)]) -> Tally {
        Tally::seeded(counts.iter().copied().collect())
    }

    /// How many of each are held as counted (see `Tally::amount`).
    fn held(tally: &Tally, ids: &[i64]) -> Vec<Option<u64>> {
        ids.iter().map(|&id| tally.amount(id)).collect()
    }

    /// Charges `parts` of the training `before` to `after` to the counts given.
    fn charged(
        (before, after): (Training, Training),
        parts: &[Part],
        items: &[(i64, u32)],
        equipment: &[(i64, Count)],
    ) -> (Tally, Tally) {
        let items: Vec<(i64, Count)> = items.iter().map(|&(id, c)| (id, Count::Exact(c))).collect();
        let (mut items, mut equipment) = (tally(&items), tally(equipment));
        charge(
            &planner(),
            &boxes(),
            STUDENT,
            (&before, &after),
            parts,
            &mut items,
            &mut equipment,
        );
        (items, equipment)
    }

    /// ★3 to ★4 takes 100 elephs (the item of the student's id), and nothing else makes
    /// them up: short of them, some were had unseen and their count is not known.
    #[test]
    fn a_star_takes_the_students_elephs() {
        let star = (trained(3, 1, 1), trained(4, 1, 1));
        let (items, _) = charged(star, &[Part::Star], &[(STUDENT, 150)], &[]);
        assert_eq!(held(&items, &[STUDENT]), [Some(50)]);
        let short = [(STUDENT, 60), (ELEPH_BOX, 90)];
        let (items, _) = charged(star, &[Part::Star], &short, &[]);
        assert_eq!(held(&items, &[STUDENT, ELEPH_BOX]), [None, Some(90)]);
    }

    /// A skill from Lv3 to Lv5 takes 4 and 5 notes: those held first, then the boxes that
    /// give the note, the box of six schools before the one of every school.
    #[test]
    fn a_skill_takes_its_materials_then_the_boxes_of_fewest_choices() {
        let skill = (trained(3, 3, 1), trained(3, 5, 1));
        let part = [Part::Skill(2)];
        let ids = [NOTE, SIX, EVERY];
        let (items, _) = charged(skill, &part, &[(NOTE, 20), (SIX, 3), (EVERY, 10)], &[]);
        assert_eq!(
            held(&items, &ids),
            [Some(11), Some(3), Some(10)],
            "enough held"
        );
        let (items, _) = charged(skill, &part, &[(NOTE, 4), (SIX, 3), (EVERY, 10)], &[]);
        assert_eq!(
            held(&items, &ids),
            [Some(0), Some(0), Some(8)],
            "boxes make up 5"
        );
        // All of them short, some notes were had unseen: how many are left is not known.
        let (items, _) = charged(skill, &part, &[(NOTE, 4), (SIX, 1), (EVERY, 1)], &[]);
        assert_eq!(held(&items, &ids), [None, Some(1), Some(1)], "all short");
        // Notes never counted: how many the boxes made up is not known.
        let (items, _) = charged(skill, &part, &[(SIX, 3), (EVERY, 10)], &[]);
        assert_eq!(
            held(&items, &ids),
            [None, Some(3), Some(10)],
            "never counted"
        );
    }

    /// A hat from T1 to T3 takes 20 blueprints of T2 and 20 of T3: those held first, then
    /// the hat's universal ones, 2 for a blueprint of T2. The blueprints' box is left.
    #[test]
    fn a_tier_takes_its_blueprints_then_the_universal_ones() {
        let tier = (trained(3, 1, 1), trained(3, 1, 3));
        let part = [Part::Equipment(0)];
        let ids = [T2, T3, UNIVERSAL, PRINT_BOX];
        let k = Count::Thousands;
        let e = Count::Exact;
        let with = |counts: &[(i64, Count)]| charged(tier, &part, &[], counts).1;
        let equipment = with(&[
            (T2, e(12)),
            (T3, e(30)),
            (UNIVERSAL, k(51)),
            (PRINT_BOX, e(5)),
        ]);
        // 8 of T2 short, 2 universal ones each: 51K stays 51K, less 16.
        assert_eq!(
            held(&equipment, &ids),
            [Some(0), Some(10), Some(50_984), Some(5)]
        );
        assert_eq!(equipment.counts()[&UNIVERSAL], k(51));
        // Blueprints never counted: how many the universal ones made up is not known.
        let equipment = with(&[(T3, e(30)), (UNIVERSAL, e(100))]);
        assert_eq!(held(&equipment, &ids), [None, Some(10), Some(100), None]);
        // The universal ones short too: some blueprints were had unseen.
        let equipment = with(&[(T2, e(12)), (T3, e(30)), (UNIVERSAL, e(10))]);
        assert_eq!(held(&equipment, &ids), [None, Some(10), Some(10), None]);
    }

    #[test]
    fn only_the_parts_given_are_charged() {
        let both = (trained(3, 3, 1), trained(4, 5, 3));
        let (items, equipment) = charged(
            both,
            &[Part::Skill(2)],
            &[(STUDENT, 150), (NOTE, 20)],
            &[(T2, Count::Exact(40))],
        );
        assert_eq!(held(&items, &[STUDENT, NOTE]), [Some(150), Some(11)]);
        assert_eq!(held(&equipment, &[T2]), [Some(40)]);
    }

    /// A training is counted once its old value was read after the list was last shown.
    #[test]
    fn a_training_seen_before_and_after_is_counted() {
        let (before, after) = (trained(3, 3, 1), trained(4, 5, 3));
        let mut ledger = Ledger::default();
        ledger.frame();
        ledger.listed(List::Items);
        ledger.listed(List::Equipment);
        ledger.frame();
        ledger.student(STUDENT, Some(&before), &before);
        ledger.frame();
        assert_eq!(
            ledger.trained(STUDENT, &before, &after),
            [Part::Star, Part::Skill(2), Part::Equipment(0)]
        );
        // Trained on at once, the value taken was seen as it was taken.
        ledger.frame();
        assert_eq!(
            ledger.trained(STUDENT, &after, &trained(4, 6, 3)),
            [Part::Skill(2)]
        );
    }

    /// The old value kept from an earlier run tells nothing of when the training came.
    #[test]
    fn a_training_not_seen_before_in_this_run_is_not_counted() {
        let mut ledger = Ledger::default();
        ledger.frame();
        assert_eq!(
            ledger.trained(STUDENT, &trained(3, 3, 1), &trained(4, 5, 3)),
            []
        );
    }

    /// A list shown since the old value was read may count the training already; the
    /// other list's parts are counted still.
    #[test]
    fn a_training_is_not_counted_in_a_list_shown_since() {
        let (before, after) = (trained(3, 3, 1), trained(4, 5, 3));
        let mut ledger = Ledger::default();
        ledger.frame();
        ledger.student(STUDENT, Some(&before), &before);
        ledger.frame();
        ledger.listed(List::Items);
        ledger.frame();
        assert_eq!(
            ledger.trained(STUDENT, &before, &after),
            [Part::Equipment(0)]
        );
    }

    /// A frame that could not read a part keeps what was known of it, and is no sight of
    /// it: read then, the star may have gone up before the list was shown.
    #[test]
    fn a_part_a_frame_could_not_read_was_not_seen_in_it() {
        let (before, after) = (trained(3, 3, 1), trained(4, 3, 1));
        let unread = Training {
            rarity: None,
            ..before
        };
        let mut ledger = Ledger::default();
        ledger.frame();
        ledger.student(STUDENT, Some(&before), &before);
        ledger.frame();
        ledger.listed(List::Items);
        ledger.frame();
        ledger.student(STUDENT, Some(&before), &unread);
        ledger.frame();
        assert_eq!(ledger.trained(STUDENT, &before, &after), []);
    }

    /// Nothing in the game goes down: a value read lower is a misreading put right.
    #[test]
    fn a_value_gone_down_is_not_counted() {
        let (before, after) = (trained(4, 5, 3), trained(3, 3, 1));
        let mut ledger = Ledger::default();
        ledger.frame();
        ledger.student(STUDENT, Some(&before), &before);
        ledger.frame();
        assert_eq!(ledger.trained(STUDENT, &before, &after), []);
    }

    fn offer(id: i64, left: u32) -> Offer {
        Offer {
            id,
            coin: 0,
            left,
            price: None,
        }
    }

    /// A purchase is counted as a training is: its offer read as known after the list.
    #[test]
    fn a_purchase_is_counted_once_its_offer_was_seen_after_the_list() {
        let shop = "総力戦";
        let known = [offer(1, 5), offer(2, 5)];
        let mut ledger = Ledger::default();
        ledger.frame();
        assert!(!ledger.bought(shop, 1, 0, List::Items), "not seen before");
        ledger.frame();
        // The second offer reads otherwise than known: no sight of it.
        ledger.offers(shop, &known, &[offer(1, 5), offer(2, 3)]);
        ledger.frame();
        assert!(ledger.bought(shop, 1, 0, List::Items));
        assert!(!ledger.bought(shop, 2, 0, List::Items), "not seen as known");
        ledger.frame();
        assert!(ledger.bought(shop, 1, 0, List::Items), "bought on at once");
        ledger.frame();
        ledger.listed(List::Items);
        ledger.frame();
        assert!(
            !ledger.bought(shop, 1, 0, List::Items),
            "the list shown since"
        );
        assert!(
            ledger.bought(shop, 2, 0, List::Equipment),
            "its list not shown"
        );
    }

    #[test]
    fn a_good_is_counted_in_its_list() {
        assert_eq!(counted(43), Some((List::Items, 43)));
        assert_eq!(
            counted(shop::EQUIPMENT_GOODS + 43),
            Some((List::Equipment, 43))
        );
        assert_eq!(counted(shop::CURRENCY_GOODS + 1), None);
    }
}
