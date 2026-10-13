import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Calls `onChange` with the watcher's snapshot ({ status, items, equipment, students }) now and on
 * every change. Outside the app (the browser tests) it shows
 * `window.__AOI_DEMO__` instead, and `window.__AOI_PUSH__(snapshot)` sends a later one.
 */
export async function follow(onChange) {
  if (!("__TAURI_INTERNALS__" in window)) {
    const w = /** @type {any} */ (window);
    w.__AOI_PUSH__ = onChange;
    onChange(w.__AOI_DEMO__ ?? { status: { state: "no_game" }, items: [], equipment: [] });
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

/**
 * Where the window loads the icon `name` from: the app's `icon` protocol;
 * outside the app `/icon/<name>`, which the browser tests answer.
 */
export const iconUrl = (name) =>
  "__TAURI_INTERNALS__" in window ? convertFileSrc(name, "icon") : `/icon/${name}`;

/**
 * The app's version and the newer one the latest GitHub release brings
 * (`latest`, none when up to date); rejects with why it could not tell.
 * Outside the app it answers with the demo's `update`.
 */
export async function checkUpdate() {
  if (!("__TAURI_INTERNALS__" in window)) {
    const answer = /** @type {any} */ (window).__AOI_DEMO__?.update;
    if (answer?.error) throw answer.error;
    return answer ?? { current: "v0.0.0", latest: null };
  }
  return invoke("check_update");
}

/**
 * Puts the latest release in place of the running app and starts it; rejects
 * with why it could not. Outside the app it notes the call in
 * `window.__AOI_CALLS__`.
 */
export async function applyUpdate() {
  if (!("__TAURI_INTERNALS__" in window)) {
    const w = /** @type {any} */ (window);
    (w.__AOI_CALLS__ ??= []).push({ update: true });
    return;
  }
  return invoke("apply_update");
}
