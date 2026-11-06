//! What the window shows of a snapshot: the labels, cells and tones its
//! screens draw, as the Svelte page's `view.js` derived them. Pure functions:
//! nothing here draws, reads or keeps anything.

use std::{collections::BTreeMap, ops::Range};

use icu_normalizer::ComposingNormalizerBorrowed;

use crate::{
    Row,
    android::{Link, Via},
    broadcast::Cast,
    ground::Behind,
    icon::Rarity,
    kinds::{Enhancement, SkillBoxes},
    plan::{BlueprintRow, Held, Kind, NeedRow, Part, UnknownRow},
    quantity::Count,
    shop::{self, GoodRow, ShopRow},
    strategy::{PlanCard, PlanCoin, PlanShop},
    student::Training,
    supply::{ProcureLine, Source},
    watch::{Place, Screen, Status, StudentRow},
    work::{CommissionRow, Farm, Golden, HardCard, HeldCell, PlaceRow, SchoolRow, Stock},
};

/// A student's star counted on into the unique weapon's (★5 with it at ★2 is 6), `None` unread.
pub use crate::plan::star_of;

/// A colour the page draws a badge, a cell or a row in (its classes and `data-tone`s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The status badge while reading.
    Active,
    /// The status badge with nothing to read.
    Idle,
    /// The status badge when the capture fails.
    Danger,
    /// The status badge while the user pauses the reading.
    Paused,
    /// Enough held (green).
    Enough,
    /// Short of what is needed (red).
    Short,
    /// Short of twice what one student takes (blue).
    Fair,
    /// Short of a slot's golden ratio (orange).
    Low,
    /// A resource left over once paid from (green).
    Plus,
    /// A resource overspent (red).
    Minus,
    /// The row that raises the fewest students, the one to play.
    Fewer,
    /// A row that raises more students than another.
    More,
    /// The school whose parts run out first.
    First,
}

// Searching

/// Folds full-width forms, case and hiragana (into katakana) so a query matches
/// however it is typed: the IME gives hiragana, the names are in katakana.
fn fold(text: &str) -> String {
    ComposingNormalizerBorrowed::new_nfkc()
        .normalize(text)
        .to_lowercase()
        .chars()
        // ぁ (U+3041) to ゖ (U+3096) are their katakana less 0x60.
        .map(|c| match c {
            '\u{3041}'..='\u{3096}' => char::from_u32(u32::from(c) + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// A row with a name to search by: an item or equipment, a student, a shop's good.
pub trait Named {
    fn name(&self) -> &str;
}

impl Named for Row {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for StudentRow {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for GoodRow {
    fn name(&self) -> &str {
        &self.name
    }
}

/// The rows whose name holds every word of `query`, however either is typed (see `fold`).
pub fn filter_rows<'a, T: Named>(rows: &'a [T], query: &str) -> Vec<&'a T> {
    let query = fold(query);
    let words: Vec<&str> = query.split_whitespace().collect();
    rows.iter()
        .filter(|row| {
            let name = fold(row.name());
            words.iter().all(|word| name.contains(word))
        })
        .collect()
}

// Tabs

/// A tab or a sub-tab: the id the window keys it by, and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub id: &'static str,
    pub name: &'static str,
}

/// The lists the app reads, in tab order, with their names.
pub const LISTS: [Choice; 6] = [
    Choice {
        id: "items",
        name: "アイテム",
    },
    Choice {
        id: "equipment",
        name: "装備",
    },
    Choice {
        id: "shops",
        name: "ショップ",
    },
    Choice {
        id: "students",
        name: "生徒",
    },
    Choice {
        id: "plan",
        name: "育成",
    },
    Choice {
        id: "work",
        name: "お仕事",
    },
];

/// The item and equipment tabs' sub-tabs by tab: everything held as tiles, then
/// tables by kind (the user's of 2026-10-21).
pub const SUBTABS: [(&str, &[Choice]); 2] = [
    (
        "items",
        &[
            Choice {
                id: "all",
                name: "全体",
            },
            Choice {
                id: "skills",
                name: "スキル素材",
            },
            Choice {
                id: "ooparts",
                name: "オーパーツ",
            },
            Choice {
                id: "currency",
                name: "通貨等",
            },
        ],
    ),
    (
        "equipment",
        &[
            Choice {
                id: "all",
                name: "全体",
            },
            Choice {
                id: "blueprints",
                name: "設計図",
            },
            Choice {
                id: "enhancements",
                name: "強化素材",
            },
        ],
    ),
];

/// The sub-tabs of the tab `tab` (`SUBTABS[tab]`), none for a tab without them.
pub fn subtabs(tab: &str) -> Option<&'static [Choice]> {
    SUBTABS
        .iter()
        .find(|(id, _)| *id == tab)
        .map(|&(_, choices)| choices)
}

/// The four tiers' names, the first to the last.
pub const TIERS: [&str; 4] = ["初級", "中級", "上級", "最上級"];

// The status badge

/// Where the frames read come from, which the status badge's icon shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// A streaming app's stream.
    Cast,
    /// A phone's screen.
    Phone,
    /// The game's window.
    Window,
}

/// The status badge: its label and tone, the icon of where the frames come
/// from (none: a dot), and its hover's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub label: String,
    pub tone: Tone,
    pub source: Option<Capture>,
    pub title: String,
}

/// The name of the list `screen` reads (see `LISTS`).
fn list_name(screen: Screen) -> &'static str {
    let id = match screen {
        Screen::Items => "items",
        Screen::Equipment => "equipment",
        Screen::Students => "students",
        Screen::Shops => "shops",
    };
    LISTS.iter().find(|l| l.id == id).map_or(id, |l| l.name)
}

/// The watcher's status as a short label and a tone for its badge.
fn badge(status: &Status) -> (String, Tone) {
    match status {
        Status::Reading { list, .. } => (format!("{}を集計中", list_name(*list)), Tone::Active),
        Status::OtherScreen => ("対象外の画面".to_owned(), Tone::Idle),
        Status::Failed { .. } => ("キャプチャ失敗".to_owned(), Tone::Danger),
        Status::Paused => ("一時中断".to_owned(), Tone::Paused),
        Status::NoSource => ("取り込み元なし".to_owned(), Tone::Idle),
        Status::TooSmall => ("画面が小さすぎます".to_owned(), Tone::Idle),
        Status::NoGame => ("ゲーム未検出".to_owned(), Tone::Idle),
    }
}

/// How a phone's screen comes in, as the window names it.
fn via_name(via: Via) -> &'static str {
    match via {
        Via::Usb => "USB",
        Via::Wifi => "Wi-Fi",
    }
}

/// Where the frames read come from, and its name: a streaming app's stream or a
/// phone's screen when one comes in, else the game's window (none where nothing
/// can be captured).
fn capture(status: &Status, phone: &Link, cast: &Cast) -> Option<(Capture, String)> {
    if let Cast::Receiving { from, .. } = cast {
        Some((Capture::Cast, format!("配信アプリ（{from}）")))
    } else if let Link::Streaming { device, via } = phone {
        Some((Capture::Phone, format!("{device}（{}）", via_name(*via))))
    } else {
        (!matches!(status, Status::NoSource))
            .then(|| (Capture::Window, "ゲームのウィンドウ".to_owned()))
    }
}

/// The status badge of the watcher's `status`, the phone's link and the streaming app's cast.
pub fn status_view(status: &Status, phone: &Link, cast: &Cast) -> StatusView {
    let (label, tone) = badge(status);
    let source = capture(status, phone, cast);
    let mut title = Vec::new();
    if let Some((_, name)) = &source {
        title.push(format!("取り込み元: {name}"));
    }
    if let Status::Reading { fps, .. } = status {
        title.push(format!("{} fps", fps.round()));
    }
    title.push(
        if matches!(status, Status::Paused) {
            "押すと読み取りを再開"
        } else {
            "押すと読み取りを一時中断"
        }
        .to_owned(),
    );
    StatusView {
        label,
        tone,
        source: source.map(|(kind, _)| kind),
        title: title.join("\n"),
    }
}

// Numbers

/// A whole number as the window writes it, grouped by thousands ("1,234", "-1,100").
pub fn exact(n: impl Into<i128>) -> String {
    let n: i128 = n.into();
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::from(if n < 0 { "-" } else { "" });
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// A number written exactly, "?" where it is unknown.
fn number<T: Into<i128>>(n: Option<T>) -> String {
    n.map_or_else(|| "?".to_owned(), exact)
}

/// `k` in units of the `places`th decimal (hundredths for 2) as the window
/// writes it: grouped, its trailing zeros dropped down to `least` decimals.
fn decimals(k: u64, places: u32, least: usize) -> String {
    let unit = 10_u64.pow(places);
    let mut fraction = format!("{:0width$}", k % unit, width = places as usize);
    while fraction.len() > least && fraction.ends_with('0') {
        fraction.pop();
    }
    let whole = exact(k / unit);
    if fraction.is_empty() {
        whole
    } else {
        format!("{whole}.{fraction}")
    }
}

/// Credits or exp in millions ("28.4M"), to a hundredth below a million,
/// rounded half up.
fn millions(n: u64) -> String {
    if n < 1_000_000 {
        format!("{}M", decimals((n + 5_000) / 10_000, 2, 0))
    } else {
        format!("{}M", decimals((n + 50_000) / 100_000, 1, 0))
    }
}

/// A count of students or pieces to a tenth, cut down, not rounded: a share
/// short of what is needed never shows as reaching it (238 / 60 shows 3.9, not
/// 4.0; the user's call, 2026-10-17).
fn tenths(n: f64) -> String {
    // The 1e-9 keeps an exact tenth that floating point lands just under (0.7 * 3).
    decimals((n * 10.0 + 1e-9).floor() as u64, 1, 1)
}

/// A count in thousands rounded half up to a tenth ("172.7K", the user's of
/// 2026-10-29); below a thousand as it is, a million on in millions.
fn thousands(n: u64) -> String {
    if n < 1000 {
        exact(n)
    } else if n < 1_000_000 {
        format!("{}K", decimals((n + 50) / 100, 1, 1))
    } else {
        millions(n)
    }
}

/// How many students are raised ("5.9人分"), "?" where it is unknown.
fn raised(n: Option<f64>) -> String {
    n.map_or_else(|| "?".to_owned(), |n| format!("{}人分", tenths(n)))
}

/// How many a count holds ("51K" about 51,000; none 0).
fn amount_of(count: Option<Count>) -> u64 {
    match count {
        None => 0,
        Some(Count::Exact(n)) => u64::from(n),
        Some(Count::Thousands(k)) => u64::from(k) * 1000,
    }
}

/// A count as the list shows it: exact below 10,000, in thousands ("51K") from there.
pub fn format_count(count: Count) -> String {
    match count {
        Count::Exact(n) => exact(n),
        Count::Thousands(n) => format!("{n}K"),
    }
}

/// A count held as its tile writes it, "—" where its list was never read.
pub fn held_text(held: Option<Count>) -> String {
    held.map_or_else(|| "—".to_owned(), format_count)
}

// Students

/// A student's training as the students tab's cells (see `student_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentCells {
    pub level: String,
    /// The star counted on into the unique weapon's (see `star_of`), `None` unread.
    pub star: Option<u8>,
    /// The unique weapon's level for the star's hover, empty without one.
    pub weapon: String,
    pub skills: String,
    pub equipment: String,
    pub gear: String,
    pub ability: String,
    pub bond: String,
}

/// A value read, "?" where it could not be.
fn show(v: Option<u8>) -> String {
    v.map_or_else(|| "?".to_owned(), |v| v.to_string())
}

/// A student's training as table cells, written as timeline-arona writes a
/// party: skills "5M7M", equipment "T10/9/8"; "?" unread, "—" what the student lacks.
pub fn student_cells(t: &Training) -> StudentCells {
    let [ex, others @ ..] = t.skills;
    let others: String = others
        .iter()
        .map(|&s| {
            if s == Some(10) {
                "M".to_owned()
            } else {
                show(s)
            }
        })
        .collect();
    let tiers: Vec<String> = t
        .equipment
        .iter()
        .map(|e| show(e.map(|e| e.tier)))
        .collect();
    StudentCells {
        level: show(t.level),
        star: star_of(t),
        weapon: match t.weapon {
            Some(Some(weapon)) => format!("固有武器 Lv{}", weapon.level),
            _ => String::new(),
        },
        skills: show(ex) + &others,
        equipment: format!("T{}", tiers.join("/")),
        gear: match t.gear {
            None => "?".to_owned(),
            Some(0) => "—".to_owned(),
            Some(gear) => format!("T{gear}"),
        },
        ability: t.ability.map(show).join("/"),
        bond: show(t.bond),
    }
}

/// A student's star on hover: the unique weapon's level (`weapon`, see
/// `student_cells`), then how many of the student's elephs are held ("?" never read).
pub fn star_title(weapon: &str, eleph: Option<Count>) -> String {
    let elephs = format!(
        "神名文字: {}",
        eleph.map_or_else(|| "?".to_owned(), format_count)
    );
    if weapon.is_empty() {
        elephs
    } else {
        format!("{weapon}\n{elephs}")
    }
}

/// A column of the students tab to order by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudentColumn {
    #[default]
    Level,
    Star,
    Skills,
    Equipment,
    Gear,
    Ability,
    Bond,
}

/// What `column` orders a training by: numbers compared in turn, `None` unread.
fn student_key(t: &Training, column: StudentColumn) -> Option<(u32, u32)> {
    let one = |v: Option<u8>| v.map(|v| (u32::from(v), 0));
    let sum = |values: &[Option<u8>]| {
        values
            .iter()
            .try_fold(0, |sum, v| Some(sum + u32::from((*v)?)))
            .map(|sum| (sum, 0))
    };
    match column {
        StudentColumn::Level => one(t.level),
        // The star on into the unique weapon's, then the weapon's level.
        StudentColumn::Star => star_of(t).map(|star| {
            let level = match t.weapon {
                Some(Some(weapon)) => weapon.level,
                _ => 0,
            };
            (u32::from(star), u32::from(level))
        }),
        StudentColumn::Skills => sum(&t.skills),
        StudentColumn::Equipment => sum(&t.equipment.map(|e| e.map(|e| e.tier))),
        StudentColumn::Gear => one(t.gear),
        StudentColumn::Ability => sum(&t.ability),
        StudentColumn::Bond => one(t.bond),
    }
}

/// The students ordered by `column`, highest first, what could not be read
/// last; ties, and the default order, go by level, then star, then bond.
pub fn sort_students<'a>(
    rows: impl IntoIterator<Item = &'a StudentRow>,
    column: StudentColumn,
) -> Vec<&'a StudentRow> {
    let order = [
        column,
        StudentColumn::Level,
        StudentColumn::Star,
        StudentColumn::Bond,
    ];
    let mut rows: Vec<&StudentRow> = rows.into_iter().collect();
    // `None` sorts below every `Some`: reversed, it comes last.
    rows.sort_by_key(|r| {
        (
            order.map(|column| std::cmp::Reverse(student_key(&r.training, column))),
            r.id,
        )
    });
    rows
}

// Shops

/// Whether a shop that resets as the month changes (`resets`) was read at
/// `read_at` in a month before `now`'s (both local times as `keep::now` writes them).
pub fn shop_stale(resets: bool, read_at: &str, now: &str) -> bool {
    resets && shop::period(read_at) < shop::period(now)
}

/// The face a good's or an item's tile is painted (`data-face`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// A student's eleph's own.
    Eleph,
    /// Any other good's rarity.
    Rarity(Rarity),
}

/// The face of a good or an item: an eleph its own, any other its rarity, none where that is unknown.
pub fn good_face(eleph: bool, rarity: Option<Rarity>) -> Option<Face> {
    if eleph {
        Some(Face::Eleph)
    } else {
        rarity.map(Face::Rarity)
    }
}

/// A good's tile: how many come with a purchase ("×5", empty without terms),
/// and what each of the shop's coins buys of it (`None` where the coin does not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoodTile {
    pub amount: String,
    pub offers: Vec<Option<OfferText>>,
}

/// What a coin buys of a good: the times left at the price ("12回@10", "@?"
/// without terms), or "売切れ" at none (`sold`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferText {
    pub text: String,
    pub sold: bool,
}

/// A good of a shop as its tile shows it.
pub fn good_tile(good: &GoodRow) -> GoodTile {
    let priced = good.offers.iter().flatten().find_map(|offer| offer.price);
    GoodTile {
        amount: priced.map_or_else(String::new, |p| format!("×{}", exact(p.amount))),
        offers: good
            .offers
            .iter()
            .map(|offer| {
                offer.map(|offer| {
                    let sold = offer.left == 0;
                    let text = if sold {
                        "売切れ".to_owned()
                    } else {
                        format!(
                            "{}回@{}",
                            exact(offer.left),
                            number(offer.price.map(|p| p.price))
                        )
                    };
                    OfferText { text, sold }
                })
            })
            .collect(),
    }
}

/// Each shop with its goods narrowed by `query` (see `filter_rows`); shops left with none go.
pub fn filter_shops(shops: &[ShopRow], query: &str) -> Vec<ShopRow> {
    shops
        .iter()
        .filter_map(|shop| {
            let goods: Vec<GoodRow> = filter_rows(&shop.goods, query)
                .into_iter()
                .cloned()
                .collect();
            (!goods.is_empty()).then(|| ShopRow {
                name: shop.name.clone(),
                read_at: shop.read_at.clone(),
                resets: shop.resets,
                coins: shop.coins.clone(),
                goods,
            })
        })
        .collect()
}

// The plan

/// The name of a line of the plan that counts no item, by kind.
fn plan_kind(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Credits => Some("クレジット"),
        Kind::ReportExp => Some("レポート（経験値）"),
        Kind::OrbExp => Some("強化珠（経験値）"),
        Kind::Item | Kind::Equipment => None,
    }
}

/// The name of a line of the plan: its kind's, else its own.
fn line_name(kind: Kind, name: &str) -> String {
    plan_kind(kind).unwrap_or(name).to_owned()
}

/// An amount of a line of the plan: credits in millions, any other exactly.
fn amount(kind: Kind, n: u64) -> String {
    if kind == Kind::Credits {
        millions(n)
    } else {
        exact(n)
    }
}

/// What is held of a line: the count and its mark ("約" where it is about), `None` unknown.
fn held(have: Option<Held>) -> (Option<u64>, &'static str) {
    match have {
        None => (None, ""),
        Some(Held::Exact(n)) => (Some(n), ""),
        Some(Held::About(n)) => (Some(n), "約"),
    }
}

/// A line of the plan as table cells (see `need_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedCells {
    pub name: String,
    pub need: String,
    pub have: String,
    pub short: String,
    pub enough: bool,
}

/// A line of the plan as table cells: what is needed, what is held ("?"
/// unknown) and how far short it falls; credits in millions.
pub fn need_cells(row: &NeedRow) -> NeedCells {
    let (have, mark) = held(row.have);
    let enough = have.is_some_and(|have| have >= row.need);
    NeedCells {
        name: line_name(row.kind, &row.name),
        need: amount(row.kind, row.need),
        have: have.map_or_else(
            || "?".to_owned(),
            |h| format!("{mark}{}", amount(row.kind, h)),
        ),
        short: match have {
            None => "?".to_owned(),
            Some(_) if enough => "足りる".to_owned(),
            Some(h) => format!("不足 {mark}{}", amount(row.kind, row.need - h)),
        },
        enough,
    }
}

/// A kind of the plan that comes in four tiers: its id (its first tier's) and
/// its tiers in order, `None` where none is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family<'a> {
    pub id: i64,
    pub tiers: [Option<&'a NeedRow>; 4],
}

/// The plan's lines grouped (see `group_plan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanGroups<'a> {
    pub families: Vec<Family<'a>>,
    pub rest: Vec<&'a NeedRow>,
}

/// The plan's lines of the kinds that come in four tiers, one per kind in id
/// order with its tiers in order, and the other lines.
pub fn group_plan(rows: &[NeedRow]) -> PlanGroups<'_> {
    let mut families: BTreeMap<i64, [Option<&NeedRow>; 4]> = BTreeMap::new();
    let mut rest = Vec::new();
    for row in rows {
        let Some((id, tier)) = row.family else {
            rest.push(row);
            continue;
        };
        let tiers = families.entry(id).or_default();
        if let Some(at) = usize::from(tier)
            .checked_sub(1)
            .and_then(|i| tiers.get_mut(i))
        {
            *at = Some(row);
        }
    }
    PlanGroups {
        families: families
            .into_iter()
            .map(|(id, tiers)| Family { id, tiers })
            .collect(),
        rest,
    }
}

/// A tier of a kind as "needed / held", and whether that is enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierCell {
    pub text: String,
    pub enough: bool,
}

/// A tier of a kind (a line of the plan or a tier's blueprints) as "needed / held".
pub fn tier_cell(need: u64, have: Option<Held>) -> TierCell {
    let (have, mark) = held(have);
    let held = have.map_or_else(|| "?".to_owned(), |h| format!("{mark}{}", exact(h)));
    TierCell {
        text: format!("{} / {held}", exact(need)),
        enough: have.is_some_and(|have| have >= need),
    }
}

/// A slot's blueprints as cells (see `blueprint_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintCells {
    pub name: String,
    pub icon: String,
    pub tiers: Vec<BlueprintTier>,
    pub universal: UniversalCell,
}

/// A tier's blueprints of a slot: "needed / held", empty where none is needed;
/// T10 toned by whether it is enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintTier {
    pub key: i64,
    pub label: String,
    pub name: String,
    pub icon: String,
    pub text: String,
    pub tone: Option<Tone>,
}

/// What T2 to T9 take of a slot's universal blueprints over those held, toned as T10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniversalCell {
    pub name: String,
    pub icon: String,
    pub text: String,
    pub tone: Tone,
}

/// A slot's blueprints (see the plan's `BlueprintRow`) as cells: each tier
/// "needed / held", the tiers below T10 paid the rest by the universal ones.
pub fn blueprint_cells(row: &BlueprintRow) -> BlueprintCells {
    let enough = |enough| if enough { Tone::Enough } else { Tone::Short };
    let universal = tier_cell(row.universal.need, Some(row.universal.have));
    BlueprintCells {
        name: row.name.clone(),
        icon: row.icon.clone(),
        tiers: row
            .tiers
            .iter()
            .map(|t| {
                let cell = tier_cell(t.need, Some(t.have));
                BlueprintTier {
                    key: t.id,
                    label: format!("T{}", t.tier),
                    name: t.name.clone(),
                    icon: t.icon.clone(),
                    text: if t.need > 0 { cell.text } else { String::new() },
                    tone: (t.tier == 10 && t.need > 0).then(|| enough(cell.enough)),
                }
            })
            .collect(),
        universal: UniversalCell {
            name: row.universal.name.clone(),
            icon: row.universal.icon.clone(),
            text: universal.text,
            tone: enough(universal.enough),
        },
    }
}

/// A star as a teacher names it: the student's ★1 to ★4, then the unique
/// weapon's 固有1 to 固有4 (★5 is the weapon's first, the user's of 2026-10-13).
pub fn star_name(star: u8) -> String {
    if star <= 4 {
        format!("★{star}")
    } else {
        format!("固有{}", star - 4)
    }
}

/// A star a target may be, and whether it is below the student's now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarChoice {
    pub star: u8,
    pub disabled: bool,
}

/// The stars a target may be, ★1 to 固有4, none below the student's star `now` (`None`: any).
pub fn star_choices(now: Option<u8>) -> [StarChoice; 8] {
    std::array::from_fn(|i| {
        let star = i as u8 + 1;
        StarChoice {
            star,
            disabled: now.is_some_and(|now| star < now),
        }
    })
}

/// Whose stars a star is drawn as: the student's yellow ones or the unique weapon's blue ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarKind {
    Student,
    Weapon,
}

/// A star as the game draws it: whose stars, a place a star, and its name to read it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarMarks {
    pub kind: StarKind,
    pub places: Range<u8>,
    pub label: String,
}

/// A star as the game draws it (the user's of 2026-10-22): ★1 to ★4 as so many
/// of the student's stars, 固有1 to 固有4 as so many of the unique weapon's.
pub fn star_marks(star: u8) -> StarMarks {
    let (kind, count) = if star > 4 {
        (StarKind::Weapon, star - 4)
    } else {
        (StarKind::Student, star)
    };
    StarMarks {
        kind,
        places: 0..count,
        label: star_name(star),
    }
}

/// A part of a target as its column names it.
fn part_name(part: Part) -> &'static str {
    match part {
        Part::Level => "Lv",
        Part::Star => "星",
        Part::Skill(i) => ["EX", "N", "P", "S"].get(i).copied().unwrap_or("?"),
        Part::Equipment(i) => ["装備1", "装備2", "装備3"].get(i).copied().unwrap_or("?"),
    }
}

/// The parts of the targets that could not be counted, one line per student.
pub fn unknown_text(rows: &[UnknownRow]) -> Vec<String> {
    let mut by_student: Vec<(i64, &str, Vec<&str>)> = Vec::new();
    for row in rows {
        let part = part_name(row.part);
        if let Some((.., parts)) = by_student.iter_mut().find(|(id, ..)| *id == row.id) {
            parts.push(part);
        } else {
            by_student.push((row.id, &row.name, vec![part]));
        }
    }
    by_student
        .into_iter()
        .map(|(_, name, parts)| format!("{name}: {}", parts.join("・")))
        .collect()
}

/// A button a menu opens by: its top, bottom and left, in pixels of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
}

/// A width and a height in pixels: the window's, or a menu's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub width: f32,
    pub height: f32,
}

/// Where a menu opens: its left and top in pixels of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuAt {
    pub left: f32,
    pub top: f32,
}

/// Where a menu of the size `menu` opens by its button (`anchor`) in the window
/// (`viewport`): under it, over it where the window ends first, from the
/// window's top where it fits neither; at the button's left, short of the
/// window's right edge; 4 px off the button and the edges.
pub fn menu_place(anchor: Anchor, viewport: Extent, menu: Extent) -> MenuAt {
    const GAP: f32 = 4.0;
    let left = GAP.max(anchor.left.min(viewport.width - GAP - menu.width));
    let below = anchor.bottom + GAP;
    if below + menu.height <= viewport.height {
        return MenuAt { left, top: below };
    }
    let above = anchor.top - GAP - menu.height;
    MenuAt {
        left,
        top: if above >= 0.0 { above } else { GAP },
    }
}

// Procurement

/// The fragments `n` elephs cost from `rate` on: 20 at each rate up to 4, then 5 each.
pub fn fragment_cost(rate: u32, n: u64) -> u64 {
    let (mut cost, mut left, mut at) = (0, n, u64::from(rate.max(1)));
    while left > 0 {
        let take = if at >= 5 { left } else { left.min(20) };
        cost += take * at.min(5);
        left -= take;
        at += 1;
    }
    cost
}

/// A kind of line as the window keys it.
fn kind_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Credits => "credits",
        Kind::ReportExp => "report_exp",
        Kind::OrbExp => "orb_exp",
        Kind::Item => "item",
        Kind::Equipment => "equipment",
    }
}

/// A line of the plan by its kind and id ("item-4033").
fn line_key(line: &ProcureLine) -> String {
    format!("{}-{}", kind_key(line.kind), line.id)
}

/// A source of a line: a box, a shop's coin, a slot's universal blueprints, or the fragments.
fn source_key(source: &Source) -> String {
    match source {
        Source::Box { id, .. } => format!("box:{id}"),
        Source::Shop { shop, coin, .. } => format!("shop:{shop}:{coin}"),
        Source::Universal { id, .. } => format!("universal:{id}"),
        Source::Fragments { .. } => "fragments".to_owned(),
    }
}

/// What a source spends: the coin, which shops share; else the source itself.
fn resource_key(source: &Source) -> String {
    match source {
        Source::Shop { coin, .. } => format!("coin:{coin}"),
        _ => source_key(source),
    }
}

/// A source's icon.
fn source_icon(source: &Source) -> &str {
    match source {
        Source::Box { icon, .. }
        | Source::Shop { icon, .. }
        | Source::Universal { icon, .. }
        | Source::Fragments { icon, .. } => icon,
    }
}

/// What paying `n` from `source` gives its line and costs its resource.
fn paid(source: &Source, n: u64) -> (u64, u64) {
    match source {
        Source::Shop { gives, price, .. } => (n * gives, n * u64::from(*price)),
        Source::Fragments { rate, .. } => (n, fragment_cost(*rate, n)),
        Source::Universal { rate, .. } => (n, n * rate),
        Source::Box { .. } => (n, n),
    }
}

/// How many more a source sells: a shop this month, or the fragments; `None`
/// for one that sells without end.
fn limit(source: &Source) -> Option<u64> {
    match source {
        Source::Shop { left, .. } | Source::Fragments { left, .. } => Some(u64::from(*left)),
        Source::Box { .. } | Source::Universal { .. } => None,
    }
}

/// A press on a chip: with the second button (`right`), with shift.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Click {
    pub right: bool,
    pub shift: bool,
}

/// How many to pay from `source` after a press, `n` paid and `lacking` still
/// short: one more, or with shift as many as make the line up; one fewer with
/// a right press, none with shift. A box may be opened past those held, and
/// universal blueprints spent past them; a shop sells no more than it has left.
pub fn step(source: &Source, n: u64, lacking: u64, click: Click) -> u64 {
    if click.right {
        return if click.shift { 0 } else { n.saturating_sub(1) };
    }
    let each = match source {
        Source::Shop { gives, .. } => (*gives).max(1),
        _ => 1,
    };
    let more = if click.shift {
        lacking.div_ceil(each)
    } else {
        1
    };
    limit(source).map_or(n + more, |left| left.min(n + more))
}

/// The payments settled (see `settle`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    pub lines: Vec<SettledLine>,
    pub resources: Vec<Resource>,
}

/// A line settled: the line, its key ("item-4033"), what is given and what it
/// still lacks, and its sources as chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledLine {
    pub line: ProcureLine,
    pub key: String,
    pub given: u64,
    pub left: u64,
    pub chips: Vec<Chip>,
}

/// A source of a line as a chip: the source, its key, how many are paid, what
/// they give, and what its resource has left after every line's payments
/// (`None` not read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    pub source: Source,
    pub key: String,
    pub n: u64,
    pub gives: u64,
    pub after: Option<i64>,
}

/// A resource paid from (a coin, a box, universal blueprints, the fragments):
/// held (`None` not read), spent and left after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub held: Option<u64>,
    pub spent: u64,
    pub after: Option<i64>,
}

/// The resource `source` spends, none of it spent yet.
fn resource_of(source: &Source) -> Resource {
    let (name, held) = match source {
        Source::Shop { coin, held, .. } => (coin, *held),
        Source::Box { name, held, .. }
        | Source::Universal { name, held, .. }
        | Source::Fragments { name, held, .. } => (name, Some(*held)),
    };
    Resource {
        key: resource_key(source),
        name: name.clone(),
        icon: source_icon(source).to_owned(),
        held,
        spent: 0,
        after: None,
    }
}

/// The `payments` (by "line|source" key, see `SettledLine::key` and
/// `Chip::key`) settled: each line with what is given and what it still lacks,
/// its sources as chips, and each resource held, spent and left after.
pub fn settle(lines: &[ProcureLine], payments: &BTreeMap<String, u64>) -> Settled {
    let paid_from = |line: &ProcureLine, source: &Source| {
        let key = format!("{}|{}", line_key(line), source_key(source));
        let n = payments.get(&key).copied().unwrap_or(0);
        (n, paid(source, n))
    };
    let mut resources: Vec<Resource> = Vec::new();
    for line in lines {
        for source in &line.sources {
            let (_, (_, cost)) = paid_from(line, source);
            let key = resource_key(source);
            if let Some(resource) = resources.iter_mut().find(|r| r.key == key) {
                resource.spent += cost;
            } else {
                resources.push(Resource {
                    spent: cost,
                    ..resource_of(source)
                });
            }
        }
    }
    for resource in &mut resources {
        resource.after = resource
            .held
            .map(|held| held as i64 - resource.spent as i64);
    }
    let after = |source: &Source| {
        let key = resource_key(source);
        resources
            .iter()
            .find(|r| r.key == key)
            .and_then(|r| r.after)
    };
    let lines = lines
        .iter()
        .map(|line| {
            let chips: Vec<Chip> = line
                .sources
                .iter()
                .map(|source| {
                    let (n, (gives, _)) = paid_from(line, source);
                    Chip {
                        source: source.clone(),
                        key: source_key(source),
                        n,
                        gives,
                        after: after(source),
                    }
                })
                .collect();
            let given: u64 = chips.iter().map(|chip| chip.gives).sum();
            SettledLine {
                line: line.clone(),
                key: line_key(line),
                given,
                left: line.short.saturating_sub(given),
                chips,
            }
        })
        .collect();
    Settled { lines, resources }
}

/// What a source is, on hover: the box, the shop's terms, or the fragments' rate.
fn source_title(source: &Source) -> String {
    match source {
        Source::Shop {
            shop,
            coin,
            price,
            gives,
            left,
            ..
        } => format!(
            "{shop}: {coin} {} で ×{}、今月あと {} 回",
            exact(*price),
            exact(*gives),
            exact(*left)
        ),
        Source::Fragments {
            name, rate, left, ..
        } => format!(
            "{name}: 1 文字 {rate} カケラから、あと {} 文字",
            exact(*left)
        ),
        Source::Universal {
            name, rate, held, ..
        } => format!(
            "{name}: T10設計図 1 枚に {rate} 枚（T2〜T9 に使った残り {} 枚）",
            exact(*held)
        ),
        Source::Box { name, choices, .. } => format!("{name}（{choices} 種から 1 つ）"),
    }
}

/// The settled procurement as text (see `procure_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcureCells {
    pub lines: Vec<LineCells>,
    pub resources: Vec<ResourceCells>,
}

/// A line of the procurement: its shortfall and what it still lacks (credits
/// in millions), whether it is made up, and its chips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCells {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub short: String,
    pub left: String,
    pub done: bool,
    pub chips: Vec<ChipCells>,
}

/// A chip: the resource left after ("?" not read, "売切れ" once a shop sells no
/// more, `out`), whether that is below none, what it gives ("+20") and its hover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipCells {
    pub key: String,
    pub icon: String,
    pub text: String,
    pub negative: bool,
    pub given: String,
    pub out: bool,
    pub title: String,
}

/// A resource paid from: held, spent ("-30") and left after, toned above or below none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCells {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub held: String,
    pub spent: String,
    pub after: String,
    pub tone: Option<Tone>,
}

/// The settled procurement (see `settle`) as text: each line and its chips,
/// and each resource paid from (those untouched left out).
pub fn procure_cells(settled: &Settled) -> ProcureCells {
    let chip = |kind: Kind, c: &Chip| {
        // A shop's purchases this month, or the fragments' exchanges, all paid.
        let out = limit(&c.source).is_some_and(|left| c.n >= left);
        ChipCells {
            key: c.key.clone(),
            icon: source_icon(&c.source).to_owned(),
            text: if out {
                "売切れ".to_owned()
            } else {
                number(c.after)
            },
            negative: !out && c.after.is_some_and(|after| after < 0),
            given: if c.n > 0 {
                format!("+{}", amount(kind, c.gives))
            } else {
                String::new()
            },
            out,
            title: source_title(&c.source),
        }
    };
    ProcureCells {
        lines: settled
            .lines
            .iter()
            .map(|l| LineCells {
                key: l.key.clone(),
                name: line_name(l.line.kind, &l.line.name),
                icon: l.line.icon.clone(),
                short: amount(l.line.kind, l.line.short),
                left: amount(l.line.kind, l.left),
                done: l.left == 0,
                chips: l.chips.iter().map(|c| chip(l.line.kind, c)).collect(),
            })
            .collect(),
        resources: settled
            .resources
            .iter()
            .filter(|r| r.spent > 0)
            .map(|r| ResourceCells {
                key: r.key.clone(),
                name: r.name.clone(),
                icon: r.icon.clone(),
                held: number(r.held),
                spent: format!("-{}", exact(r.spent)),
                after: number(r.after),
                tone: match r.after.map(i64::signum) {
                    Some(1) => Some(Tone::Plus),
                    Some(-1) => Some(Tone::Minus),
                    _ => None,
                },
            })
            .collect(),
    }
}

// The work tab

/// The work tab's folds of the screens told by their title (`title::read`).
const WORK_SCREENS: [(&str, &str); 5] = [
    ("学園交流会", "work:学園交流会"),
    ("指名手配", "work:指名手配"),
    ("任務 Normal", "work:任務"),
    ("特別依頼", "work:特別依頼"),
    ("ショップ: 通常アイテム", "work:通常アイテム"),
];

/// The work tab's fold for where the game is: a screen told by its title, or a
/// month-end shop among `month_end`; `None` where the tab has none.
pub fn work_fold(place: Option<Place>, month_end: &[PlanShop]) -> Option<String> {
    match place? {
        Place::Screen(name) => WORK_SCREENS
            .iter()
            .find(|&&(screen, _)| screen == name)
            .map(|&(_, fold)| fold.to_owned()),
        Place::Shop(name) => month_end
            .iter()
            .any(|shop| shop.name == name)
            .then(|| format!("work:月末:{name}")),
    }
}

/// The month, day, hour and minute of a local time ("2026-11-03 14:37:12" is "11/03 14:37").
pub fn short_time(at: &str) -> String {
    at.chars()
        .skip(5)
        .take(11)
        .collect::<String>()
        .replacen('-', "/", 1)
}

/// What the 育成 tab tells of the plan's being behind: when the reads it counts
/// from were, what was read otherwise since, and that 再読込 counts again; nothing while it counts alike.
pub fn behind_text(behind: Option<&Behind>) -> String {
    let Some(behind) = behind else {
        return String::new();
    };
    let since = behind
        .since
        .as_deref()
        .filter(|since| !since.is_empty())
        .map_or_else(String::new, |since| {
            format!("{} の読み取りで計算中。", short_time(since))
        });
    let what: Vec<&str> = behind
        .students
        .iter()
        .chain(&behind.shops)
        .map(String::as_str)
        .chain(behind.held.then_some("所持数"))
        .collect();
    format!(
        "{since}{}が変わっています。再読込で数え直します（払った分は取り消されます）",
        what.join("・")
    )
}

/// What a slot's blueprints make (see `stock_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockCells {
    pub pieces: String,
    pub held: String,
    pub tone: Option<Tone>,
}

/// What a slot's blueprints make: how many pieces from T1 to T10, to a tenth
/// ("4.5個分"), what is held against what one piece takes, and how short it
/// falls: of one piece `Short`, of the slot's `golden` ratio `Low`.
pub fn stock_cells(stock: &Stock, golden: u32) -> StockCells {
    let about = if stock.about { "約" } else { "" };
    StockCells {
        pieces: format!("{}個分", tenths(stock.pieces)),
        held: format!("{about}{} / {}", exact(stock.held), exact(stock.need)),
        tone: if stock.pieces < 1.0 {
            Some(Tone::Short)
        } else if stock.pieces < f64::from(golden) {
            Some(Tone::Low)
        } else {
            None
        },
    }
}

/// A slot's golden ratio in its hover: how many students, and why: the slot's
/// share of every student's slots in `total` students, cut to two decimals.
pub fn golden_title(golden: &Golden) -> String {
    let Golden {
        students,
        slots,
        all,
        total,
    } = *golden;
    let share = (slots * total * 100).checked_div(all).unwrap_or(0);
    [
        format!("黄金比 {students}人"),
        format!("全実装生徒の装備欄 {} のうち {}", exact(all), exact(slots)),
        format!(
            "{} ÷ {} × {total} = {}.{:02}（0.25以上は切り上げ）",
            exact(slots),
            exact(all),
            share / 100,
            share % 100
        ),
    ]
    .join("\n")
}

/// The skill materials' choice boxes of every set summed for a tier: the first
/// set's box with every set's held, and what each set holds ("429 + 5 + 0").
#[derive(Debug, Clone, PartialEq)]
pub struct BoxTotal {
    pub cell: HeldCell,
    pub parts: String,
}

/// The BD row and the tech note row of the summed choice boxes (see `box_totals`).
#[derive(Debug, Clone, PartialEq)]
pub struct BoxTotals {
    pub bd: [Option<BoxTotal>; 4],
    pub notes: [Option<BoxTotal>; 4],
}

/// The skill materials' choice boxes of every set as one (the user's of
/// 2026-10-31): a set without the tier's box, or holding none, counts 0; none
/// held anywhere stays unknown; a tier no set has stays empty; none without sets.
pub fn box_totals(sets: &[SkillBoxes]) -> Option<BoxTotals> {
    if sets.is_empty() {
        return None;
    }
    let row = |pick: fn(&SkillBoxes) -> &[Option<HeldCell>; 4]| -> [Option<BoxTotal>; 4] {
        std::array::from_fn(|tier| {
            let cells: Vec<Option<&HeldCell>> =
                sets.iter().map(|set| pick(set)[tier].as_ref()).collect();
            let first = cells.iter().flatten().next()?;
            let counts: Vec<Option<Count>> = cells.iter().map(|c| c.and_then(|c| c.held)).collect();
            let total: u64 = counts.iter().map(|&count| amount_of(count)).sum();
            let held = counts.iter().any(Option::is_some).then(|| {
                if total < 10_000 {
                    Count::Exact(total as u32)
                } else {
                    Count::Thousands(((total + 500) / 1000) as u32)
                }
            });
            let parts: Vec<String> = counts
                .iter()
                .map(|count| count.map_or_else(|| "0".to_owned(), format_count))
                .collect();
            Some(BoxTotal {
                cell: HeldCell {
                    held,
                    ..HeldCell::clone(first)
                },
                parts: parts.join(" + "),
            })
        })
    };
    Some(BoxTotals {
        bd: row(|set| &set.bd),
        notes: row(|set| &set.notes),
    })
}

/// How a drop held stands against what one student needs: short of one
/// `Short`, of two `Fair`, two or more `Enough`; none where nothing is needed.
pub fn held_tone(held: Option<Count>, need: Option<u64>) -> Option<Tone> {
    let need = need?;
    let n = amount_of(held);
    Some(if n < need {
        Tone::Short
    } else if n < 2 * need {
        Tone::Fair
    } else {
        Tone::Enough
    })
}

/// The farm to the golden ratio as text (see `farm_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmCells {
    pub lines: Vec<FarmLine>,
    pub total: FarmTotal,
    pub next: Vec<NextLine>,
}

/// A mission of the farm: its runs, AP and the slots it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmLine {
    pub name: String,
    pub runs: String,
    pub ap: String,
    pub slots: String,
}

/// The farm's runs and AP in all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmTotal {
    pub runs: String,
    pub ap: String,
}

/// A mission past the golden ratio, with the slots it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextLine {
    pub name: String,
    pub slots: String,
}

/// The farm to the golden ratio: each mission's runs, AP and the slots it
/// serves, and the totals; and the missions past the ratio with their slots.
pub fn farm_cells(farm: &Farm) -> FarmCells {
    FarmCells {
        lines: farm
            .stages
            .iter()
            .map(|s| FarmLine {
                name: s.name.clone(),
                runs: format!("{}回", exact(s.runs)),
                ap: format!("{}AP", exact(s.ap)),
                slots: s.slots.join("・"),
            })
            .collect(),
        total: FarmTotal {
            runs: format!("{}回", exact(farm.runs)),
            ap: format!("{}AP", exact(farm.ap)),
        },
        next: farm
            .next
            .iter()
            .map(|s| NextLine {
                name: s.name.clone(),
                slots: s.slots.join("・"),
            })
            .collect(),
    }
}

/// A daily play's place without the kind of play it is ("指名手配：ハイウェイ" is "ハイウェイ").
pub fn place_name(name: &str) -> &str {
    let tail = name.rsplit('：').next().unwrap_or(name);
    tail.rsplit(" - ").next().unwrap_or(tail)
}

/// How many students what is held raises ("6.6人分"), and how, in two lines:
/// the exp held times its rate ("約" where it is about), and over what one takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseCells {
    pub students: String,
    pub numerator: String,
    pub denominator: String,
}

/// How a raise comes (the user's of 2026-10-29: 「分子」「/ 分母」): the exp held
/// times the rate it is taken at, and over what one student takes, written by `format`.
fn raise_terms(
    exp: u64,
    about: bool,
    rate: f64,
    each: u64,
    format: fn(u64) -> String,
) -> (String, String) {
    let about = if about { "約" } else { "" };
    let exp = (exp as f64 * rate).round() as u64;
    (
        format!("{about}{} EXP", format(exp)),
        format!("/ {} EXP", format(each)),
    )
}

/// A kind of enhancement material in the equipment tab: how many students what
/// is held raises, and how, the orbs in millions, the parts in thousands.
pub fn enhancement_cells(kind: &Enhancement) -> RaiseCells {
    let r = &kind.raise;
    let format = if kind.category == "Exp" {
        millions
    } else {
        thousands
    };
    let (numerator, denominator) = raise_terms(r.exp, r.about, r.rate, r.exp_each, format);
    RaiseCells {
        students: raised(Some(r.students)),
        numerator,
        denominator,
    }
}

/// What a row of the work tab's counts holds: a kind's tiers, or a count of
/// credits ("—" never read) beside their icon.
#[derive(Debug, Clone, PartialEq)]
pub enum RaiseHeld {
    Tiers(Vec<Option<HeldCell>>),
    Credits(String),
}

/// A row of the work tab's counts, drawn as the enhancement materials' (the
/// user's of 2026-10-29): a title over what is held, with the students it
/// raises and how at the right; toned against the others, and why on hover.
#[derive(Debug, Clone, PartialEq)]
pub struct RaiseRow {
    pub key: String,
    pub title: String,
    pub held: RaiseHeld,
    pub students: String,
    pub numerator: String,
    pub denominator: String,
    pub tone: Option<Tone>,
    pub why: Option<String>,
}

/// The credits held against a student raised from Lv1 as a row, under `title`.
fn credit_row(c: &CommissionRow, title: &str) -> RaiseRow {
    RaiseRow {
        key: "credits".to_owned(),
        title: title.to_owned(),
        held: RaiseHeld::Credits(c.credits.map_or_else(|| "—".to_owned(), exact)),
        students: raised(c.credit_students),
        numerator: c.credits.map_or_else(|| "?".to_owned(), millions),
        denominator: format!("/ {}", millions(c.credits_each)),
        tone: None,
        why: None,
    }
}

/// `rows` toned by the students each raises (`counts`): `Fewer` the one to
/// play, `More` the others; none when one is unknown or two tie.
fn toned<const N: usize>(mut rows: [RaiseRow; N], counts: [Option<f64>; N]) -> [RaiseRow; N] {
    let Some(counts) = counts.into_iter().collect::<Option<Vec<f64>>>() else {
        return rows;
    };
    let tie = counts
        .iter()
        .enumerate()
        .any(|(i, a)| counts[..i].iter().any(|b| a.total_cmp(b).is_eq()));
    if tie {
        return rows;
    }
    let fewest = counts.iter().copied().fold(f64::INFINITY, f64::min);
    for (row, n) in rows.iter_mut().zip(&counts) {
        row.tone = Some(if n.total_cmp(&fewest).is_eq() {
            Tone::Fewer
        } else {
            Tone::More
        });
    }
    rows
}

/// The commissions as their rows, the reports first as the game lists 拠点防衛
/// (the user's of 2026-10-16): the reports by tier, then the credits.
pub fn commission_rows(c: &CommissionRow) -> [RaiseRow; 2] {
    let (numerator, denominator) =
        raise_terms(c.report_exp, c.report_about, 1.0, c.exp_each, millions);
    let reports = RaiseRow {
        key: "reports".to_owned(),
        title: c.reports_name.clone(),
        held: RaiseHeld::Tiers(c.report_tiers.to_vec()),
        students: raised(Some(c.report_students)),
        numerator,
        denominator,
        tone: None,
        why: None,
    };
    toned(
        [reports, credit_row(c, &c.credits_name)],
        [Some(c.report_students), c.credit_students],
    )
}

/// The credits and the reports for the item tab's 通貨等, as the commissions count them: the credits first.
pub fn currency_rows(c: &CommissionRow) -> [RaiseRow; 2] {
    let [reports, credits] = commission_rows(c);
    [
        RaiseRow {
            title: "クレジット".to_owned(),
            ..credits
        },
        RaiseRow {
            title: "レポート".to_owned(),
            ..reports
        },
    ]
}

/// A coin of the item tab's 通貨等: its name, icon and how many are held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoinCell {
    pub name: String,
    pub icon: String,
    pub held: Option<Count>,
}

/// The coins for the item tab's 通貨等: those the item list holds (`coins`),
/// then each coin of the shops read the item list lacks (熟達証書), as its shop last read it.
pub fn currency_coins(coins: &[HeldCell], shops: &[ShopRow]) -> Vec<CoinCell> {
    let mut all: Vec<CoinCell> = coins
        .iter()
        .map(|c| CoinCell {
            name: c.name.clone(),
            icon: c.icon.clone(),
            held: c.held,
        })
        .collect();
    for coin in shops.iter().flat_map(|s| &s.coins) {
        if !all.iter().any(|c| c.name == coin.name) {
            all.push(CoinCell {
                name: coin.name.clone(),
                icon: coin.icon.clone(),
                held: coin.held,
            });
        }
    }
    all
}

/// The normal items shop's daily buys as their rows: the credits, then the
/// enhancement orbs as the equipment tab's row of them (`orbs`, category Exp);
/// the credits alone before the equipment is counted by kind.
pub fn normal_item_rows(c: &CommissionRow, orbs: Option<&Enhancement>) -> Vec<RaiseRow> {
    let credits = credit_row(c, "クレジット");
    let Some(orbs) = orbs else {
        return vec![credits];
    };
    let cells = enhancement_cells(orbs);
    let row = RaiseRow {
        key: "orbs".to_owned(),
        title: "強化珠".to_owned(),
        held: RaiseHeld::Tiers(orbs.tiers.to_vec()),
        students: cells.students,
        numerator: cells.numerator,
        denominator: cells.denominator,
        tone: None,
        why: None,
    };
    toned(
        [credits, row],
        [c.credit_students, Some(orbs.raise.students)],
    )
    .into()
}

/// Each school's scrimmage as its row: the school's name over the parts it
/// drops (`places`), with the weapons they raise (`schools`, see
/// `school_cells`); the school to play first toned `First`, why on hover.
pub fn scrimmage_rows(places: &[PlaceRow], schools: &[SchoolRow]) -> Vec<RaiseRow> {
    places
        .iter()
        .zip(schools)
        .map(|(place, school)| {
            let cells = school_cells(school);
            RaiseRow {
                key: place.name.clone(),
                title: place_name(&place.name).to_owned(),
                held: RaiseHeld::Tiers(place.kinds.first().map_or_else(Vec::new, |k| k.to_vec())),
                students: cells.students,
                numerator: cells.numerator,
                denominator: cells.denominator,
                tone: cells.first.then_some(Tone::First),
                why: Some(cells.why),
            }
        })
        .collect()
}

/// A school's scrimmage parts held (see `school_cells`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolCells {
    pub students: String,
    pub numerator: String,
    pub denominator: String,
    pub first: bool,
    pub why: String,
}

/// A school's scrimmage parts held: how many unique weapons they raise from Lv1
/// to their cap, and how; whether it runs out first, and why, for the hover.
pub fn school_cells(row: &SchoolRow) -> SchoolCells {
    let students = raised(Some(row.students));
    let share = format!("{}%", (row.share * 100.0).round());
    let mut why = vec![format!("素材が合う生徒: {share}")];
    if row.share > 0.0 {
        why.push(format!(
            "{students} ÷ {share} = {}",
            tenths(row.students / row.share)
        ));
    }
    if row.first {
        why.push("最も少ないので先に回します".to_owned());
    }
    let (numerator, denominator) =
        raise_terms(row.exp, row.about, row.rate, row.exp_each, thousands);
    SchoolCells {
        students,
        numerator,
        denominator,
        first: row.first,
        why: why.join("\n"),
    }
}

// Month-end shops and the hard missions

/// A month-end shop's card (see `plan_card`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCardCells {
    pub amount: String,
    pub text: String,
    /// The star 固有4 (see `star_marks`) where the card's student takes no more.
    pub star: Option<u8>,
}

/// A month-end shop's card: how many one buy gives ("×5"), and the times left at
/// the price ("16回@50", "@?" without terms) or "売切れ"; where its student takes no more, 固有4 instead.
pub fn plan_card(card: &PlanCard) -> PlanCardCells {
    PlanCardCells {
        amount: card
            .amount
            .filter(|&amount| amount > 0)
            .map_or_else(String::new, |amount| format!("×{}", exact(amount))),
        text: if card.barred {
            String::new()
        } else if card.left == 0 {
            "売切れ".to_owned()
        } else {
            format!("{}回@{}", exact(card.left), number(card.price))
        },
        star: card.barred.then_some(8),
    }
}

/// A month-end card's hover: its good's name, an eleph's student's star, and how many are held.
pub fn plan_title(card: &PlanCard, students: &[StudentRow]) -> String {
    format!(
        "{}\n所持: {}",
        tile_title(card.id, &card.name, card.eleph, students),
        card.held.map_or_else(|| "?".to_owned(), format_count)
    )
}

/// A good's or an item's hover: its name and, for an eleph (its id its
/// student's), the student's star ("未読み取り" for a student never read).
pub fn tile_title(id: i64, name: &str, eleph: bool, students: &[StudentRow]) -> String {
    if !eleph {
        return name.to_owned();
    }
    let star = students.iter().find(|s| s.id == id).map_or_else(
        || "未読み取り".to_owned(),
        |student| star_of(&student.training).map_or_else(|| "?".to_owned(), star_name),
    );
    format!("{name}\n星: {star}")
}

/// A hard missions' card (see `hard_tile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardTile {
    pub amount: String,
    pub star: Option<u8>,
    pub held: String,
    pub title: String,
}

/// A hard missions' card (the user's of 2026-10-22): how many missions drop the
/// student's eleph ("3個所"), the student's star and the elephs held ("148文字");
/// and its hover: the eleph, the star, the count held, the missions and a day's AP.
pub fn hard_tile(card: &HardCard) -> HardTile {
    let held = card.held.map_or_else(|| "?".to_owned(), format_count);
    let star = card.star.map_or_else(|| "未読み取り".to_owned(), star_name);
    HardTile {
        amount: format!("{}個所", card.stages.len()),
        star: card.star,
        held: format!("{held}文字"),
        title: [
            card.name.clone(),
            format!("星: {star}"),
            format!("所持: {held}"),
            format!("任務 Hard: {}", card.stages.join("、")),
            format!("1日 {} AP", exact(card.ap)),
        ]
        .join("\n"),
    }
}

/// A day's AP the hard missions' cards reserved take, those at 固有4 too.
pub fn hard_cost(cards: &[HardCard]) -> u32 {
    cards.iter().filter(|c| c.reserved).map(|c| c.ap).sum()
}

/// A month-end shop's coin (see `plan_coin`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCoinCells {
    pub left: String,
    pub reserved: bool,
    pub short: bool,
    pub title: String,
}

/// A month-end shop's coin: how many are left once the cards reserved are paid
/// ("?" not read), whether any is reserved (the user's of 2026-10-21), and how that comes.
pub fn plan_coin(coin: &PlanCoin) -> PlanCoinCells {
    let left = coin
        .held
        .map(|held| i128::from(amount_of(Some(held))) - i128::from(coin.need));
    PlanCoinCells {
        left: number(left),
        reserved: coin.need > 0,
        short: coin.short,
        title: format!(
            "{}: 所持 {} − 予約 {}",
            coin.name,
            coin.held.map_or_else(|| "?".to_owned(), format_count),
            exact(coin.need)
        ),
    }
}

// Updates, the phone and the streaming app

/// Where checking for an update and taking it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Checking,
    Latest { current: String },
    Available { current: String, latest: String },
    Installing { latest: String },
    Failed { error: String },
}

/// Where checking for an update and taking it stands, as the window tells it.
pub fn update_text(update: &UpdateState) -> String {
    match update {
        UpdateState::Checking => "アップデートを確認しています…".to_owned(),
        UpdateState::Latest { current } => format!("最新のバージョンです（{current}）"),
        UpdateState::Available { current, latest } => {
            format!("{latest} があります（今 {current}）")
        }
        UpdateState::Installing { latest } => format!("{latest} に更新しています…"),
        UpdateState::Failed { error } => format!("アップデートできませんでした: {error}"),
    }
}

/// Where the link to a phone stands, as one line.
pub fn phone_line(link: &Link) -> String {
    match link {
        Link::Pairing { .. } => {
            "この QR コードを読み取るか、USB ケーブルでつないでください".to_owned()
        }
        Link::Searching { device } => format!("{device} を探しています…"),
        Link::Allowing => {
            "スマホに出た「USB デバッグを許可しますか？」で許可を押してください".to_owned()
        }
        Link::Streaming { device, via } => {
            format!("{device} の画面を {} で受け取っています", via_name(*via))
        }
        Link::Stopped { device } => format!("{device} とは切断しています"),
        Link::Failed { message } => message.clone(),
        Link::Unpaired => "スマホとはまだつないでいません".to_owned(),
    }
}

/// The sentence of where waiting for a streaming app stands.
pub fn cast_line(cast: &Cast) -> String {
    match cast {
        Cast::Waiting { .. } => {
            "配信アプリでこの QR コードを読み取り、配信を始めてください".to_owned()
        }
        Cast::Receiving { from, .. } => format!("{from} から画面を受け取っています"),
        Cast::Failed { message } => message.clone(),
        Cast::Off => {
            "待ち受けると、スマホの配信アプリ（iPhone は Moblin）から画面を受け取れます".to_owned()
        }
    }
}

/// What went wrong with the last stream, or how to send the screen larger; `None` when nothing.
pub fn cast_note(cast: &Cast) -> Option<String> {
    match cast {
        Cast::Waiting { problem, .. } => problem.clone(),
        Cast::Receiving { sideways: true, .. } => Some(
            "画面が横倒しで小さく届いています。配信アプリで縦向き（Portrait）の配信にすると大きく届きます"
                .to_owned(),
        ),
        _ => None,
    }
}

/// Modules of light margin a QR code needs around it to be read.
pub const QR_QUIET: usize = 4;

/// Where a QR code's dark `modules` (rows of booleans) are drawn: a unit square
/// each at `(x, y)` inside its quiet zone, the code `len + 2 * QR_QUIET` units a side.
pub fn qr_squares(modules: &[Vec<bool>]) -> Vec<(usize, usize)> {
    modules
        .iter()
        .enumerate()
        .flat_map(|(y, row)| {
            row.iter()
                .enumerate()
                .filter(|&(_, &dark)| dark)
                .map(move |(x, _)| (x + QR_QUIET, y + QR_QUIET))
        })
        .collect()
}

#[cfg(test)]
mod tests;

/// A line of the plan that falls short, with the bounties that drop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BountyCandidate {
    pub id: i64,
    pub name: String,
    pub icon: String,
    /// How many are short ("?" for a line never read).
    pub short: String,
    /// The bounties' places that drop it, as their names read without "指名手配：".
    pub places: Vec<String>,
}

/// The plan's item lines short of what they need that a bounty drops, each with those bounties'
/// places (the user's of 2026-10-13: 「毎日プレイ出来る「指名手配」の項目で学園の技術ノート・BDや
/// オーパーツで参加すべきステージの候補を表示する」). A line never read is kept, its shortfall "?".
pub fn bounty_candidates(plan: &[NeedRow], bounties: &[PlaceRow]) -> Vec<BountyCandidate> {
    plan.iter()
        .filter(|line| line.kind == crate::plan::Kind::Item)
        .filter_map(|line| {
            // A line never read may fall short: how short is not known.
            let short = match line.have {
                Some(held) => exact(line.need.checked_sub(held.amount()).filter(|&n| n > 0)?),
                None => "?".to_owned(),
            };
            let places: Vec<String> = bounties
                .iter()
                .filter(|place| {
                    place
                        .kinds
                        .iter()
                        .flatten()
                        .flatten()
                        .any(|cell| cell.id == line.id)
                })
                .map(|place| place_name(&place.name).to_owned())
                .collect();
            (!places.is_empty()).then(|| BountyCandidate {
                id: line.id,
                name: line.name.clone(),
                icon: line.icon.clone(),
                short,
                places,
            })
        })
        .collect()
}
