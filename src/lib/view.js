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
