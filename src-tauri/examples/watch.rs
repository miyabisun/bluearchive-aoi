//! Watches the running game the way the app does and prints every status and
//! tally change with the time, for measuring against the real game:
//! `cargo run --release --example watch -- 60` (seconds to watch).

use std::{sync::Arc, time::Instant};

use aoi::{Recognizer, watch};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let recognizer = Arc::new(Recognizer::embedded().unwrap());
    let started = Instant::now();
    watch::start(
        recognizer,
        aoi::keep::Kept::default(),
        move |status, rows| {
            let at = started.elapsed().as_secs_f32();
            match rows {
                Some(aoi::watch::Update::List(list, rows)) => {
                    println!("{at:6.1}s {list:?} rows {}", rows.len());
                }
                Some(aoi::watch::Update::Students(rows)) => {
                    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
                    println!("{at:6.1}s students {}: {}", rows.len(), names.join(" "));
                }
                None => println!("{at:6.1}s {}", serde_json::to_string(status).unwrap()),
            }
        },
    );
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}
