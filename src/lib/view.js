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
    case "paused":
      return { label: "一時中断", tone: "paused" };
    default:
      return { label: "ゲーム未検出", tone: "idle" };
  }
}

const exact = new Intl.NumberFormat("ja-JP");

/** A count as the list shows it: exact below 10,000, in thousands ("51K") from there. */
export function formatCount(count) {
  return "thousands" in count ? `${count.thousands}K` : exact.format(count.exact);
}

const known = (v) => v !== null && v !== undefined;
const show = (v) => (known(v) ? String(v) : "?");

/**
 * A student's training as table cells, written as timeline-arona writes a
 * party: skills "5M7M" (the EX level, then M for a skill at 10), equipment
 * "T10/9/8"; the star on into the unique weapon's ("固有2", see `starName`),
 * the weapon's level apart for its hover. "?" marks what could not be read;
 * "—" what the student lacks.
 */
export function studentCells(t) {
  const [ex, ...others] = t.skills;
  const star = starOf(t);
  return {
    level: show(t.level),
    star: star === null ? "?" : starName(star),
    weapon: t.weapon ? `固有武器 Lv${t.weapon.level}` : "",
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
  // The star on into the unique weapon's, then the weapon's level.
  star: (t) => (starOf(t) === null ? null : [starOf(t), t.weapon?.level ?? 0]),
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
 * ties, and the default order, go by level, then star, then bond.
 */
export function sortStudents(rows, column = "level") {
  const order = [column, "level", "star", "bond"];
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

/** The four tiers' names, the first to the last. */
export const TIERS = ["初級", "中級", "上級", "最上級"];

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

/** The fragments `n` elephs cost from `rate` on: 20 at each rate up to 4, then 5 each. */
export function fragmentCost(rate, n) {
  let [cost, left, at] = [0, n, Math.max(1, rate)];
  while (left > 0) {
    const take = at >= 5 ? left : Math.min(20, left);
    cost += take * Math.min(at, 5);
    left -= take;
    at += 1;
  }
  return cost;
}

/** A line of the plan by its kind and id ("item-4033"). */
const lineKey = (line) => `${line.kind}-${line.id}`;

/** A source of a line: a box, a shop's coin, a slot's universal blueprints, or the fragments. */
function sourceKey(source) {
  switch (source.type) {
    case "box":
      return `box:${source.id}`;
    case "shop":
      return `shop:${source.shop}:${source.coin}`;
    case "universal":
      return `universal:${source.id}`;
    default:
      return "fragments";
  }
}

/** What a source spends: the coin, which shops share; else the source itself. */
const resourceKey = (source) =>
  source.type === "shop" ? `coin:${source.coin}` : sourceKey(source);

/** What paying `n` from `source` gives its line and costs its resource. */
function paid(source, n) {
  switch (source.type) {
    case "shop":
      return { gives: n * source.gives, cost: n * source.price };
    case "fragments":
      return { gives: n, cost: fragmentCost(source.rate, n) };
    case "universal":
      return { gives: n, cost: n * source.rate };
    default:
      return { gives: n, cost: n };
  }
}

/** Whether a source sells only so many: a shop this month, or the fragments. */
const limited = (source) => source.type === "shop" || source.type === "fragments";

/**
 * How many to pay from `source` after a click, `n` paid and `lacking` still
 * short: one more, or with shift as many as make the line up; one fewer with
 * a right click, none with shift. A box may be opened past those held (the
 * rest can be tailor-made), and universal blueprints spent past them (the
 * rest can be farmed); a shop sells no more than it has left.
 */
export function step(source, n, lacking, { right = false, shift = false } = {}) {
  if (right) return shift ? 0 : Math.max(0, n - 1);
  const each = source.type === "shop" ? source.gives : 1;
  const more = shift ? Math.ceil(Math.max(0, lacking) / each) : 1;
  return Math.min(limited(source) ? source.left : Infinity, n + more);
}

/**
 * The `payments` (by `line|source` key) settled: each line with what is
 * given and what it still lacks, its sources as chips (how many paid, what
 * they give, how much of the resource is left after every line's payments),
 * and each resource held, spent and left after (`null` where not read).
 */
export function settle(lines, payments) {
  const resources = new Map();
  const settled = lines.map((line) => {
    let given = 0;
    const chips = line.sources.map((source) => {
      const key = sourceKey(source);
      const n = payments[`${lineKey(line)}|${key}`] ?? 0;
      const { gives, cost } = paid(source, n);
      given += gives;
      const at = resourceKey(source);
      const resource = resources.get(at) ?? {
        key: at,
        name: source.type === "shop" ? source.coin : source.name,
        icon: source.icon,
        held: source.held,
        spent: 0,
      };
      resource.spent += cost;
      resources.set(at, resource);
      return { source, key, at, n, gives };
    });
    return { line, key: lineKey(line), given, left: Math.max(0, line.short - given), chips };
  });
  const after = (r) => (r.held == null ? null : r.held - r.spent);
  return {
    lines: settled.map((l) => ({
      ...l,
      chips: l.chips.map(({ at, ...chip }) => ({ ...chip, after: after(resources.get(at)) })),
    })),
    resources: [...resources.values()].map((r) => ({ ...r, after: after(r) })),
  };
}

/** What a source is, on hover: the box, the shop's terms, or the fragments' rate. */
function sourceTitle(source) {
  switch (source.type) {
    case "shop":
      return `${source.shop}: ${source.coin} ${exact.format(source.price)} で ×${exact.format(source.gives)}、今月あと ${exact.format(source.left)} 回`;
    case "fragments":
      return `${source.name}: 1 文字 ${source.rate} カケラから、あと ${exact.format(source.left)} 文字`;
    case "universal":
      return `${source.name}: T10設計図 1 枚に ${source.rate} 枚（T2〜T9 に使った残り ${exact.format(source.held)} 枚）`;
    default:
      return `${source.name}（${source.choices} 種から 1 つ）`;
  }
}

/**
 * The settled procurement (see `settle`) as text: each line's shortfall and
 * what it still lacks (credits in millions), its chips (the resource left
 * after, "?" not read, "売切れ" once a shop sells no more, and what the chip
 * gives, "+20"), and each resource paid
 * from: held, spent ("-30") and left after, above none "plus", below "minus".
 */
export function procureCells(settled) {
  const amount = (kind, n) => (kind === "credits" ? millions(n) : exact.format(n));
  const count = (n) => (n == null ? "?" : exact.format(n));
  return {
    lines: settled.lines.map((l) => ({
      key: l.key,
      name: PLAN_KINDS[l.line.kind] ?? l.line.name,
      icon: l.line.icon,
      short: amount(l.line.kind, l.line.short),
      left: amount(l.line.kind, l.left),
      done: l.left === 0,
      chips: l.chips.map((c) => {
        // A shop's purchases this month, or the fragments' exchanges, all paid.
        const out = limited(c.source) && c.n >= c.source.left;
        return {
          key: c.key,
          icon: c.source.icon,
          text: out ? "売切れ" : count(c.after),
          negative: !out && c.after != null && c.after < 0,
          given: c.n > 0 ? `+${amount(l.line.kind, c.gives)}` : "",
          out,
          title: sourceTitle(c.source),
        };
      }),
    })),
    resources: settled.resources
      .filter((r) => r.spent > 0)
      .map((r) => ({
        key: r.key,
        name: r.name,
        icon: r.icon,
        held: count(r.held),
        spent: `-${exact.format(r.spent)}`,
        after: count(r.after),
        tone: r.after == null || r.after === 0 ? "" : r.after > 0 ? "plus" : "minus",
      })),
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

/**
 * A slot's blueprints (see the plan's `BlueprintRow`) as cells: each tier
 * "needed / held" (nothing where none is needed), T10 coloured by whether it
 * is enough, the tiers below paid the rest by the universal ones; and what
 * T2 to T9 take of those over the ones held, coloured as T10.
 */
export function blueprintCells(row) {
  const universal = tierCell(row.universal);
  return {
    name: row.name,
    icon: row.icon,
    tiers: row.tiers.map((t) => {
      const cell = tierCell(t);
      return {
        key: t.id,
        label: `T${t.tier}`,
        name: t.name,
        icon: t.icon,
        text: t.need > 0 ? cell.text : "",
        tone: t.tier === 10 && t.need > 0 ? (cell.enough ? "enough" : "short") : "",
      };
    }),
    universal: {
      name: row.universal.name,
      icon: row.universal.icon,
      text: universal.text,
      tone: universal.enough ? "enough" : "short",
    },
  };
}

/** A tier of a kind as "needed / held", and whether that is enough. */
export function tierCell(row) {
  const { have, mark } = held(row);
  return {
    text: `${exact.format(row.need)} / ${have === null ? "?" : mark + exact.format(have)}`,
    enough: have !== null && have >= row.need,
  };
}

/**
 * A star as a teacher names it: the student's ★1 to ★4, then the unique
 * weapon's 固有1 to 固有4 (★5 is the weapon's first, user's naming of 2026-10-13).
 */
export const starName = (star) => (star <= 4 ? `★${star}` : `固有${star - 4}`);

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
  star: "星",
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

/**
 * What a slot's blueprints make: how many pieces from T1 to T10, to a tenth
 * ("4.5個分"), what is held against what one piece takes ("約" where it is
 * about), and how short it falls: of one piece "short", of the slot's
 * `golden` ratio "low".
 */
export function stockCells(stock, golden) {
  return {
    pieces: `${tenths.format(stock.pieces)}個分`,
    held: `${stock.about ? "約" : ""}${exact.format(stock.held)} / ${exact.format(stock.need)}`,
    tone: stock.pieces < 1 ? "short" : stock.pieces < golden ? "low" : "",
  };
}

/** How many a count holds ("51K" about 51,000; none 0). */
const amountOf = (count) => (count == null ? 0 : (count.exact ?? count.thousands * 1000));

/**
 * How a drop held stands against what students need: short of one "short"
 * (red), of two "fair" (blue), two or more "enough" (green).
 */
export function heldTone(held, need) {
  if (need == null) return "";
  const n = amountOf(held);
  return n < need ? "short" : n < 2 * need ? "fair" : "enough";
}

/** The farm to the golden ratio: each mission's runs, AP and the slots it serves, and the totals. */
export function farmCells(farm) {
  return {
    lines: farm.stages.map((s) => ({
      name: s.name,
      runs: `${exact.format(s.runs)}回`,
      ap: `${exact.format(s.ap)}AP`,
      slots: s.slots.join("・"),
    })),
    total: { runs: `${exact.format(farm.runs)}回`, ap: `${exact.format(farm.ap)}AP` },
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

/** Where checking for an update and taking it stands, as the window tells it. */
export function updateText(update) {
  switch (update.state) {
    case "checking":
      return "アップデートを確認しています…";
    case "latest":
      return `最新の版です（${update.current}）`;
    case "available":
      return `${update.latest} があります（今 ${update.current}）`;
    case "installing":
      return `${update.latest} に更新しています…`;
    default:
      return `アップデートできませんでした: ${update.error}`;
  }
}
