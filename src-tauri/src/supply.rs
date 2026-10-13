//! What the training plan is short of, and how the choice boxes held make it
//! up.

use std::{collections::HashMap, hash::BuildHasher};

use serde::{Deserialize, Serialize};

use crate::plan::{Held, Kind, NeedRow};

/// A choice box (`data/schaledb/boxes.json`): one box gives one of its
/// choices, items or (`equipment`) blueprints.
#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceBox {
    pub id: i64,
    pub equipment: bool,
    pub choices: Vec<i64>,
}

/// A line of the plan that falls short: by how much, the boxes opened for it
/// (the box's id and how many) and what is still short after them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Short {
    pub kind: Kind,
    pub id: i64,
    pub short: u64,
    pub boxes: Vec<(i64, u64)>,
    pub left: u64,
}

/// The lines of `rows` held short of what they need (a line not counted left
/// out), each made up from the `boxes` held (`held` by box id): the boxes of
/// fewest choices first, so a box any school takes is kept for what the others
/// cannot give, and the lines in the plan's order.
pub fn cover<S: BuildHasher>(
    rows: &[NeedRow],
    boxes: &[ChoiceBox],
    held: &HashMap<i64, u64, S>,
) -> Vec<Short> {
    let mut shorts: Vec<Short> = rows
        .iter()
        .filter_map(|row| {
            let (Held::Exact(have) | Held::About(have)) = row.have?;
            (have < row.need).then(|| Short {
                kind: row.kind,
                id: row.id,
                short: row.need - have,
                boxes: Vec::new(),
                left: row.need - have,
            })
        })
        .collect();
    let mut order: Vec<&ChoiceBox> = boxes.iter().collect();
    order.sort_by_key(|b| (b.choices.len(), b.id));
    for choice_box in order {
        let mut ready = held.get(&choice_box.id).copied().unwrap_or(0);
        let kind = if choice_box.equipment {
            Kind::Equipment
        } else {
            Kind::Item
        };
        for short in shorts
            .iter_mut()
            .filter(|s| s.kind == kind && choice_box.choices.contains(&s.id))
        {
            let opened = ready.min(short.left);
            if opened > 0 {
                short.left -= opened;
                ready -= opened;
                short.boxes.push((choice_box.id, opened));
            }
        }
    }
    shorts
}

/// Makes up the elephs (`is_eleph`) still short from the fragments held (the
/// item `coin`), when they are `enough` for every eleph the plan buys with
/// them: `buy` tells, for an eleph and how many are short, the fragments they
/// cost and how many the shop sells (`None` where its rate was not read).
pub fn by_fragments(
    shorts: &mut [Short],
    coin: i64,
    enough: bool,
    is_eleph: impl Fn(i64) -> bool,
    buy: impl Fn(i64, u64) -> Option<(u64, u64)>,
) {
    if !enough {
        return;
    }
    for short in shorts
        .iter_mut()
        .filter(|s| s.kind == Kind::Item && s.left > 0 && is_eleph(s.id))
    {
        if let Some((cost, bought)) = buy(short.id, short.left) {
            short.left -= bought.min(short.left);
            short.boxes.push((coin, cost));
        }
    }
}

/// A good a shop sells: the shop and its coin (by index), the good, how many
/// times more it sells, at what price, and what it makes up: the plan's line
/// (kind and id) and how much of it one purchase gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForSale {
    pub shop: usize,
    pub coin: usize,
    pub good: i64,
    pub left: u32,
    pub price: u32,
    pub line: (Kind, i64),
    pub gives: u64,
}

/// What to buy of a good: where (as `ForSale`), how many times, for how many
/// coins, and how much of its line that makes up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bought {
    pub shop: usize,
    pub coin: usize,
    pub good: i64,
    pub times: u64,
    pub cost: u64,
    pub gives: u64,
}

/// Buys what the `shorts` still lack from the goods `for_sale`, in the order
/// given (what is worth buying first first): each as many times as its line
/// needs, the shop has left and the coins held (`coins[shop][coin]`, spent as
/// they go) pay for.
pub fn buy(shorts: &mut [Short], for_sale: &[ForSale], coins: &mut [Vec<u64>]) -> Vec<Bought> {
    let mut bought = Vec::new();
    for sale in for_sale {
        let Some(short) = shorts
            .iter_mut()
            .find(|s| (s.kind, s.id) == sale.line && s.left > 0)
        else {
            continue;
        };
        let Some(held) = coins.get_mut(sale.shop).and_then(|c| c.get_mut(sale.coin)) else {
            continue;
        };
        let price = u64::from(sale.price);
        let times = short
            .left
            .div_ceil(sale.gives.max(1))
            .min(u64::from(sale.left))
            .min(held.checked_div(price).unwrap_or(u64::MAX));
        if times == 0 {
            continue;
        }
        *held -= times * price;
        short.left = short.left.saturating_sub(times * sale.gives);
        bought.push(Bought {
            shop: sale.shop,
            coin: sale.coin,
            good: sale.good,
            times,
            cost: times * price,
            gives: times * sale.gives,
        });
    }
    bought
}

/// A line the plan falls short on, for the window: what it is (as its
/// `NeedRow`), by how much, the boxes opened for it and what is still short.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShortRow {
    pub kind: Kind,
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub short: u64,
    pub boxes: Vec<OpenedRow>,
    pub left: u64,
}

/// Boxes opened for a line: which, by name and icon, and how many.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedRow {
    pub name: String,
    pub icon: String,
    pub count: u64,
}

/// The 購買戦略 tab: what to buy shop by shop, what is still short after it,
/// and the shops not read this month, whose stock is not known.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BuyPlan {
    pub shops: Vec<ShopBuy>,
    pub left: Vec<LeftRow>,
    pub unread: Vec<String>,
}

/// A shop's purchases: its coins, held and spent, and the goods to buy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShopBuy {
    pub name: String,
    pub coins: Vec<CoinSpend>,
    pub goods: Vec<BuyRow>,
}

/// A shop's coin: how many are held (`None` not read) and spent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoinSpend {
    pub name: String,
    pub icon: String,
    pub held: Option<u64>,
    pub spent: u64,
}

/// A good to buy: with which coin (by index), how many times, for how many
/// coins, and how much of its line (`kind`: a count, credits or exp) that
/// makes up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuyRow {
    pub name: String,
    pub icon: String,
    pub coin: usize,
    pub times: u64,
    pub cost: u64,
    pub gives: u64,
    pub kind: Kind,
}

/// What is still short after the shops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeftRow {
    pub kind: Kind,
    pub name: String,
    pub icon: String,
    pub left: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: Kind, id: i64, need: u64, have: Option<Held>) -> NeedRow {
        NeedRow {
            kind,
            id,
            name: String::new(),
            icon: String::new(),
            family: None,
            need,
            have,
        }
    }

    fn choice(id: i64, equipment: bool, choices: &[i64]) -> ChoiceBox {
        ChoiceBox {
            id,
            equipment,
            choices: choices.to_vec(),
        }
    }

    #[test]
    fn the_boxes_of_fewest_choices_make_up_the_shortfalls_first() {
        let rows = [
            row(Kind::Item, 4000, 100, Some(Held::Exact(40))),
            row(Kind::Item, 4030, 50, Some(Held::Exact(20))),
            // Enough, or not counted: not short.
            row(Kind::Item, 4010, 10, Some(Held::Exact(10))),
            row(Kind::Credits, 0, 10, None),
            row(Kind::Item, 9999, 3, Some(Held::About(1000))),
        ];
        // Any school's box, and a box of Gehenna's and others' (4030 among them).
        let boxes = [
            choice(150_000, false, &[4000, 4010, 4030, 4040]),
            choice(150_012, false, &[4030, 4040]),
        ];
        let held = HashMap::from([(150_000, 70), (150_012, 25)]);
        assert_eq!(
            cover(&rows, &boxes, &held),
            [
                Short {
                    kind: Kind::Item,
                    id: 4000,
                    short: 60,
                    boxes: vec![(150_000, 60)],
                    left: 0,
                },
                Short {
                    kind: Kind::Item,
                    id: 4030,
                    short: 30,
                    // Gehenna's 25 first, then 5 of the 10 left of any school's.
                    boxes: vec![(150_012, 25), (150_000, 5)],
                    left: 0,
                },
            ]
        );
        // No more boxes than held are opened.
        let few = HashMap::from([(150_000, 10)]);
        let shorts = cover(&rows, &boxes, &few);
        assert_eq!(shorts.iter().map(|s| s.left).collect::<Vec<_>>(), [50, 30]);
    }

    #[test]
    fn the_fragments_held_make_up_the_elephs_when_enough_for_all() {
        let short = |id, n| Short {
            kind: Kind::Item,
            id,
            short: n,
            boxes: Vec::new(),
            left: n,
        };
        let shorts = || vec![short(10_005, 30), short(20_059, 386), short(4030, 5)];
        let is_eleph = |id: i64| id >= 10_000;
        // Aru's rate unread; the shop sells Mutsuki 300 more at 1 a fragment.
        let cost = |id: i64, n: u64| (id == 20_059).then(|| (n.min(300), n.min(300)));
        let mut enough = shorts();
        by_fragments(&mut enough, 23, true, is_eleph, cost);
        assert_eq!(
            enough
                .iter()
                .map(|s| (s.id, s.boxes.clone(), s.left))
                .collect::<Vec<_>>(),
            [
                (10_005, vec![], 30),
                (20_059, vec![(23, 300)], 86),
                (4030, vec![], 5)
            ]
        );
        // Short of fragments, they make up none.
        let mut few = shorts();
        by_fragments(&mut few, 23, false, is_eleph, cost);
        assert_eq!(few, shorts());
    }

    #[test]
    fn the_shops_sell_what_is_short_as_far_as_their_stock_and_the_coins_go() {
        let short = |id, n| Short {
            kind: Kind::Item,
            id,
            short: n,
            boxes: Vec::new(),
            left: n,
        };
        let sale = |shop, coin, good: i64, left, price, gives| ForSale {
            shop,
            coin,
            good,
            left,
            price,
            line: (Kind::Item, good),
            gives,
        };
        let mut shorts = vec![short(4033, 29), short(3032, 6), short(10_019, 12)];
        let for_sale = [
            // Elephs come 5 a purchase: 3 purchases make up 12.
            sale(0, 0, 10_019, 20, 50, 5),
            // 総力戦's plain coin sells 20 more notes; the rest from the rare coin.
            sale(0, 0, 4033, 20, 25, 1),
            sale(0, 1, 4033, 6, 25, 1),
            // Another shop's notes: what is left of the line, as far as its coins go.
            sale(1, 0, 4033, 20, 25, 1),
            sale(0, 0, 3032, 25, 50, 1),
        ];
        // 総力戦: 750 plain coins, 1,000 rare; 大決戦 50.
        let mut coins = vec![vec![750, 1000], vec![50]];
        let bought = buy(&mut shorts, &for_sale, &mut coins);
        let what: Vec<(usize, usize, i64, u64, u64, u64)> = bought
            .iter()
            .map(|b| (b.shop, b.coin, b.good, b.times, b.cost, b.gives))
            .collect();
        assert_eq!(
            what,
            [
                (0, 0, 10_019, 3, 150, 15),
                // 600 plain coins left buy 20 notes for 500; 100 left buy 2 BDs.
                (0, 0, 4033, 20, 500, 20),
                (0, 1, 4033, 6, 150, 6),
                (1, 0, 4033, 2, 50, 2),
                (0, 0, 3032, 2, 100, 2),
            ]
        );
        assert_eq!(coins, [vec![0, 850], vec![0]]);
        assert_eq!(shorts.iter().map(|s| s.left).collect::<Vec<_>>(), [1, 4, 0]);
    }

    #[test]
    fn a_box_of_blueprints_makes_up_blueprints_alone() {
        // The equipment list's ids are apart from the items'.
        let rows = [
            row(Kind::Item, 101_003, 5, Some(Held::Exact(0))),
            row(Kind::Equipment, 101_003, 5, Some(Held::Exact(1))),
        ];
        let boxes = [choice(150_030, true, &[101_003, 102_003])];
        let shorts = cover(&rows, &boxes, &HashMap::from([(150_030, 732)]));
        assert_eq!(
            shorts
                .iter()
                .map(|s| (s.kind, s.boxes.clone(), s.left))
                .collect::<Vec<_>>(),
            [
                (Kind::Item, vec![], 5),
                (Kind::Equipment, vec![(150_030, 4)], 0)
            ]
        );
    }
}
