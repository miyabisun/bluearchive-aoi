/**
 * Folds full-width forms, case and hiragana (into katakana) so a query matches
 * however it is typed: the IME gives hiragana, the names are in katakana.
 */
const fold = (text) =>
  text
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[ぁ-ゖ]/g, (c) => String.fromCharCode(c.charCodeAt(0) + 0x60));

/** The rows whose name holds every word of `query`. */
export function filterRows(rows, query) {
  const words = fold(query).split(/\s+/).filter(Boolean);
  return rows.filter((row) => {
    const name = fold(row.name);
    return words.every((word) => name.includes(word));
  });
}

/** The lists the app reads, in tab order, with their names. */
export const LISTS = [
  { id: "items", name: "アイテム" },
  { id: "equipment", name: "装備" },
  { id: "shops", name: "ショップ" },
  { id: "students", name: "生徒" },
  { id: "plan", name: "育成" },
  { id: "work", name: "お仕事" },
];

const listName = (id) => LISTS.find((l) => l.id === id)?.name ?? id;

/** The watcher's status as a short label and a tone for its badge. */
export function statusView(status) {
  switch (status.state) {
    case "reading":
      return {
        label: `${listName(status.list)}を集計中 ${Math.round(status.fps)} fps`,
        tone: "active",
      };
    case "other_screen":
      return { label: "対象外の画面", tone: "idle" };
    case "failed":
      return { label: "キャプチャ失敗", tone: "danger" };
    default:
      return { label: "ゲーム未検出", tone: "idle" };
  }
}

const exact = new Intl.NumberFormat("ja-JP");

/** A count as the list shows it: exact below 10,000, in thousands ("51K") from there. */
export function formatCount(count) {
  return "thousands" in count ? `${count.thousands}K` : exact.format(count.exact);
}

/**
 * The tab to show after a new status: when the game starts showing another list,
 * its tab; otherwise the current one, so a tab chosen by hand stays.
 */
export function followTab(view, status) {
  if (status.state !== "reading" || status.list === view.lastRead) return view;
  return { tab: status.list, lastRead: status.list };
}

const known = (v) => v !== null && v !== undefined;
const show = (v) => (known(v) ? String(v) : "?");

/**
 * A student's training as table cells, written as timeline-arona writes a
 * party: skills "5M7M" (the EX level, then M for a skill at 10), equipment
 * "T10/9/8". "?" marks what could not be read; "—" what the student lacks.
 * The unique weapon is `null` while locked, absent when unknown.
 */
export function studentCells(t) {
  const [ex, ...others] = t.skills;
  const weapon = !("weapon" in t) || t.weapon === undefined ? "?" : t.weapon;
  return {
    level: show(t.level),
    rarity: known(t.rarity) ? `★${t.rarity}` : "?",
    weapon: weapon === "?" ? "?" : weapon === null ? "—" : `★${weapon.stars} Lv${weapon.level}`,
    skills: show(ex) + others.map((s) => (s === 10 ? "M" : show(s))).join(""),
    equipment: "T" + t.equipment.map((e) => (known(e) ? String(e.tier) : "?")).join("/"),
    equipmentLevels: "Lv" + t.equipment.map((e) => (known(e) ? String(e.level) : "?")).join("/"),
    gear: !known(t.gear) ? "?" : t.gear === 0 ? "—" : `T${t.gear}`,
    ability: t.ability.map(show).join("/"),
    bond: show(t.bond),
  };
}

const sum = (values) => (values.every(known) ? values.reduce((a, b) => a + b, 0) : null);

/**
 * What each column of the student table orders by: a number, or numbers
 * compared in turn; `null` for what could not be read.
 */
const STUDENT_KEYS = {
  level: (t) => t.level,
  rarity: (t) => t.rarity,
  // A locked weapon comes after every unlocked one, and before an unread one.
  weapon: (t) =>
    t.weapon === undefined ? null : t.weapon === null ? [0, 0] : [t.weapon.stars, t.weapon.level],
  skills: (t) => sum(t.skills),
  equipment: (t) =>
    t.equipment.every(known)
      ? [sum(t.equipment.map((e) => e.tier)), sum(t.equipment.map((e) => e.level))]
      : null,
  gear: (t) => t.gear,
  ability: (t) => sum(t.ability),
  bond: (t) => t.bond,
};

/** Highest first, what could not be read last. */
function descending(a, b) {
  if (!known(a) || !known(b)) return Number(!known(a)) - Number(!known(b));
  const [x, y] = [[a].flat(), [b].flat()];
  const at = x.findIndex((v, i) => v !== y[i]);
  return at < 0 ? 0 : y[at] - x[at];
}

/**
 * The students ordered by `column` (a key of the training), highest first;
 * ties, and the default order, go by level, then rarity, then bond.
 */
export function sortStudents(rows, column = "level") {
  const order = [column, "level", "rarity", "bond"];
  return [...rows].sort(
    (a, b) =>
      order.reduce(
        (found, key) =>
          found || descending(STUDENT_KEYS[key](a.training), STUDENT_KEYS[key](b.training)),
        0,
      ) || a.id - b.id,
  );
}

/** The year and month of a time as "2026-10". */
const month = (date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;

/** Whether `shop` resets as the month changes and was read in a month before `now`'s. */
export function shopStale(shop, now = new Date()) {
  return shop.resets && shop.read_at.slice(0, 7) < month(now);
}

/**
 * A good as its tile shows it: how many come with a purchase ("×5", none
 * without terms), and what each of the shop's coins buys of it, the times
 * left at the price ("12回@10", "@?" without terms, "売切れ" at none; `null`
 * where the coin does not buy it).
 */
export function goodTile(good) {
  const priced = good.offers.find((offer) => offer?.price);
  return {
    amount: priced ? `×${exact.format(priced.price.amount)}` : "",
    offers: good.offers.map((offer) => {
      if (!offer) return null;
      const sold = offer.left === 0;
      const price = offer.price ? exact.format(offer.price.price) : "?";
      return { text: sold ? "売切れ" : `${exact.format(offer.left)}回@${price}`, sold };
    }),
  };
}

/** Each shop with its goods narrowed by `query` (see `filterRows`); shops left with none go. */
export function filterShops(shops, query) {
  return shops
    .map((shop) => ({ ...shop, goods: filterRows(shop.goods, query) }))
    .filter((shop) => shop.goods.length > 0);
}

/** The lines of the plan that count no item, by kind. */
const PLAN_KINDS = {
  credits: "クレジット",
  report_exp: "レポート（経験値）",
  orb_exp: "強化珠（経験値）",
};

/** Credits in millions ("28.4M"), to a hundredth below a million. */
const millions = (n) =>
  new Intl.NumberFormat("ja-JP", { maximumFractionDigits: n < 1e6 ? 2 : 1 }).format(n / 1e6) + "M";

/** What a line of the plan holds: the count ("約" where it is in thousands), `null` unknown. */
function held(row) {
  if (!row.have) return { have: null, mark: "" };
  return "about" in row.have
    ? { have: row.have.about, mark: "約" }
    : { have: row.have.exact, mark: "" };
}

/**
 * A line of the plan as table cells: what is needed, what is held ("?"
 * unknown) and how far short it falls; credits in millions.
 */
export function needCells(row) {
  const { have, mark } = held(row);
  const format = row.kind === "credits" ? millions : (n) => exact.format(n);
  const enough = have !== null && have >= row.need;
  return {
    name: PLAN_KINDS[row.kind] ?? row.name,
    need: format(row.need),
    have: have === null ? "?" : mark + format(have),
    short: have === null ? "?" : enough ? "足りる" : `不足 ${mark}${format(row.need - have)}`,
    enough,
  };
}

/**
 * The plan's lines of the kinds that come in four tiers, one per kind with
 * its tiers in order (`null` where none is needed), and the other lines.
 */
export function groupPlan(rows) {
  const families = new Map();
  const rest = [];
  for (const row of rows) {
    if (!row.family) {
      rest.push(row);
      continue;
    }
    const [id, tier] = row.family;
    const family = families.get(id) ?? { id, tiers: [null, null, null, null] };
    family.tiers[tier - 1] = row;
    families.set(id, family);
  }
  return { families: [...families.values()].sort((a, b) => a.id - b.id), rest };
}

/** A tier of a kind as "needed / held", and whether that is enough. */
export function tierCell(row) {
  const { have, mark } = held(row);
  return {
    text: `${exact.format(row.need)} / ${have === null ? "?" : mark + exact.format(have)}`,
    enough: have !== null && have >= row.need,
  };
}

/** A star as the targets name it: the student's ★1 to ★5, then the unique weapon's 2 to 4. */
export const starName = (star) => (star <= 5 ? `★${star}` : `固有${star - 4}`);

/**
 * A student's star counted on into the unique weapon's (★5 with the weapon at
 * ★2 is 6), `null` unread: the weapon is had from ★5, so a ★5 needs it read.
 */
export function starOf(t) {
  if (!known(t.rarity)) return null;
  if (t.rarity < 5) return t.rarity;
  return t.weapon ? 4 + t.weapon.stars : null;
}

/** The columns of a target: the level, the four skills and the three equipment slots. */
const PART_NAMES = {
  level: "Lv",
  star: "★",
  skill: ["EX", "N", "P", "S"],
  equipment: ["装備1", "装備2", "装備3"],
};

const partName = (part) =>
  typeof part === "string"
    ? PART_NAMES[part]
    : Object.entries(part).map(([kind, i]) => PART_NAMES[kind][i])[0];

/** The parts of the targets that could not be counted, one line per student. */
export function unknownText(rows) {
  const byStudent = new Map();
  for (const row of rows) {
    const parts = byStudent.get(row.id) ?? { name: row.name, parts: [] };
    parts.parts.push(partName(row.part));
    byStudent.set(row.id, parts);
  }
  return [...byStudent.values()].map(({ name, parts }) => `${name}: ${parts.join("・")}`);
}

const tenths = new Intl.NumberFormat("ja-JP", {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});
const hundredths = new Intl.NumberFormat("ja-JP", { maximumFractionDigits: 2 });

/**
 * What a slot's blueprints make: how many pieces from T1 to T10, to a tenth
 * ("4.5個分"), what is held against what one piece takes ("約" where it is
 * about), and the mission that gives the most per AP ("29-5で0.26個/回").
 */
export function stockCells(stock) {
  const best = stock.best;
  return {
    pieces: `${tenths.format(stock.pieces)}個分`,
    held: `${stock.about ? "約" : ""}${exact.format(stock.held)} / ${exact.format(stock.need)}`,
    best: best ? `${best.name}で${hundredths.format(best.per_run)}個/回` : "",
  };
}

/** A daily play's place without the kind of play it is ("指名手配：ハイウェイ" is "ハイウェイ"). */
export const placeName = (name) => name.split(/：| - /).at(-1);

/**
 * The commissions against a student raised from Lv1: how many students the
 * credits and the reports' exp held raise ("6.0人分"), from what over what,
 * and which raises fewer (the one to play), none when they tie or one is unknown.
 */
export function commissionCells(c) {
  const students = (n) => (n == null ? "?" : `${tenths.format(n)}人分`);
  const fewer =
    c.credit_students == null || c.credit_students === c.report_students
      ? null
      : c.credit_students < c.report_students
        ? "credits"
        : "reports";
  return {
    credits: {
      name: c.credits_name,
      students: students(c.credit_students),
      detail: `${c.credits == null ? "?" : millions(c.credits)} ÷ ${millions(c.credits_each)}`,
    },
    reports: {
      name: c.reports_name,
      students: students(c.report_students),
      detail: `${c.report_about ? "約" : ""}${millions(c.report_exp)} EXP ÷ ${millions(c.exp_each)} EXP`,
    },
    fewer,
  };
}
