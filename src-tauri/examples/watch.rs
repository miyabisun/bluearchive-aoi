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
        // No phone is paired in a folder of its own.
        std::env::temp_dir().join("aoi-watch"),
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
                Some(aoi::watch::Update::Credits(credits)) => {
                    println!("{at:6.1}s credits {credits}");
                }
                Some(aoi::watch::Update::Shops(shops)) => {
                    let sizes: Vec<String> = shops
                        .iter()
                        .map(|s| format!("{} {}", s.name, s.goods.len()))
                        .collect();
                    println!("{at:6.1}s shops {}", sizes.join(" / "));
                }
                Some(aoi::watch::Update::Phone(link)) => {
                    println!("{at:6.1}s phone {}", serde_json::to_string(&link).unwrap());
                }
                None => println!("{at:6.1}s {}", serde_json::to_string(status).unwrap()),
            }
        },
    );
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}
