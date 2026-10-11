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

/** The watcher's status as a short label and a tone for its badge. */
export function statusView(status) {
  switch (status.state) {
    case "reading":
      return { label: `集計中 ${Math.round(status.fps)} fps`, tone: "active" };
    case "other_screen":
      return { label: "対象外の画面", tone: "idle" };
    case "failed":
      return { label: "キャプチャ失敗", tone: "danger" };
    default:
      return { label: "ゲーム未検出", tone: "idle" };
  }
}
