//! The window: shows what the watcher sees and the tally.

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    List, Recognizer, Row, export,
    watch::{self, Status, StudentRow, Update},
};

/// Everything the window shows; sent whole on every change.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    status: Status,
    items: Vec<Row>,
    equipment: Vec<Row>,
    students: Vec<StudentRow>,
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
    let now = chrono::Local::now();
    let dir = app
        .path()
        .document_dir()
        .map_err(|e| e.to_string())?
        .join("Aoi");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("aoi-{}.yaml", now.format("%Y%m%d-%H%M%S")));
    let text = export::yaml(
        &now.format("%Y-%m-%d %H:%M:%S").to_string(),
        &snapshot.students,
        &snapshot.items,
        &snapshot.equipment,
    );
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

pub fn run(context: tauri::Context) {
    let recognizer = Arc::new(Recognizer::embedded().expect("the built-in data is broken"));
    tauri::Builder::default()
        .manage(Latest(Mutex::new(Snapshot {
            status: Status::NoGame,
            items: Vec::new(),
            equipment: Vec::new(),
            students: Vec::new(),
        })))
        .setup(move |app| {
            let handle = app.handle().clone();
            watch::start(recognizer, move |status, rows| {
                let latest = handle.state::<Latest>();
                let mut snapshot = latest.0.lock().unwrap_or_else(PoisonError::into_inner);
                snapshot.status = status.clone();
                match rows {
                    Some(Update::List(List::Items, rows)) => snapshot.items = rows,
                    Some(Update::List(List::Equipment, rows)) => snapshot.equipment = rows,
                    Some(Update::Students(rows)) => snapshot.students = rows,
                    None => {}
                }
                let _ = handle.emit(EVENT, &*snapshot);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![snapshot, export])
        .run(context)
        .expect("bluearchive-aoi failed to start");
}
