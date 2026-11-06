//! The window as a user drives it, over what a test keeps and reports in place of the game:
//! the counterparts of the Playwright suite (`e2e/app.spec.js` before GPUI).

mod app;
mod students;

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicUsize, Ordering},
};

use futures::channel::mpsc;
use gpui::{Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext, px, size};

use super::{Aoi, theme::Theme};
use crate::{
    Recognizer,
    app::{Core, Places, Snapshot},
    keep::Kept,
    settings::Settings,
    watch::{Status, Update},
};

/// The built-in data, loaded once for every test.
pub fn recognizer() -> Arc<Recognizer> {
    static RECOGNIZER: OnceLock<Arc<Recognizer>> = OnceLock::new();
    RECOGNIZER
        .get_or_init(|| Arc::new(Recognizer::embedded().expect("the built-in data loads")))
        .clone()
}

/// A folder of its own for each window, so what one keeps no other reads.
fn places() -> Places {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "aoi-ui-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    Places {
        state: dir.join("state.json"),
        latest: dir.join("Aoi").join("aoi.yaml"),
        settings: dir.join("settings.json"),
        exports: dir.join("Aoi"),
    }
}

/// The window, 720×820 as it opens, over `kept`, with the menu's `settings`.
pub fn open_with<'a>(
    cx: &'a mut TestAppContext,
    kept: &Kept,
    settings: Settings,
) -> (Entity<Aoi>, Arc<Core>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        cx.set_global(Theme::sumi());
        super::bind_keys(cx);
    });
    let (changed, changes) = mpsc::unbounded();
    let core = Arc::new(Core::new(recognizer(), places(), kept, None, changed));
    let shown = core.clone();
    let (aoi, vcx) =
        cx.add_window_view(move |window, cx| Aoi::new(shown, changes, settings, window, cx));
    vcx.simulate_resize(size(px(720.), px(820.)));
    vcx.run_until_parked();
    (aoi, core, vcx)
}

/// The window over `kept`, with the menu's defaults.
pub fn open<'a>(
    cx: &'a mut TestAppContext,
    kept: &Kept,
) -> (Entity<Aoi>, Arc<Core>, &'a mut VisualTestContext) {
    open_with(cx, kept, Settings::default())
}

/// What the watcher would report: its status, and what it read.
pub fn report(core: &Core, vcx: &mut VisualTestContext, status: &Status, update: Option<Update>) {
    core.update(|snapshot, core| {
        snapshot.apply(core.recognizer(), &core.places, status, update);
    });
    vcx.run_until_parked();
}

/// Changes the snapshot as the app would, and lets the window take it.
pub fn set(core: &Core, vcx: &mut VisualTestContext, change: impl FnOnce(&mut Snapshot)) {
    core.update(|snapshot, _| change(snapshot));
    vcx.run_until_parked();
}

fn leak(selector: String) -> &'static str {
    Box::leak(selector.into_boxed_str())
}

pub fn bounds(vcx: &mut VisualTestContext, selector: &str) -> Option<Bounds<Pixels>> {
    vcx.run_until_parked();
    vcx.debug_bounds(leak(selector.to_owned()))
}

/// Whether an element with this debug selector is on the screen.
pub fn has(vcx: &mut VisualTestContext, selector: &str) -> bool {
    bounds(vcx, selector).is_some()
}

/// Whether this text is on the screen.
pub fn shows(vcx: &mut VisualTestContext, text: &str) -> bool {
    has(vcx, &format!("text:{text}"))
}

pub fn click(vcx: &mut VisualTestContext, selector: &str) {
    let at = bounds(vcx, selector)
        .unwrap_or_else(|| panic!("{selector} is not on the screen"))
        .center();
    vcx.simulate_click(at, Modifiers::none());
    vcx.run_until_parked();
}

/// Keys pressed and released one after another, as a keyboard sends them ("tab right enter").
pub fn keys(vcx: &mut VisualTestContext, keystrokes: &str) {
    for stroke in keystrokes.split_whitespace() {
        let keystroke = gpui::Keystroke::parse(leak(stroke.to_owned())).expect("a keystroke");
        vcx.simulate_keystrokes(stroke);
        vcx.simulate_event(gpui::KeyUpEvent { keystroke });
        vcx.run_until_parked();
    }
}
