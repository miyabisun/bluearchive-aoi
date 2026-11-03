/** The themes the menu offers: light, dark, or the OS's (自動). */
export const THEMES = [
  { id: "light", name: "ライト" },
  { id: "dark", name: "ダーク" },
  { id: "auto", name: "自動" },
];

/**
 * The mouse cursors the menu offers (the user's of 2026-11-03): Windows' own, or an arrow after
 * the game's in blue, red or green, as the game's own highlighted cursor would show in what is read.
 */
export const CURSORS = [
  { id: "none", name: "なし" },
  { id: "blue", name: "青" },
  { id: "red", name: "赤" },
  { id: "green", name: "緑" },
];

const KEY = "aoi-theme";
const CURSOR_KEY = "aoi-cursor";

/** What was kept under `key`, `fallback` when nothing was or it cannot be read. */
function recall(key, fallback) {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

/** Keeps `value` under `key` for the next start. */
function remember(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Unkept, the choice lasts until the window closes.
  }
}

/** The theme chosen before, the OS's when none was or it cannot be read. */
export function storedTheme() {
  return recall(KEY, "auto");
}

/** Shows the page in `theme` and keeps the choice for the next start. */
export function applyTheme(theme) {
  const root = document.documentElement;
  if (theme === "auto") delete root.dataset.theme;
  else root.dataset.theme = theme;
  remember(KEY, theme);
}

/** The cursor chosen before, Windows' own when none was or it cannot be read. */
export function storedCursor() {
  return recall(CURSOR_KEY, "none");
}

/** Keeps the cursor chosen for the next start. */
export function keepCursor(cursor) {
  remember(CURSOR_KEY, cursor);
}
