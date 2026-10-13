/** The themes the menu offers: light, dark, or the OS's (自動). */
export const THEMES = [
  { id: "light", name: "ライト" },
  { id: "dark", name: "ダーク" },
  { id: "auto", name: "自動" },
];

const KEY = "aoi-theme";

/** The theme chosen before, the OS's when none was or it cannot be read. */
export function storedTheme() {
  try {
    return localStorage.getItem(KEY) ?? "auto";
  } catch {
    return "auto";
  }
}

/** Shows the page in `theme` and keeps the choice for the next start. */
export function applyTheme(theme) {
  const root = document.documentElement;
  if (theme === "auto") delete root.dataset.theme;
  else root.dataset.theme = theme;
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Unkept, the choice lasts until the window closes.
  }
}
