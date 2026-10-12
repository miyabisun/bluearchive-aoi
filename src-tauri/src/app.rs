//! The window: shows what the watcher sees and the tally, and keeps it.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    List, Recognizer, Row, export,
    keep::{self, Kept, ReadAt},
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
    read_at: ReadAt,
    /// Why what was read could not be kept or brought back, while it lasts.
    keep_error: Option<String>,
}

/// Where what is read is kept: `state.json` in the app's folder, brought back
/// on the next run, and `Documents\Aoi\aoi.yaml`, the latest of it to read.
struct Places {
    state: PathBuf,
    latest: PathBuf,
}

/// The local time, as the kept data and the exports write it.
fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Snapshot {
    /// The window as it starts: what was kept, before anything is read.
    fn starting(recognizer: &Recognizer, kept: &Kept, keep_error: Option<String>) -> Self {
        let rows = |list, counts: &std::collections::BTreeMap<i64, crate::quantity::Count>| {
            recognizer.rows(list, &Tally::seeded(counts.clone().into_iter().collect()))
        };
        Self {
            status: Status::NoGame,
            items: rows(List::Items, &kept.items),
            equipment: rows(List::Equipment, &kept.equipment),
            students: watch::student_rows(recognizer, &kept.students),
            read_at: kept.read_at.clone(),
            keep_error,
        }
    }

    /// What is kept of the window.
    fn kept(&self) -> Kept {
        let counts = |rows: &[Row]| rows.iter().map(|r| (r.id, r.count)).collect();
        Kept {
            items: counts(&self.items),
            equipment: counts(&self.equipment),
            students: self.students.iter().map(|s| (s.id, s.training)).collect(),
            read_at: self.read_at.clone(),
        }
    }

    /// Everything read, as YAML headed by `stamp`.
    fn yaml(&self, stamp: &str) -> String {
        export::yaml(
            stamp,
            &self.read_at,
            &self.students,
            &self.items,
            &self.equipment,
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
                    }
                    snapshot.keep_error = keep_in(&places, &snapshot)
                        .err()
                        .map(|e| format!("読み取り結果を保存できませんでした: {e}"));
                }
                let _ = handle.emit(EVENT, &*snapshot);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![snapshot, export])
        .run(context)
        .expect("bluearchive-aoi failed to start");
}
