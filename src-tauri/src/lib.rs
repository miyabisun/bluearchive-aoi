//! Reads kinds and counts from screenshots of the Blue Archive item list and
//! equipment list, and students' training from the student screen.
//!
//! Both lists share the cutting of cells from the grey list (`grid`), the screen
//! checks (`screen`), the counts (`quantity`) and the telling apart of icons
//! (`icon`, `catalog`); what is on each list and how it is ordered is decided
//! apart (`items`, `equipment`).

pub mod app;
pub mod catalog;
pub mod credit;
pub mod equipment;
pub mod export;
pub mod grid;
pub mod icon;
pub mod items;
pub mod keep;
pub mod ocr;
mod order;
pub mod plan;
pub mod price;
pub mod quantity;
pub mod screen;
pub mod shop;
pub mod student;
pub mod supply;
pub mod tally;
pub mod update;
pub mod watch;
pub mod work;
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// The picture of the item or equipment icon `name` built into the program
/// (`data/schaledb/icons`, `data/schaledb/equipment-icons`, `data/extra/icons`,
/// `data/extra/equipment-icons`), with its media type.
pub fn icon(name: &str) -> Option<(&'static [u8], &'static str)> {
    [
        ("schaledb/icons/", ".webp", "image/webp"),
        ("schaledb/equipment-icons/", ".webp", "image/webp"),
        ("extra/icons/", ".png", "image/png"),
        ("extra/equipment-icons/", ".png", "image/png"),
    ]
    .into_iter()
    .find_map(|(dir, ext, media)| {
        let path = format!("{dir}{name}{ext}");
        embedded::FILES
            .iter()
            .find(|(file, _)| *file == path)
            .map(|(_, bytes)| (*bytes, media))
    })
}

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use image::{RgbImage, RgbaImage};
use serde::{Deserialize, Serialize};

use catalog::Catalog;
use quantity::{Count, Digits};

/// One fully visible cell of a list, in reading order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// SchaleDB id within the list, or `None` when the icon could not be told apart.
    pub id: Option<i64>,
    /// Count shown as "xN", or `None` when it could not be read.
    pub quantity: Option<Count>,
    /// An entry the user does not keep track of: the cell is read for the list
    /// order but is not to be shown or exported.
    pub ignored: bool,
}

/// The lists that are read. Their ids overlap: an id names an entry only
/// together with its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum List {
    Items,
    Equipment,
}

/// An entry of the SchaleDB snapshot (`data/schaledb/items.json`,
/// `data/schaledb/equipment.json`), or of the items it lacks
/// (`data/extra/items.json`).
#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub category: String,
    /// Equipment tier (1 to 10); 0 for items and for equipment without one.
    #[serde(default)]
    pub tier: u8,
    /// None for the items SchaleDB lacks.
    #[serde(default)]
    pub rarity: Option<icon::Rarity>,
}

/// What a frame shows that can be read.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A list in its default order, cell by cell, and whether its start shows.
    List(List, Vec<Cell>, bool),
    /// The student screen: the student (`None` when the name could not be
    /// told) and the training shown.
    Student(Option<i64>, student::Training),
    /// A shop and its offers.
    Shop(shop::Seen),
}

/// A student of the SchaleDB snapshot (`data/schaledb/students.json`).
#[derive(Deserialize, Debug, Clone)]
pub struct StudentEntry {
    pub id: i64,
    pub name: String,
}

/// A student the snapshot lacks (`data/extra/students.json`), known by name
/// until SchaleDB lists them. Such a student takes an id of Aoi's own below
/// zero, which an export writes as no id.
#[derive(Deserialize, Debug, Clone)]
struct ExtraStudent {
    name: String,
}

/// A currency of the SchaleDB snapshot (`data/schaledb/currency.json`), for
/// the shops' coins that are no item.
#[derive(Deserialize, Debug, Clone)]
struct Currency {
    name: String,
    icon: String,
}

/// One line of a tally, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub id: i64,
    pub name: String,
    /// The icon's name (see `icon`).
    pub icon: String,
    pub tier: Option<u8>,
    pub count: Count,
}

/// Reads a data file by its path under `data`.
type Read<'a> = dyn Fn(&str) -> Result<Vec<u8>, String> + 'a;

fn json<T: for<'de> Deserialize<'de>>(read: &Read, path: &str) -> Result<T, String> {
    serde_json::from_slice(&read(path)?).map_err(|e| format!("{path}: {e}"))
}

fn picture(read: &Read, path: &str) -> Result<RgbaImage, String> {
    image::load_from_memory(&read(path)?)
        .map(|i| i.to_rgba8())
        .map_err(|e| format!("{path}: {e}"))
}

pub struct Recognizer {
    items: Catalog,
    /// The shops' goods (the items and the few the item list lacks), what
    /// tells them apart, and what the shops ask for them.
    shop_goods: Vec<Item>,
    goods: shop::Goods,
    prices: price::Prices,
    currency: Vec<Currency>,
    stages: work::Stages,
    /// What one student takes of each item (see `work::person_needs`).
    person_needs: HashMap<i64, u64>,
    /// The growth data training plans are counted with.
    planner: plan::Planner,
    /// The choice boxes that make up what a plan is short of.
    boxes: Vec<supply::ChoiceBox>,
    equipment: Catalog,
    digits: Digits,
    /// The sort buttons in the default order (`data/sort-default.png`).
    sort_template: RgbImage,
    /// Each list's screen title (`data/title-*.png`).
    titles: Vec<(List, RgbImage)>,
    students: Vec<(i64, String)>,
    /// The student screen's title and its selected "基本情報" tab.
    /// The student screen's title, its selected 基本情報 tab, and the same tab
    /// while another is selected.
    student_screen: (RgbImage, RgbImage, RgbImage),
    /// References for the short texts of the student screen.
    fonts: student::glyph::Fonts,
    /// The OCR reading students' names; without it (no Japanese OCR on this
    /// Windows) the student screen is not read.
    ocr: Option<ocr::Ocr>,
}

impl Recognizer {
    /// Loads the files under `data` (the crate's `data` directory): the SchaleDB
    /// snapshot (`schaledb`), the items and students it lacks (`extra`, icons
    /// cut from screenshots), the count references and the screen templates.
    pub fn load(data: &Path) -> Result<Self, String> {
        Self::build(&|path| {
            std::fs::read(data.join(path))
                .map_err(|e| format!("{}: {e}", data.join(path).display()))
        })
    }

    /// The same data, built into the program.
    pub fn embedded() -> Result<Self, String> {
        Self::build(&|path| {
            embedded::FILES
                .iter()
                .find(|(name, _)| *name == path)
                .map(|(_, bytes)| bytes.to_vec())
                .ok_or_else(|| format!("{path} is not built in"))
        })
    }

    fn build(read: &Read) -> Result<Self, String> {
        let template =
            |path: &str| picture(read, path).map(|p| image::DynamicImage::from(p).to_rgb8());
        let planner = plan::Planner {
            growth: json(read, "schaledb/growth.json")?,
            growers: json(read, "schaledb/students.json")?,
            pieces: json(read, "schaledb/equipment.json")?,
        };
        let terms: BTreeMap<String, price::Terms> =
            serde_yaml::from_slice(&read("shops.yaml")?).map_err(|e| format!("shops.yaml: {e}"))?;
        let (shop_goods, goods) = items::goods(read, |name| {
            terms
                .values()
                .flat_map(|t| &t.goods)
                .any(|rule| rule.item == name)
        })?;
        // The goods the item list lacks, as the shops' terms see them.
        let mut priced: Vec<price::Good> = json(read, "schaledb/items.json")?;
        priced.extend(
            shop_goods
                .iter()
                .filter(|g| g.id >= shop::EQUIPMENT_GOODS)
                .map(|g| price::Good {
                    id: g.id,
                    name: g.name.clone(),
                    category: g.category.clone(),
                    subcategory: None,
                    rarity: g.rarity.map(|r| format!("{r:?}")).unwrap_or_default(),
                }),
        );
        Ok(Self {
            person_needs: work::person_needs(&planner.growers, &planner.growth),
            planner,
            items: items::catalog(read)?,
            shop_goods,
            goods,
            prices: price::Prices::new(
                terms,
                priced,
                &json::<Vec<price::Student>>(read, "schaledb/students.json")?,
            ),
            currency: json(read, "schaledb/currency.json")?,
            stages: json(read, "schaledb/stages.json")?,
            boxes: json(read, "schaledb/boxes.json")?,
            equipment: equipment::catalog(read)?,
            digits: json(read, "digits.json")?,
            sort_template: template("sort-default.png")?,
            titles: vec![
                (List::Items, template("title-items.png")?),
                (List::Equipment, template("title-equipment.png")?),
            ],
            students: json::<Vec<StudentEntry>>(read, "schaledb/students.json")?
                .into_iter()
                .map(|s| (s.id, s.name))
                .chain(
                    json::<Vec<ExtraStudent>>(read, "extra/students.json")?
                        .into_iter()
                        .zip(1..)
                        .map(|(s, n): (ExtraStudent, i64)| (-n, s.name)),
                )
                .collect(),
            student_screen: (
                template("title-students.png")?,
                template("student-tab.png")?,
                template("student-tab-other.png")?,
            ),
            fonts: json(read, "student-fonts.json")?,
            ocr: ocr::Ocr::japanese().ok(),
        })
    }

    /// The work tab against the items and equipment held and the credits.
    pub fn work_rows(
        &self,
        items: &[Row],
        equipment: &[Row],
        credits: Option<u64>,
    ) -> work::WorkRows {
        let counts = |rows: &[Row]| -> HashMap<i64, Count> {
            rows.iter().map(|r| (r.id, r.count)).collect()
        };
        let (items_held, equipment_held) = (counts(items), counts(equipment));
        let (goods, pieces) = (self.items.entries(), self.equipment.entries());
        let stages = &self.stages;
        work::WorkRows {
            slots: work::slots(&self.planner.pieces, pieces, &equipment_held),
            // The missions are played in a campaign, which doubles the drops.
            farm: work::farm(
                &self.planner.pieces,
                pieces,
                &stages.missions,
                &equipment_held,
                2,
            ),
            bounties: work::places(
                &stages.bounties,
                goods,
                |e| {
                    self.prices
                        .family(e.id)
                        .map(|(family, _)| family.to_string())
                },
                &items_held,
                &self.person_needs,
            ),
            scrimmages: work::places(
                &stages.scrimmages,
                pieces,
                |e| Some(e.category.clone()),
                &equipment_held,
                &HashMap::<i64, u64>::new(),
            ),
            commissions: work::commissions(
                &stages.commissions,
                credits,
                self.raise_cost(),
                &self.planner.growth.reports,
                &items_held,
            ),
        }
    }

    /// The credits and the report exp a student takes from Lv1, untrained, to
    /// the default target with the unique weapon's 4th star and its level cap
    /// (the user's measure, 2026-10-13: level, skills, equipment, stars and the
    /// weapon, their credits); the first student SchaleDB lists stands for all.
    fn raise_cost(&self) -> (u64, u64) {
        /// The unique weapon's level cap at its 4th star.
        const WEAPON_TOP: usize = 60;
        let Some(grower) = self.planner.growers.first() else {
            return (0, 0);
        };
        let fresh = student::Training {
            level: Some(1),
            rarity: Some(1),
            bond: Some(1),
            skills: [Some(1); 4],
            weapon: Some(None),
            equipment: [Some(student::Gear { tier: 1, level: 1 }); 3],
            gear: Some(0),
            ability: [Some(0); 3],
        };
        let target = plan::Target {
            star: Some(8),
            ..plan::Target::default()
        };
        let needs = self.planner.needs(
            &BTreeMap::from([(grower.id, target)]),
            &BTreeMap::from([(grower.id, fresh)]),
        );
        let growth = &self.planner.growth;
        let weapon =
            growth.weapon_exp.get(WEAPON_TOP - 1).copied().unwrap_or(0) * growth.weapon_credits;
        (needs.credits + weapon, needs.report_exp)
    }

    /// What the `targets` take against what is held, line by line (credits,
    /// report exp, orb exp, then items in id order); the blueprints slot by
    /// slot; and the parts that could not be counted, their students named.
    pub fn plan_rows(
        &self,
        targets: &BTreeMap<i64, plan::Target>,
        students: &[watch::StudentRow],
        items: &[Row],
        equipment: &[Row],
        credits: Option<u64>,
    ) -> (
        Vec<plan::NeedRow>,
        Vec<plan::BlueprintRow>,
        Vec<plan::UnknownRow>,
    ) {
        use plan::{Held, Kind, NeedRow, held};
        let trained = students.iter().map(|s| (s.id, s.training)).collect();
        let needs = self.planner.needs(targets, &trained);
        let counts = |rows: &[Row]| -> HashMap<i64, Count> {
            rows.iter().map(|r| (r.id, r.count)).collect()
        };
        let (items_held, equipment_held) = (counts(items), counts(equipment));
        let growth = &self.planner.growth;
        let line = |kind, id, entry: Option<&Item>, need, have| NeedRow {
            kind,
            id,
            name: entry.map(|e| e.name.clone()).unwrap_or_default(),
            icon: entry.map(|e| e.icon.clone()).unwrap_or_default(),
            family: (kind == Kind::Item)
                .then(|| self.prices.family(id))
                .flatten(),
            need,
            have,
        };
        let entry = |list: List, id: i64| self.catalog(list).entries().iter().find(|e| e.id == id);
        let mut rows = vec![
            line(
                Kind::Credits,
                0,
                None,
                needs.credits,
                credits.map(Held::Exact),
            ),
            line(
                Kind::ReportExp,
                0,
                None,
                needs.report_exp,
                Some(held(&items_held, &growth.reports)),
            ),
            line(
                Kind::OrbExp,
                0,
                None,
                needs.orb_exp,
                Some(held(&equipment_held, &growth.orbs)),
            ),
        ];
        rows.retain(|r| r.need > 0);
        rows.extend(needs.items.iter().map(|(&id, &need)| {
            line(
                Kind::Item,
                id,
                entry(List::Items, id),
                need,
                Some(held(&items_held, &[(id, 1)])),
            )
        }));
        let blueprints = plan::blueprints(
            &needs.equipment,
            &self.planner.pieces,
            self.equipment.entries(),
            &equipment_held,
        );
        let unknown = needs
            .unknown
            .into_iter()
            .map(|(id, part)| plan::UnknownRow {
                id,
                name: students
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                part,
            })
            .collect();
        (rows, blueprints, unknown)
    }

    /// The lines of the `plan` that fall short, each with what could make it
    /// up (see `supply::Source`): the choice boxes (fewest choices first, the
    /// `items` counting those held), for elephs the fragments at the rate the
    /// カケラ(文字) shop showed, and the goods the `shops` read this month
    /// (`now` local time) sell, shop by shop in the menu's order, each coin from
    /// the plain one; then the `blueprints` short (see `supply::blueprint_lines`).
    pub fn procure_rows(
        &self,
        plan: &[plan::NeedRow],
        blueprints: &[plan::BlueprintRow],
        items: &[Row],
        shops: &[shop::ShopRow],
        now: &str,
    ) -> Vec<supply::ProcureLine> {
        let held = |id: i64| amount(items.iter().find(|r| r.id == id).map(|r| r.count));
        let read: Vec<&shop::ShopRow> = shops
            .iter()
            .filter(|s| s.name != shop::LASTING)
            .filter(|s| !(s.resets && s.read_at.get(..7) < now.get(..7)))
            .collect();
        let for_sale = self.for_sale(&read);
        let entries = self.items.entries();
        let named = |id: i64| entries.iter().find(|e| e.id == id);
        let fragments = self.coin_item(shop::LASTING, 0);
        let source = |b: &supply::ChoiceBox| supply::Source::Box {
            id: b.id,
            name: named(b.id).map(|e| e.name.clone()).unwrap_or_default(),
            icon: named(b.id).map(|e| e.icon.clone()).unwrap_or_default(),
            held: held(b.id),
            choices: b.choices.len(),
        };
        let mut lines: Vec<supply::ProcureLine> = supply::shorts(plan)
            .into_iter()
            .map(|(kind, id, short)| {
                let line = plan.iter().find(|r| r.kind == kind && r.id == id);
                let boxes = supply::boxes_for(kind, id, &self.boxes).into_iter();
                let mut sources: Vec<supply::Source> = boxes.map(source).collect();
                let eleph = kind == plan::Kind::Item && self.prices.is_kind(id, "神名文字");
                if let (true, Some(coin), Some(offer)) = (eleph, fragments, kakera_offer(shops, id))
                    && let (Some(rate), true) = (offer.shown, offer.left > 0)
                {
                    sources.push(supply::Source::Fragments {
                        name: named(coin).map(|e| e.name.clone()).unwrap_or_default(),
                        icon: named(coin).map(|e| e.icon.clone()).unwrap_or_default(),
                        held: held(coin),
                        rate,
                        left: offer.left,
                    });
                }
                sources.extend(for_sale.iter().filter(|f| f.line == (kind, id)).map(|f| {
                    let coin = &read[f.shop].coins[f.coin];
                    supply::Source::Shop {
                        shop: read[f.shop].name.clone(),
                        coin: coin.name.clone(),
                        icon: coin.icon.clone(),
                        held: coin.held.map(|c| amount(Some(c))),
                        price: f.price,
                        gives: f.gives,
                        left: f.left,
                    }
                }));
                supply::ProcureLine {
                    kind,
                    id,
                    name: line.map(|r| r.name.clone()).unwrap_or_default(),
                    icon: line.map(|r| r.icon.clone()).unwrap_or_default(),
                    short,
                    sources,
                }
            })
            .collect();
        lines.extend(supply::blueprint_lines(blueprints, &self.boxes, source));
        lines
    }

    /// The goods the `shops` sell, each coin from the plain one and the goods
    /// in the shop's order, and what each makes up: an item, the credits
    /// (クレジットポイント), or the exp of a report.
    fn for_sale(&self, shops: &[&shop::ShopRow]) -> Vec<supply::ForSale> {
        let credits = shop::CURRENCY_GOODS + 1;
        let mut for_sale = Vec::new();
        for (at, row) in shops.iter().enumerate() {
            for coin in 0..row.coins.len() {
                for good in &row.goods {
                    let Some(Some(offer)) = good.offers.get(coin) else {
                        continue;
                    };
                    let Some(price) = offer.price.filter(|_| offer.left > 0) else {
                        continue;
                    };
                    let gives = u64::from(price.amount);
                    let report = self.planner.growth.reports.iter().find(|r| r.0 == good.id);
                    let (line, gives) = if good.id == credits {
                        ((plan::Kind::Credits, 0), gives)
                    } else if let Some(&(_, exp)) = report {
                        ((plan::Kind::ReportExp, 0), gives * exp)
                    } else {
                        ((plan::Kind::Item, good.id), gives)
                    };
                    for_sale.push(supply::ForSale {
                        shop: at,
                        coin,
                        good: good.id,
                        left: offer.left,
                        price: price.price,
                        line,
                        gives,
                    });
                }
            }
        }
        for_sale
    }

    /// The item a shop's coin is (as `shop::Seen::held` tells it), where it is one.
    pub fn coin_item(&self, shop: &str, coin: usize) -> Option<i64> {
        let name = self.prices.coins(shop).get(coin)?;
        let items = self.items.entries();
        items.iter().find(|i| &i.name == name).map(|i| i.id)
    }

    /// The shops' goods: the items and the few the item list lacks.
    pub fn shop_goods(&self) -> &[Item] {
        &self.shop_goods
    }

    /// The shelves in the menu's order, their offers' items named; a coin that
    /// is an item held as `items` count it.
    pub fn shop_rows(
        &self,
        shelves: &BTreeMap<String, shop::Shelf>,
        items_held: &HashMap<i64, Count>,
    ) -> Vec<shop::ShopRow> {
        let items = self.items.entries();
        let good = |id: i64| self.shop_goods.iter().find(|g| g.id == id);
        let named = |id: i64| good(id).map(|g| g.name.clone()).unwrap_or_default();
        let item = |name: &str| items.iter().find(|i| i.name == name);
        let icon = |id: i64| good(id).map(|g| g.icon.clone());
        shop::SHOPS
            .iter()
            .filter_map(|&name| {
                let kept = shelves.get(name)?;
                let coins = self.prices.coins(name);
                // The coins in id order: the plain coin before the rare.
                let mut order: Vec<usize> = (0..coins.len()).collect();
                order.sort_by_key(|&i| item(&coins[i]).map_or(i64::MAX, |e| e.id));
                let coin_rows = order
                    .iter()
                    .map(|&i| shop::CoinRow {
                        held: match item(&coins[i]) {
                            Some(e) => items_held.get(&e.id).copied(),
                            None => kept
                                .held
                                .get(i)
                                .copied()
                                .flatten()
                                .and_then(|n| u32::try_from(n).ok())
                                .map(Count::Exact),
                        },
                        at: u8::try_from(i).unwrap_or_default(),
                        name: coins[i].clone(),
                        icon: item(&coins[i])
                            .map(|e| e.icon.clone())
                            .or_else(|| {
                                let c = self.currency.iter().find(|c| c.name == coins[i]);
                                c.map(|c| c.icon.clone())
                            })
                            .unwrap_or_default(),
                    })
                    .collect();
                let mut goods: Vec<_> = shop::goods(&kept.offers, coins.len())
                    .into_iter()
                    .map(|(id, offers)| shop::GoodRow {
                        id,
                        name: named(id),
                        icon: icon(id).unwrap_or_default(),
                        sold: offers.iter().flatten().all(|o| o.left == 0),
                        offers: order
                            .iter()
                            .map(|&i| {
                                offers[i].map(|o| shop::OfferRow {
                                    left: o.left,
                                    price: self.prices.find(name, id, o.coin, o.price),
                                    shown: o.price,
                                    coin: o.coin,
                                })
                            })
                            .collect(),
                    })
                    .collect();
                // What is worth buying first, those the plain coin buys before
                // the rare's; the goods bought up last.
                goods.sort_by_key(|g| {
                    let (kind, had) = self.prices.rank(g.id);
                    let coin = g.offers.iter().position(Option::is_some);
                    (g.sold, kind, coin, had, g.id)
                });
                Some(shop::ShopRow {
                    name: name.to_owned(),
                    read_at: kept.read_at.clone(),
                    resets: name != shop::LASTING,
                    coins: coin_rows,
                    goods,
                })
            })
            .collect()
    }

    /// Whether some good of `shop` costs what its card shows (read with it).
    pub fn reads_price(&self, shop: &str) -> bool {
        self.prices.reads_price(shop)
    }

    /// The shop screen (see `shop::Goods::read`); `None` on any other screen,
    /// and without the OCR.
    pub fn read_shop(&self, image: &RgbImage) -> Option<shop::Seen> {
        self.goods.read(image, self.ocr.as_ref()?, &self.prices)
    }

    pub fn students(&self) -> &[(i64, String)] {
        &self.students
    }

    /// Reads whatever the frame shows that can be read: a list in its default
    /// order, the student screen's 基本情報 tab, or a shop.
    pub fn read_frame(&self, image: &RgbImage) -> Option<Frame> {
        if let Some((list, cells, start)) = self.read_list(image) {
            return Some(Frame::List(list, cells, start));
        }
        if let Some((id, training)) = self.read_student(image) {
            return Some(Frame::Student(id, training));
        }
        self.read_shop(image).map(Frame::Shop)
    }

    /// The credits the top bar shows; `None` where there is no bar, or its
    /// number reads as no such number.
    pub fn read_credits(&self, image: &RgbImage) -> Option<u64> {
        let ocr = self.ocr.as_ref()?;
        if !credit::bar_shows(image) {
            return None;
        }
        let (x0, y0, x1, y1) = student::rect(image, credit::NUMBER)?;
        let number = image::imageops::crop_imm(image, x0, y0, x1 - x0, y1 - y0).to_image();
        credit::parse(&ocr.read(&number).ok()?)
    }

    /// Reads the student screen: all of its 基本情報 tab, and the name plate
    /// (level, rarity, bond) on its other tabs, where a student is levelled up;
    /// any other screen gives `None`.
    pub fn read_student(&self, image: &RgbImage) -> Option<(Option<i64>, student::Training)> {
        let ocr = self.ocr.as_ref()?;
        let (title, tab, other_tab) = &self.student_screen;
        let unit = (image.height() as f32 * screen::STUDENT_UNIT).round() as u32;
        screen::title(image, unit, std::slice::from_ref(title))?;
        let basic = screen::student_tab(image, tab);
        if !basic && !screen::student_tab(image, other_tab) {
            return None;
        }
        let (x0, y0, x1, y1) = student::rect(image, student::NAME)?;
        let name = image::imageops::crop_imm(image, x0, y0, x1 - x0, y1 - y0).to_image();
        // The letters alone read surest; a few names read only as they are.
        let id = [student::name_letters(&name), name]
            .iter()
            .find_map(|picture| {
                let text = ocr.read(picture).ok()?;
                student::name::find(&text, &self.students)
            });
        let training = if basic {
            student::read(image, &self.fonts)
        } else {
            student::plate(image, &self.fonts)
        };
        Some((id, training))
    }

    pub fn catalog(&self, list: List) -> &Catalog {
        match list {
            List::Items => &self.items,
            List::Equipment => &self.equipment,
        }
    }

    /// The tally's tracked counts of `list`, in the default list order.
    pub fn rows(&self, list: List, tally: &tally::Tally) -> Vec<Row> {
        self.catalog(list).rows(tally.counts())
    }

    /// Reads the fully visible cells in reading order as cells of `list`.
    pub fn read(&self, image: &RgbImage, list: List) -> Vec<Cell> {
        self.catalog(list)
            .read_cards(image, &grid::find_cards(image), &self.digits, None)
    }

    /// Reads the cells, and whether the list's start shows, when the frame
    /// shows a list sorted the default way, ascending; any other screen or
    /// order gives `None`.
    pub fn read_list(&self, image: &RgbImage) -> Option<(List, Vec<Cell>, bool)> {
        let (panel, cards) = grid::find(image)?;
        if !screen::sorted_by_default(image, panel.card_height, &self.sort_template) {
            return None;
        }
        let pictures: Vec<RgbImage> = self.titles.iter().map(|(_, t)| t.clone()).collect();
        let list = self.titles[screen::title(image, panel.card_height, &pictures)?].0;
        let cells = self
            .catalog(list)
            .read_cards(image, &cards, &self.digits, None);
        Some((list, cells, panel.shows_start))
    }
}

/// A count as a number, about as many where it is in thousands; none 0.
fn amount(count: Option<Count>) -> u64 {
    match count {
        Some(Count::Exact(n)) => u64::from(n),
        Some(Count::Thousands(k)) => u64::from(k) * 1000,
        None => 0,
    }
}

/// What the カケラ(文字) shop of `shops` sells of the eleph `id`.
fn kakera_offer(shops: &[shop::ShopRow], id: i64) -> Option<shop::OfferRow> {
    shops
        .iter()
        .find(|s| s.name == shop::LASTING)?
        .goods
        .iter()
        .find(|g| g.id == id)?
        .offers
        .first()
        .copied()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        image: String,
        list: List,
        cells: Vec<Label>,
    }

    #[derive(Deserialize)]
    struct Label {
        id: Option<i64>,
    }

    /// An entry missing from the snapshot (a new release) must never be read
    /// as a look-alike, nor make other cells wrong. Each labeled entry is
    /// removed in turn and the whole screenshot is read again.
    #[test]
    fn an_entry_missing_from_the_snapshot_is_never_misread() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let recognizer = Recognizer::load(&root.join("data")).unwrap();
        let fixtures: Vec<Fixture> = serde_json::from_str(
            &std::fs::read_to_string(root.join("tests/fixtures/labels.json")).unwrap(),
        )
        .unwrap();
        let mut wrong = Vec::new();
        for fixture in fixtures {
            let image = image::open(root.join("tests/fixtures").join(&fixture.image))
                .unwrap()
                .to_rgb8();
            let catalog = recognizer.catalog(fixture.list);
            let cards = grid::find_cards(&image);
            for id in fixture.cells.iter().filter_map(|l| l.id) {
                let missing = catalog.icon_of(id);
                let read = catalog.read_cards(&image, &cards, &recognizer.digits, missing);
                for (cell, label) in read.iter().zip(&fixture.cells) {
                    let expected = label.id.filter(|&l| l != id);
                    if cell.id.is_some() && cell.id != expected {
                        wrong.push(format!(
                            "{} without {id}: read {:?} where {:?} is",
                            fixture.image, cell.id, label.id
                        ));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }
}
