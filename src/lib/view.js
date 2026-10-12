/** Folds full-width forms and case so a query matches however it is typed. */
const fold = (text) => text.normalize("NFKC").toLowerCase();

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
  { id: "students", name: "生徒" },
  { id: "shops", name: "ショップ" },
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
 * An offer as table cells: the times it can still be bought ("買い切り" at
 * none), and its price, "?" unread, after "左" or "右" for the coin of a shop
 * that takes two (`coins`).
 */
export function offerCells(offer, coins) {
  const price = known(offer.price) ? exact.format(offer.price) : "?";
  return {
    left: offer.left === 0 ? "買い切り" : String(offer.left),
    price: coins > 1 ? `${offer.coin === 0 ? "左" : "右"} ${price}` : price,
  };
}

/** Each shop with its offers narrowed by `query` (see `filterRows`); shops left with none go. */
export function filterShops(shops, query) {
  return shops
    .map((shop) => ({ ...shop, offers: filterRows(shop.offers, query) }))
    .filter((shop) => shop.offers.length > 0);
}
