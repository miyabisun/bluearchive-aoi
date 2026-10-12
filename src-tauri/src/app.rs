//! The window: shows what the watcher sees and the tally.

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    List, Recognizer, Row,
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
        .invoke_handler(tauri::generate_handler![snapshot])
        .run(context)
        .expect("bluearchive-aoi failed to start");
}
