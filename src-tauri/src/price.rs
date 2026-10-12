//! What each shop asks for its goods (`data/shops.yaml`): the price in coins,
//! how many come with a purchase and how many purchases it allows.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// A shop's terms: its coins as the screen shows them from the left, and its
/// rules, the first that fits a good giving its terms.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Terms {
    pub coins: Vec<String>,
    pub goods: Vec<Rule>,
}

/// A rule of `data/shops.yaml` (see its heading).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Rule {
    pub item: String,
    pub tier: Option<u8>,
    pub students: Option<String>,
    pub coin: Option<String>,
    pub price: u32,
    #[serde(default = "one")]
    pub amount: u32,
    pub limit: u32,
}

const fn one() -> u32 {
    1
}

/// What a good costs in a shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Price {
    pub price: u32,
    pub amount: u32,
    pub limit: u32,
}

/// An item as the rules see it (`data/schaledb/items.json`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Good {
    pub id: i64,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub subcategory: Option<String>,
    pub rarity: String,
}

/// A student as the rules see them (`data/schaledb/students.json`): how they
/// are had (0 always, 1 limited, 2 an event's, 3 fest).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Student {
    pub name: String,
    #[serde(default)]
    pub limited: u8,
}

/// The terms of every shop, and what the rules go by.
pub struct Prices {
    shops: BTreeMap<String, Terms>,
    goods: HashMap<i64, Good>,
    /// How the student of each eleph is had, by the eleph's id.
    elephs: HashMap<i64, u8>,
}

impl Prices {
    pub fn new(shops: BTreeMap<String, Terms>, goods: Vec<Good>, students: &[Student]) -> Self {
        // An eleph is "<student>の神名文字".
        let elephs = goods
            .iter()
            .filter_map(|g| {
                let student = g.name.strip_suffix(ELEPH)?;
                let had = students.iter().find(|s| s.name == student)?.limited;
                Some((g.id, had))
            })
            .collect();
        Self {
            shops,
            goods: goods.into_iter().map(|g| (g.id, g)).collect(),
            elephs,
        }
    }

    /// The coins of `shop`, left first (none for a shop without terms).
    pub fn coins(&self, shop: &str) -> &[String] {
        self.shops.get(shop).map_or(&[], |t| t.coins.as_slice())
    }

    /// What `shop` asks for the good `id` paid with its coin `coin` (0 the
    /// left one).
    pub fn find(&self, shop: &str, id: i64, coin: u8) -> Option<Price> {
        let terms = self.shops.get(shop)?;
        let good = self.goods.get(&id)?;
        let had = self.elephs.get(&id).copied();
        terms
            .goods
            .iter()
            .find(|rule| {
                let coin_fits = rule.coin.as_ref().is_none_or(|name| {
                    terms.coins.iter().position(|c| c == name) == Some(usize::from(coin))
                });
                coin_fits
                    && is(good, &rule.item)
                    && rule.tier.is_none_or(|t| tier(&good.rarity) == Some(t))
                    && rule
                        .students
                        .as_ref()
                        .is_none_or(|s| had.is_some_and(|h| HAD.get(s.as_str()) == Some(&h)))
            })
            .map(|rule| Price {
                price: rule.price,
                amount: rule.amount,
                limit: rule.limit,
            })
    }
}

const ELEPH: &str = "の神名文字";

/// How students are had, as the rules name it: 1 limited, 2 an event's, 3 fest.
static HAD: std::sync::LazyLock<HashMap<&str, u8>> =
    std::sync::LazyLock::new(|| HashMap::from([("限定", 1), ("配布", 2), ("フェス限定", 3)]));

/// Whether `good` is the item or of the kind a rule names.
fn is(good: &Good, item: &str) -> bool {
    let sub = good.subcategory.as_deref();
    match item {
        "神名文字" => good.category == "SecretStone",
        "戦術教育BD" => sub == Some("CDItem"),
        "技術ノート" => sub == Some("BookItem") && good.name.contains(item),
        "オーパーツ" => sub == Some("Artifact"),
        name => good.name == name,
    }
}

/// The tier a rarity stands for in a kind of goods (N 1 … SSR 4).
fn tier(rarity: &str) -> Option<u8> {
    ["N", "R", "SR", "SSR"]
        .iter()
        .position(|r| *r == rarity)
        .and_then(|i| u8::try_from(i + 1).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r"
総力戦:
  coins: [総力戦レアコイン, 総力戦コイン]
  goods:
    - { item: 神名文字, coin: 総力戦コイン, price: 50, amount: 5, limit: 20 }
    - { item: 神名文字, coin: 総力戦レアコイン, price: 50, amount: 5, limit: 10 }
    - { item: 秘伝ノート, coin: 総力戦コイン, price: 500, limit: 3 }
    - { item: 技術ノート, tier: 1, coin: 総力戦コイン, price: 3, limit: 50 }
    - { item: 技術ノート, tier: 4, coin: 総力戦コイン, price: 25, limit: 20 }
熟達証書:
  coins: [熟達証書]
  goods:
    - { item: 神名文字, students: 配布, price: 1800, amount: 5, limit: 6 }
    - { item: 神名文字, students: フェス限定, price: 2400, amount: 5, limit: 2 }
    - { item: オーパーツ, tier: 2, price: 9, amount: 2, limit: 15 }
";

    fn good(id: i64, name: &str, category: &str, subcategory: Option<&str>, rarity: &str) -> Good {
        Good {
            id,
            name: name.into(),
            category: category.into(),
            subcategory: subcategory.map(Into::into),
            rarity: rarity.into(),
        }
    }

    fn prices() -> Prices {
        Prices::new(
            serde_yaml::from_str(YAML).unwrap(),
            vec![
                good(10019, "アズサの神名文字", "SecretStone", None, "N"),
                good(10032, "ムツキ（正月）の神名文字", "SecretStone", None, "N"),
                good(10077, "キリノ（水着）の神名文字", "SecretStone", None, "N"),
                good(9999, "秘伝ノート", "Material", Some("BookItem"), "SSR"),
                good(
                    4000,
                    "初級技術ノート（百鬼夜行）",
                    "Material",
                    Some("BookItem"),
                    "N",
                ),
                good(
                    4003,
                    "最上級技術ノート（百鬼夜行）",
                    "Material",
                    Some("BookItem"),
                    "SSR",
                ),
                good(
                    201,
                    "破損したトーテムポール",
                    "Material",
                    Some("Artifact"),
                    "R",
                ),
            ],
            &[
                Student {
                    name: "アズサ".into(),
                    limited: 0,
                },
                Student {
                    name: "ムツキ（正月）".into(),
                    limited: 3,
                },
                Student {
                    name: "キリノ（水着）".into(),
                    limited: 2,
                },
            ],
        )
    }

    fn price(price: u32, amount: u32, limit: u32) -> Price {
        Price {
            price,
            amount,
            limit,
        }
    }

    #[test]
    fn a_good_takes_the_first_rule_of_its_kind_tier_and_coin() {
        let p = prices();
        assert_eq!(p.coins("総力戦"), ["総力戦レアコイン", "総力戦コイン"]);
        // Coin 1 is the right one, 総力戦コイン.
        assert_eq!(p.find("総力戦", 10019, 1), Some(price(50, 5, 20)));
        assert_eq!(p.find("総力戦", 10019, 0), Some(price(50, 5, 10)));
        assert_eq!(p.find("総力戦", 9999, 1), Some(price(500, 1, 3)));
        assert_eq!(p.find("総力戦", 4000, 1), Some(price(3, 1, 50)));
        assert_eq!(p.find("総力戦", 4003, 1), Some(price(25, 1, 20)));
        // No rule for the rare coin's notes, nor for a shop without terms.
        assert_eq!(p.find("総力戦", 4000, 0), None);
        assert_eq!(p.find("カケラ(文字)", 10019, 0), None);
    }

    #[test]
    fn an_eleph_goes_by_how_its_student_is_had() {
        let p = prices();
        assert_eq!(p.find("熟達証書", 10077, 0), Some(price(1800, 5, 6)));
        assert_eq!(p.find("熟達証書", 10032, 0), Some(price(2400, 5, 2)));
        assert_eq!(p.find("熟達証書", 10019, 0), None);
        assert_eq!(p.find("熟達証書", 201, 0), Some(price(9, 2, 15)));
    }

    #[test]
    fn the_shipped_terms_read() {
        let terms: BTreeMap<String, Terms> =
            serde_yaml::from_str(include_str!("../data/shops.yaml")).unwrap();
        for shop in crate::shop::SHOPS {
            assert!(terms.contains_key(shop), "{shop}");
        }
    }
}
