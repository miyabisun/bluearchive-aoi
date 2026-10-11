import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Calls `onChange` with the watcher's snapshot ({ status, rows }) now and on
 * every change. Outside the app (the browser tests) it shows
 * `window.__AOI_DEMO__` instead.
 */
export async function follow(onChange) {
  if (!("__TAURI_INTERNALS__" in window)) {
    const demo = /** @type {any} */ (window).__AOI_DEMO__;
    onChange(demo ?? { status: { state: "no_game" }, rows: [] });
    return;
  }
  await listen("snapshot", (event) => onChange(event.payload));
  onChange(await invoke("snapshot"));
}
