//! The month-end shops' purchase plan (the user's of 2026-10-16, 「購買戦略」):
//! what each shop sells of the goods worth stocking up on before its lineup
//! changes, card by card as the game shows them, each reserved by a tap and
//! kept so, against the coins it takes.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{quantity::Count, shop::ShopRow};

/// The month-end shops, in the order the work tab lists them.
pub const SHOPS: [&str; 5] = ["総力戦", "大決戦", "合同火力演習", "指名手配", "熟達証書"];

/// A card reserved: its shop, its good and the coin it is paid with. A
/// reservation stays as the card sells out and as the month changes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Reserve {
    pub shop: String,
    pub id: i64,
    pub coin: String,
}

/// A month-end shop: when it was read (none: never), its coins and the cards
/// of the goods listed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanShop {
    pub name: String,
    pub read_at: Option<String>,
    pub coins: Vec<PlanCoin>,
    pub cards: Vec<PlanCard>,
}

/// A coin of a month-end shop: how many are held and how many the cards
/// reserved take, and whether that is more than held.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanCoin {
    pub name: String,
    pub icon: String,
    pub held: Option<Count>,
    pub need: u64,
    pub short: bool,
}

/// A card: a good and the coin (by its place in `PlanShop::coins`) paying for
/// it, the times left and the price; whether it is reserved, bought up, or
/// barred (an eleph its student can take no more of).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanCard {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub coin: usize,
    pub left: u32,
    pub price: Option<u32>,
    pub amount: Option<u32>,
    pub reserved: bool,
    pub sold: bool,
    pub barred: bool,
}

/// The month-end shops of `rows` (the shops' shelves as read), each with the
/// goods `listed` for it, the cards by their goods' `worth` (the least first),
/// bought up or not, but those `barred` last; flagged as `reserved` has them.
pub fn shops<K: Ord>(
    rows: &[ShopRow],
    listed: impl Fn(&str, i64) -> bool,
    worth: impl Fn(i64) -> K,
    barred: impl Fn(i64) -> bool,
    reserved: &BTreeSet<Reserve>,
) -> Vec<PlanShop> {
    SHOPS
        .iter()
        .map(|&name| {
            let Some(row) = rows.iter().find(|r| r.name == name) else {
                return PlanShop {
                    name: name.to_owned(),
                    read_at: None,
                    coins: Vec::new(),
                    cards: Vec::new(),
                };
            };
            let mut cards: Vec<PlanCard> = row
                .goods
                .iter()
                .filter(|g| listed(name, g.id))
                .flat_map(|g| {
                    g.offers.iter().enumerate().filter_map(|(coin, offer)| {
                        let offer = offer.as_ref()?;
                        Some(PlanCard {
                            id: g.id,
                            name: g.name.clone(),
                            icon: g.icon.clone(),
                            coin,
                            left: offer.left,
                            price: offer.price.map(|p| p.price),
                            amount: offer.price.map(|p| p.amount),
                            reserved: reserved.contains(&Reserve {
                                shop: name.to_owned(),
                                id: g.id,
                                coin: row.coins[coin].name.clone(),
                            }),
                            sold: offer.left == 0,
                            barred: barred(g.id),
                        })
                    })
                })
                .collect();
            cards.sort_by_key(|c| (c.barred, worth(c.id), c.coin));
            let coins = row
                .coins
                .iter()
                .enumerate()
                .map(|(at, coin)| {
                    let need: u64 = cards
                        .iter()
                        .filter(|c| c.coin == at && c.reserved)
                        .map(|c| u64::from(c.price.unwrap_or(0)) * u64::from(c.left))
                        .sum();
                    let held = coin.held.map(|count| match count {
                        Count::Exact(n) => u64::from(n),
                        Count::Thousands(k) => u64::from(k) * 1000,
                    });
                    PlanCoin {
                        name: coin.name.clone(),
                        icon: coin.icon.clone(),
                        held: coin.held,
                        need,
                        short: held.is_some_and(|held| need > held),
                    }
                })
                .collect();
            PlanShop {
                name: name.to_owned(),
                read_at: Some(row.read_at.clone()),
                coins,
                cards,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        price::Price,
        shop::{CoinRow, GoodRow, OfferRow},
    };

    fn coin(name: &str, held: u32) -> CoinRow {
        CoinRow {
            name: name.into(),
            icon: format!("icon_{name}"),
            held: Some(Count::Exact(held)),
            at: 0,
        }
    }

    fn offer(left: u32, price: u32) -> OfferRow {
        OfferRow {
            left,
            price: Some(Price {
                price,
                amount: 5,
                limit: 20,
            }),
            shown: None,
            coin: 0,
        }
    }

    fn good(id: i64, offers: Vec<Option<OfferRow>>) -> GoodRow {
        GoodRow {
            id,
            name: format!("good {id}"),
            icon: format!("icon_{id}"),
            sold: offers.iter().flatten().all(|o| o.left == 0),
            offers,
        }
    }

    /// 総力戦: a plain coin and a rare one; an eleph sold for both, a note, a
    /// report not listed, and an eleph whose student is done.
    fn assault() -> ShopRow {
        ShopRow {
            name: "総力戦".into(),
            read_at: "2026-10-16 07:00:00".into(),
            resets: true,
            coins: vec![coin("総力戦コイン", 600), coin("総力戦レアコイン", 40)],
            goods: vec![
                good(10_001, vec![Some(offer(4, 50)), Some(offer(2, 50))]),
                good(10_002, vec![Some(offer(4, 50)), None]),
                good(5, vec![Some(offer(3, 500)), None]),
                good(90, vec![Some(offer(20, 10)), None]),
                good(10_003, vec![Some(offer(0, 50)), None]),
            ],
        }
    }

    fn reserve(id: i64, coin: &str) -> Reserve {
        Reserve {
            shop: "総力戦".into(),
            id,
            coin: coin.into(),
        }
    }

    #[test]
    fn a_shop_lists_a_card_for_each_coin_of_each_good_listed_and_those_barred_last() {
        // The note is worth more than the elephs; what is bought up stays in place.
        let worth = |id: i64| (id != 5, id);
        let plan = shops(
            &[assault()],
            |_, id| id != 90,
            worth,
            |id| id == 10_002,
            &BTreeSet::new(),
        );
        assert_eq!(
            plan.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            SHOPS
        );
        let cards: Vec<(i64, usize, bool, bool)> = plan[0]
            .cards
            .iter()
            .map(|c| (c.id, c.coin, c.sold, c.barred))
            .collect();
        assert_eq!(
            cards,
            [
                (5, 0, false, false),
                (10_001, 0, false, false),
                (10_001, 1, false, false),
                (10_003, 0, true, false),
                (10_002, 0, false, true),
            ]
        );
        // A shop never read lists nothing.
        assert_eq!((plan[1].read_at.as_ref(), plan[1].cards.len()), (None, 0));
    }

    #[test]
    fn the_cards_reserved_take_their_price_for_each_time_left_of_their_coin() {
        let reserved = BTreeSet::from([
            reserve(10_001, "総力戦コイン"),
            reserve(5, "総力戦コイン"),
            reserve(10_001, "総力戦レアコイン"),
            // Bought up: still reserved, taking nothing.
            reserve(10_003, "総力戦コイン"),
        ]);
        let plan = shops(&[assault()], |_, _| true, |id| id, |_| false, &reserved);
        let coins: Vec<(u64, bool)> = plan[0].coins.iter().map(|c| (c.need, c.short)).collect();
        // 4 × 50 + 3 × 500 = 1,700 of the 600 plain coins; 2 × 50 of the 40 rare.
        assert_eq!(coins, [(1_700, true), (100, true)]);
        let flagged: Vec<(i64, usize)> = plan[0]
            .cards
            .iter()
            .filter(|c| c.reserved)
            .map(|c| (c.id, c.coin))
            .collect();
        assert_eq!(flagged, [(5, 0), (10_001, 0), (10_001, 1), (10_003, 0)]);
        let fewer = shops(
            &[assault()],
            |_, _| true,
            |id| id,
            |_| false,
            &BTreeSet::from([reserve(10_002, "総力戦コイン")]),
        );
        assert_eq!(
            (fewer[0].coins[0].need, fewer[0].coins[0].short),
            (200, false)
        );
    }
}
