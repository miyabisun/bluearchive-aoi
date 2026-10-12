import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Calls `onChange` with the watcher's snapshot ({ status, items, equipment, students }) now and on
 * every change. Outside the app (the browser tests) it shows
 * `window.__AOI_DEMO__` instead.
 */
export async function follow(onChange) {
  if (!("__TAURI_INTERNALS__" in window)) {
    const demo = /** @type {any} */ (window).__AOI_DEMO__;
    onChange(demo ?? { status: { state: "no_game" }, items: [], equipment: [] });
    return;
  }
  await listen("snapshot", (event) => onChange(event.payload));
  onChange(await invoke("snapshot"));
}

/**
 * Writes everything read to a YAML file and resolves to its path; rejects with
 * the reason it could not. Outside the app it answers with the demo's
 * `export` (a path, or `{ error }`).
 */
export async function exportAll() {
  if (!("__TAURI_INTERNALS__" in window)) {
    const answer = /** @type {any} */ (window).__AOI_DEMO__?.export;
    if (answer?.error) throw answer.error;
    return answer;
  }
  return invoke("export");
}

/**
 * Marks the student `id` for training up to `target`, or unmarks them
 * (`null`); the app answers with a new snapshot. Outside the app it notes the
 * call in `window.__AOI_CALLS__`.
 */
export async function setTarget(id, target) {
  if (!("__TAURI_INTERNALS__" in window)) {
    const w = /** @type {any} */ (window);
    (w.__AOI_CALLS__ ??= []).push({ id, target });
    return;
  }
  return invoke("set_target", { id, target });
}
