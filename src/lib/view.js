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
