//! The window: shows what the watcher sees and the tally, and keeps it.

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    List, Recognizer, Row, export,
    keep::{self, Kept, ReadAt, now},
    plan::{NeedRow, Target, UnknownRow},
    shop::ShopRow,
    tally::Tally,
    watch::{self, Status, StudentRow, Update},
};

/// Everything the window shows; sent whole on every change.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    status: Status,
    items: Vec<Row>,
    equipment: Vec<Row>,
    students: Vec<StudentRow>,
    credits: Option<u64>,
    shops: Vec<ShopRow>,
    /// The students marked for training and their targets.
    targets: Vec<TargetRow>,
    /// What the targets take against what is held, and what could not be counted.
    plan: Vec<NeedRow>,
    plan_unknown: Vec<UnknownRow>,
    read_at: ReadAt,
    /// Why what was read could not be kept or brought back, while it lasts.
    keep_error: Option<String>,
}

/// A student marked for training, with the target.
#[derive(Debug, Clone, Serialize)]
struct TargetRow {
    id: i64,
    target: Target,
}

/// Where what is read is kept: `state.json` in the app's folder, brought back
/// on the next run, and `Documents\Aoi\aoi.yaml`, the latest of it to read.
#[derive(Clone)]
struct Places {
    state: PathBuf,
    latest: PathBuf,
}

impl Snapshot {
    /// The window as it starts: what was kept, before anything is read.
    fn starting(recognizer: &Recognizer, kept: &Kept, keep_error: Option<String>) -> Self {
        let rows = |list, counts: &std::collections::BTreeMap<i64, crate::quantity::Count>| {
            recognizer.rows(list, &Tally::seeded(counts.clone().into_iter().collect()))
        };
        let mut snapshot = Self {
            status: Status::NoGame,
            items: rows(List::Items, &kept.items),
            equipment: rows(List::Equipment, &kept.equipment),
            students: watch::student_rows(recognizer, &kept.students),
            credits: kept.credits,
            shops: recognizer.shop_rows(&kept.shops, &kept.items.clone().into_iter().collect()),
            targets: kept
                .targets
                .iter()
                .map(|(&id, &target)| TargetRow { id, target })
                .collect(),
            plan: Vec::new(),
            plan_unknown: Vec::new(),
            read_at: kept.read_at.clone(),
            keep_error,
        };
        snapshot.replan(recognizer);
        snapshot
    }

    fn targets(&self) -> BTreeMap<i64, Target> {
        self.targets.iter().map(|t| (t.id, t.target)).collect()
    }

    /// Counts the plan again from the targets and what was read.
    fn replan(&mut self, recognizer: &Recognizer) {
        (self.plan, self.plan_unknown) = recognizer.plan_rows(
            &self.targets(),
            &self.students,
            &self.items,
            &self.equipment,
            self.credits,
        );
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
            read_at: self.read_at.clone(),
        }
    }

    /// Everything read, as YAML headed by `stamp`.
    fn yaml(&self, stamp: &str) -> String {
        export::yaml(
            stamp,
            &self.read_at,
            self.credits,
            &self.students,
            &self.items,
            &self.equipment,
            &self.shops,
        )
    }
}

/// Keeps what the window shows in both places.
fn keep_in(places: &Places, snapshot: &Snapshot) -> Result<(), String> {
    keep::save(&places.state, &snapshot.kept())?;
    keep::write(&places.latest, snapshot.yaml(&now()).as_bytes())
}

/// The newest snapshot, for a page that loads after it was sent.
struct Latest(Mutex<Snapshot>);

/// The event carrying each new snapshot.
const EVENT: &str = "snapshot";

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri hands command arguments over by value.
fn snapshot(latest: tauri::State<'_, Latest>) -> Snapshot {
    latest
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Marks the student `id` for training up to `target`, or unmarks them
/// (`None`); the plan follows and is kept.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri hands command arguments over by value.
fn set_target(
    id: i64,
    target: Option<Target>,
    app: tauri::AppHandle,
    latest: tauri::State<'_, Latest>,
    recognizer: tauri::State<'_, Arc<Recognizer>>,
    places: tauri::State<'_, Places>,
) -> Result<(), String> {
    let mut snapshot = latest.0.lock().unwrap_or_else(PoisonError::into_inner);
    let mut targets = snapshot.targets();
    match target {
        Some(target) => targets.insert(id, target),
        None => targets.remove(&id),
    };
    snapshot.targets = targets
        .into_iter()
        .map(|(id, target)| TargetRow { id, target })
        .collect();
    snapshot.replan(&recognizer);
    let kept = keep_in(&places, &snapshot);
    snapshot.keep_error = kept
        .as_ref()
        .err()
        .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
    let _ = app.emit(EVENT, &*snapshot);
    kept
}

/// Writes everything read to `<Documents>\Aoi\aoi-<date>-<time>.yaml`; the path
/// written, or why it could not be.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri hands command arguments over by value.
fn export(app: tauri::AppHandle, latest: tauri::State<'_, Latest>) -> Result<String, String> {
    let snapshot = latest
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("Aoi");
    let path = dir.join(format!(
        "aoi-{}.yaml",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    keep::write(&path, snapshot.yaml(&now()).as_bytes())?;
    Ok(path.display().to_string())
}

pub fn run(context: tauri::Context) {
    let recognizer = Arc::new(Recognizer::embedded().expect("the built-in data is broken"));
    tauri::Builder::default()
        .setup(move |app| {
            let places = Places {
                state: app.path().app_data_dir()?.join("state.json"),
                latest: app.path().document_dir()?.join("Aoi").join("aoi.yaml"),
            };
            let (kept, trouble) = match keep::load(&places.state) {
                Ok(kept) => (kept, None),
                Err(e) => (
                    Kept::default(),
                    Some(format!("前回の読み取り結果を読み戻せませんでした: {e}")),
                ),
            };
            app.manage(Latest(Mutex::new(Snapshot::starting(
                &recognizer,
                &kept,
                trouble,
            ))));
            app.manage(recognizer.clone());
            app.manage(places.clone());
            let planner = recognizer.clone();
            let handle = app.handle().clone();
            watch::start(recognizer.clone(), kept, move |status, update| {
                let latest = handle.state::<Latest>();
                let mut snapshot = latest.0.lock().unwrap_or_else(PoisonError::into_inner);
                snapshot.status = status.clone();
                if let Some(update) = update {
                    let read = Some(now());
                    match update {
                        Update::List(List::Items, rows) => {
                            snapshot.items = rows;
                            snapshot.read_at.items = read;
                        }
                        Update::List(List::Equipment, rows) => {
                            snapshot.equipment = rows;
                            snapshot.read_at.equipment = read;
                        }
                        Update::Students(rows) => {
                            snapshot.students = rows;
                            snapshot.read_at.students = read;
                        }
                        Update::Credits(credits) => {
                            snapshot.credits = Some(credits);
                            snapshot.read_at.credits = read;
                        }
                        // Each shop carries when it was read.
                        Update::Shops(rows) => snapshot.shops = rows,
                    }
                    snapshot.replan(&planner);
                    snapshot.keep_error = keep_in(&places, &snapshot)
                        .err()
                        .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
                }
                let _ = handle.emit(EVENT, &*snapshot);
            });
            Ok(())
        })
        // The window shows item icons from what is built in: `icon://<name>`.
        .register_uri_scheme_protocol("icon", |_, request| {
            let name = request.uri().path().trim_start_matches('/');
            let response = tauri::http::Response::builder();
            match crate::icon(name) {
                Some((bytes, media)) => response
                    .header("Content-Type", media)
                    .body(std::borrow::Cow::Borrowed(bytes)),
                None => response
                    .status(404)
                    .body(std::borrow::Cow::Borrowed(&[][..])),
            }
            .expect("a response of a status and a body")
        })
        .invoke_handler(tauri::generate_handler![snapshot, export, set_target])
        .run(context)
        .expect("bluearchive-aoi failed to start");
}
