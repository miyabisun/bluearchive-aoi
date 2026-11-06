//! What the window shows and does: the snapshot the watcher keeps current, kept on disk, and the
//! window's commands (marking students, reserving cards, exporting, pausing, the phone and the
//! streaming app, the cursor, updates).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, PoisonError},
};

use futures::channel::mpsc::UnboundedSender;
use serde::Serialize;

use crate::{
    List, Recognizer, Row,
    android::Link,
    broadcast::Cast,
    cursor::{Highlight, Palette},
    export,
    ground::{self, Behind, Ground},
    keep::{self, Kept, ReadAt, now},
    kinds::Kinds,
    plan::{BlueprintRow, NeedRow, Target, UnknownRow},
    shop::ShopRow,
    strategy::{PlanShop, Reserve},
    student::Training,
    supply::ProcureLine,
    tally::Tally,
    update,
    watch::{self, Place, Status, StudentRow, Update},
    work::{HardCard, WorkRows},
};

/// Everything the window shows; sent whole on every change.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub status: Status,
    /// Where the game is, for the work tab to open what it shows.
    pub place: Option<Place>,
    pub items: Vec<Row>,
    pub equipment: Vec<Row>,
    pub students: Vec<StudentRow>,
    pub credits: Option<u64>,
    pub shops: Vec<ShopRow>,
    /// The students marked for training and their targets.
    pub targets: Vec<TargetRow>,
    /// What the targets take against what is held, and what could not be counted.
    pub plan: Vec<NeedRow>,
    /// The blueprints the targets take, slot by slot.
    pub blueprints: Vec<BlueprintRow>,
    pub plan_unknown: Vec<UnknownRow>,
    /// The plan's lines that fall short, with what could make each up.
    pub procure: Vec<ProcureLine>,
    /// Counts the times the plan's ground was read again (`reload_plan`): the
    /// window's payments hold until it changes.
    pub plan_version: u32,
    /// What the plan counts from (see `Ground`).
    #[serde(skip)]
    pub ground: Ground,
    /// What was read since the plan's ground and counts otherwise, none while it counts alike.
    pub plan_behind: Option<Behind>,
    /// The item and equipment tabs' tables by kind.
    pub kinds: Kinds,
    /// The daily plays against what is held.
    pub work: WorkRows,
    /// The students' elephs the hard missions drop, those reserved marked.
    pub hard: Vec<HardCard>,
    /// The month-end shops' purchase plan.
    pub month_end: Vec<PlanShop>,
    /// The cards reserved in it.
    #[serde(skip)]
    pub reserved: BTreeSet<Reserve>,
    pub read_at: ReadAt,
    /// Why what was read could not be kept or brought back, while it lasts.
    pub keep_error: Option<String>,
    /// Where the link to a phone stands.
    pub phone: Link,
    /// Where waiting for a streaming app stands.
    pub cast: Cast,
}

/// A student marked for training, with the target, and the student's name
/// and training as the plan counts them from (none for a student never read).
#[derive(Debug, Clone, Serialize)]
pub struct TargetRow {
    pub id: i64,
    pub target: Target,
    pub name: String,
    pub training: Option<Training>,
}

/// Where what is read is kept: `state.json` in the app's folder, brought back
/// on the next run, and `Documents\Aoi\aoi.yaml`, the latest of it to read.
#[derive(Clone)]
pub struct Places {
    pub state: PathBuf,
    pub latest: PathBuf,
    /// Where the window's choices are kept (see `settings`).
    pub settings: PathBuf,
    /// Where an export goes: `Documents\Aoi`.
    pub exports: PathBuf,
}

impl Snapshot {
    /// The window as it starts: what was kept, before anything is read.
    pub fn starting(recognizer: &Recognizer, kept: &Kept, keep_error: Option<String>) -> Self {
        let rows = |list, counts: &std::collections::BTreeMap<i64, crate::quantity::Count>| {
            recognizer.rows(list, &Tally::seeded(counts.clone().into_iter().collect()))
        };
        let mut snapshot = Self {
            status: Status::NoGame,
            place: None,
            items: rows(List::Items, &kept.items),
            equipment: rows(List::Equipment, &kept.equipment),
            students: watch::student_rows(recognizer, &kept.students),
            credits: kept.credits,
            shops: recognizer.shop_rows(&kept.shops, &kept.items.clone().into_iter().collect()),
            targets: Vec::new(),
            plan: Vec::new(),
            blueprints: Vec::new(),
            plan_unknown: Vec::new(),
            procure: Vec::new(),
            plan_version: 0,
            ground: Ground::default(),
            plan_behind: None,
            kinds: Kinds::default(),
            work: recognizer.work_rows(&[], &[], None),
            hard: Vec::new(),
            month_end: Vec::new(),
            reserved: kept.reserved.clone(),
            read_at: kept.read_at.clone(),
            keep_error,
            phone: Link::Unpaired,
            cast: Cast::Off,
        };
        snapshot.reground();
        snapshot.replan(recognizer, &kept.targets);
        snapshot.rework(recognizer);
        snapshot
    }

    /// What was read, as the plan's ground takes it.
    fn reads(&self) -> Ground {
        Ground {
            students: self.students.clone(),
            items: self.items.clone(),
            equipment: self.equipment.clone(),
            credits: self.credits,
            shops: self.shops.clone(),
            read_at: ground::newest(&self.read_at, &self.shops),
        }
    }

    /// Takes what was read as the plan's ground (see `Snapshot::ground`).
    fn reground(&mut self) {
        self.ground = self.reads();
        self.plan_version += 1;
    }

    /// Tells what was read since the plan's ground and counts otherwise (see `Snapshot::plan_behind`).
    fn retell(&mut self) {
        let marked: Vec<i64> = self.targets.iter().map(|t| t.id).collect();
        self.plan_behind = self.ground.behind(&self.reads(), &marked);
    }

    pub fn targets(&self) -> BTreeMap<i64, Target> {
        self.targets.iter().map(|t| (t.id, t.target)).collect()
    }

    /// Counts the plan for `targets` from its ground.
    fn replan(&mut self, recognizer: &Recognizer, targets: &BTreeMap<i64, Target>) {
        let ground = &self.ground;
        let student = |id: i64| {
            ground
                .students
                .iter()
                .chain(&self.students)
                .find(|s| s.id == id)
        };
        self.targets = targets
            .iter()
            .map(|(&id, &target)| TargetRow {
                id,
                target,
                name: student(id).map(|s| s.name.clone()).unwrap_or_default(),
                training: student(id).map(|s| s.training),
            })
            .collect();
        // A student read since the ground was taken counts as read now.
        let students: Vec<StudentRow> = targets
            .keys()
            .filter_map(|&id| student(id).cloned())
            .collect();
        (self.plan, self.blueprints, self.plan_unknown) = recognizer.plan_rows(
            targets,
            &students,
            &ground.items,
            &ground.equipment,
            ground.credits,
        );
        self.procure = recognizer.procure_rows(
            &self.plan,
            &self.blueprints,
            &ground.items,
            &ground.shops,
            &now(),
        );
        self.retell();
    }

    /// Counts the tables by kind, the daily plays and the month-end plan again
    /// from what was read.
    fn rework(&mut self, recognizer: &Recognizer) {
        self.kinds = recognizer.kinds(&self.items, &self.equipment);
        self.work = recognizer.work_rows(&self.items, &self.equipment, self.credits);
        self.hard = recognizer.hard(&self.items, &self.students, &self.reserved);
        let held = |id| crate::good_held(id, &self.items, &self.equipment, self.credits);
        self.month_end = recognizer.month_end(&self.shops, &self.students, held, &self.reserved);
        self.retell();
    }

    /// What is kept of the window.
    fn kept(&self) -> Kept {
        let counts = |rows: &[Row]| rows.iter().map(|r| (r.id, r.count)).collect();
        Kept {
            items: counts(&self.items),
            equipment: counts(&self.equipment),
            students: self.students.iter().map(|s| (s.id, s.training)).collect(),
            credits: self.credits,
            shops: self
                .shops
                .iter()
                .map(|s| (s.name.clone(), s.shelf()))
                .collect(),
            targets: self.targets(),
            reserved: self.reserved.clone(),
            read_at: self.read_at.clone(),
        }
    }

    /// Everything read, as YAML headed by `stamp`, and the cards reserved.
    pub fn yaml(&self, stamp: &str) -> String {
        let named = |id: i64| {
            self.shops
                .iter()
                .flat_map(|s| &s.goods)
                .find(|g| g.id == id)
                .map(|g| g.name.clone())
        };
        export::yaml(
            stamp,
            &self.read_at,
            self.credits,
            &self.students,
            &self.items,
            &self.equipment,
            &self.shops,
        ) + &export::reserved_yaml(&self.reserved, named)
    }
}

/// Keeps what the window shows in both places.
fn keep_in(places: &Places, snapshot: &Snapshot) -> Result<(), String> {
    keep::save(&places.state, &snapshot.kept())?;
    keep::write(&places.latest, snapshot.yaml(&now()).as_bytes())
}


impl Snapshot {
    /// Takes in what the watcher reports: its status, and what it read or where the phone,
    /// the streaming app or the game now stands. What was read is counted again and kept.
    pub fn apply(
        &mut self,
        recognizer: &Recognizer,
        places: &Places,
        status: &Status,
        update: Option<Update>,
    ) {
        self.status = status.clone();
        let Some(update) = update else { return };
        let read = Some(now());
        match update {
            Update::Phone(link) => return self.phone = link,
            Update::Cast(cast) => return self.cast = cast,
            Update::Place(place) => return self.place = place,
            Update::List(List::Items, rows) => {
                self.items = rows;
                self.read_at.items = read;
            }
            Update::List(List::Equipment, rows) => {
                self.equipment = rows;
                self.read_at.equipment = read;
            }
            Update::Students(rows) => {
                self.students = rows;
                self.read_at.students = read;
            }
            Update::Credits(credits) => {
                self.credits = Some(credits);
                self.read_at.credits = read;
            }
            // Each shop carries when it was read.
            Update::Shops(rows) => self.shops = rows,
        }
        self.rework(recognizer);
        self.keep_error = keep_in(places, self)
            .err()
            .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
    }
}

/// The app's version and the newer one the latest release brings, if any;
/// whether the app takes it itself, or opens the release page (macOS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub current: String,
    pub latest: Option<String>,
    pub installs: bool,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Asks GitHub for the latest release (see `update`); blocks on the network.
pub fn check_update() -> Result<UpdateCheck, String> {
    let tag = update::latest()?;
    Ok(UpdateCheck {
        current: format!("v{VERSION}"),
        latest: update::update_of(&tag, VERSION).map(|u| u.version),
        installs: cfg!(windows),
    })
}

/// Puts the latest release in place of the running exe (Windows); blocks on the network.
/// The caller starts the new exe and ends this one.
#[cfg(windows)]
pub fn apply_update() -> Result<(), String> {
    let tag = update::latest()?;
    let update = update::update_of(&tag, VERSION).ok_or("最新のバージョンです")?;
    update::install(&update)
}

/// Opens the latest release's page: the app cannot replace itself here.
#[cfg(not(windows))]
pub fn apply_update() -> Result<(), String> {
    update::open_page()
}

/// Starts the exe now in place (the new version) and ends this one.
pub fn restart(cx: &mut gpui::App) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    cx.quit();
}

/// What the window works with: the snapshot the watcher keeps current, and what the
/// window's commands need. Every change is signalled, so the window takes the new snapshot.
pub struct Core {
    latest: Mutex<Snapshot>,
    recognizer: Arc<Recognizer>,
    pub places: Places,
    highlight: Highlight,
    watcher: OnceLock<watch::Watcher>,
    changed: UnboundedSender<()>,
}

impl Core {
    /// The window's data as it starts from what was `kept` (and why it could not be, if so).
    pub fn new(
        recognizer: Arc<Recognizer>,
        places: Places,
        kept: &Kept,
        keep_error: Option<String>,
        changed: UnboundedSender<()>,
    ) -> Self {
        Self {
            latest: Mutex::new(Snapshot::starting(&recognizer, kept, keep_error)),
            recognizer,
            places,
            highlight: Highlight::start(),
            watcher: OnceLock::new(),
            changed,
        }
    }

    /// Starts reading the game (its phone's key and the phone paired sit beside what is kept).
    pub fn watch(self: &Arc<Self>, kept: Kept, phone_dir: PathBuf) {
        let core = Arc::downgrade(self);
        let watcher = watch::start(
            self.recognizer.clone(),
            kept,
            phone_dir,
            move |status, update| {
                if let Some(core) = core.upgrade() {
                    core.update(|snapshot, core| {
                        snapshot.apply(&core.recognizer, &core.places, status, update);
                    });
                }
            },
        );
        let _ = self.watcher.set(watcher);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Snapshot> {
        self.latest.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Changes the snapshot and signals it.
    pub fn update<T>(&self, change: impl FnOnce(&mut Snapshot, &Self) -> T) -> T {
        let result = change(&mut self.lock(), self);
        let _ = self.changed.unbounded_send(());
        result
    }

    /// What the snapshot is counted with.
    pub fn recognizer(&self) -> &Recognizer {
        &self.recognizer
    }

    /// The snapshot now.
    pub fn snapshot(&self) -> Snapshot {
        self.lock().clone()
    }

    /// Marks the student `id` for training up to `target`, or unmarks them
    /// (`None`); the plan follows and is kept.
    pub fn set_target(&self, id: i64, target: Option<Target>) -> Result<(), String> {
        self.update(|snapshot, core| {
            let mut targets = snapshot.targets();
            match target {
                Some(target) => targets.insert(id, target),
                None => targets.remove(&id),
            };
            snapshot.replan(&core.recognizer, &targets);
            let kept = keep_in(&core.places, snapshot);
            snapshot.keep_error = kept
                .as_ref()
                .err()
                .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
            kept
        })
    }

    /// Reserves a month-end shop's `card`, or takes the reservation back; kept.
    pub fn set_reserved(&self, card: Reserve, reserved: bool) -> Result<(), String> {
        self.update(|snapshot, core| {
            if reserved {
                snapshot.reserved.insert(card);
            } else {
                snapshot.reserved.remove(&card);
            }
            snapshot.rework(&core.recognizer);
            let kept = keep_in(&core.places, snapshot);
            snapshot.keep_error = kept
                .as_ref()
                .err()
                .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
            kept
        })
    }

    /// Takes what was read now as the plan's ground and counts the plan again
    /// (the window's 再読込).
    pub fn reload_plan(&self) {
        self.update(|snapshot, core| {
            snapshot.reground();
            let targets = snapshot.targets();
            snapshot.replan(&core.recognizer, &targets);
        });
    }

    /// Writes everything read to `<Documents>\Aoi\aoi-<date>-<time>.yaml`; the path
    /// written, or why it could not be.
    pub fn export(&self) -> Result<String, String> {
        let snapshot = self.snapshot();
        let path = self.places.exports.join(format!(
            "aoi-{}.yaml",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        keep::write(&path, snapshot.yaml(&now()).as_bytes())?;
        Ok(path.display().to_string())
    }

    /// Pauses reading the game, or resumes it; the status tells which.
    pub fn set_paused(&self, paused: bool) {
        if let Some(watcher) = self.watcher.get() {
            watcher.pause(paused);
        }
    }

    /// Shows a QR code to pair a phone with, and with `usb` takes one plugged in
    /// too (see `android::Phone::pair`); the snapshot carries it.
    pub fn pair_phone(&self, usb: bool) {
        if let Some(watcher) = self.watcher.get() {
            watcher.phone().pair(usb);
        }
    }

    /// Connects to the paired phone, or leaves it unconnected.
    pub fn connect_phone(&self, connect: bool) {
        if let Some(watcher) = self.watcher.get() {
            if connect {
                watcher.phone().connect();
            } else {
                watcher.phone().stop();
            }
        }
    }

    /// Waits for a streaming app under a new key, or stops waiting; the snapshot
    /// carries where it stands.
    pub fn wait_for_stream(&self, wait: bool) {
        if let Some(watcher) = self.watcher.get() {
            if wait {
                watcher.cast().start();
            } else {
                watcher.cast().stop();
            }
        }
    }

    /// Puts the arrow in `palette` in for Windows' arrow cursor, or Windows' own back
    /// (none); whether the cursor can be changed here (Windows only).
    pub fn set_cursor(&self, palette: Option<Palette>) -> bool {
        self.highlight.set(palette);
        cfg!(windows)
    }
}

impl Drop for Core {
    /// Windows' own cursor back as the app ends: it is changed only while the app runs.
    fn drop(&mut self) {
        self.highlight.set(None);
    }
}

/// The app's identifier, the name of its data folder.
pub const IDENTIFIER: &str = "io.github.miyabisun.bluearchive-aoi";

/// Where the app keeps what it read, its settings and its exports: the app's data folder
/// (`%APPDATA%\io.github.miyabisun.bluearchive-aoi` on Windows, where the Tauri versions kept
/// them) and `Documents\Aoi`.
pub fn places() -> Result<Places, String> {
    let data = dirs::data_dir()
        .ok_or("アプリのデータの置き場所が分かりません")?
        .join(IDENTIFIER);
    let documents = dirs::document_dir()
        .ok_or("ドキュメントフォルダが分かりません")?
        .join("Aoi");
    Ok(Places {
        state: data.join("state.json"),
        latest: documents.join("aoi.yaml"),
        settings: data.join("settings.json"),
        exports: documents,
    })
}

/// What was kept, and why it could not be brought back, if so.
pub fn load_kept(recognizer: &Recognizer, places: &Places) -> (Kept, Option<String>) {
    let (mut kept, trouble) = match keep::load(&places.state) {
        Ok(kept) => (kept, None),
        Err(e) => (
            Kept::default(),
            Some(format!("前回の読み取り結果を読み戻せませんでした: {e}")),
        ),
    };
    kept.renumber(recognizer.renamed());
    (kept, trouble)
}

pub fn run() {
    update::clean_up();
    let recognizer = Arc::new(Recognizer::embedded().expect("the built-in data is broken"));
    let places = places().expect("the app's folders are known");
    crate::ui::run(recognizer, places);
}
