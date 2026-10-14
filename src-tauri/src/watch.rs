//! Watches Blue Archive: captures its window while it runs and reads the item
//! list from the newest frame, dropping frames the reader could not keep up with.
//! The game runs on Windows only; elsewhere there is nothing to capture yet,
//! and what carries the frames lies unused.
#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod window;

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
};

use image::RgbImage;
use serde::Serialize;

use crate::{
    Frame as Shown, List, Recognizer, Row,
    android::{Link, Phone},
    broadcast::{Cast, Receiver},
    keep::{Kept, now},
    quantity::Count,
    screen,
    shop::{Shelves, ShopRow},
    student::{Training, roster::Roster},
    tally::{CONFIRM, Tally},
};

/// What the watcher is doing, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    /// Blue Archive is not running (or shows no window).
    NoGame,
    /// Nothing can be captured where the app runs: the game is Windows'.
    NoSource,
    /// The game shows something other than a list in its default order.
    OtherScreen,
    /// `list` is being read, `fps` frames a second; `dropped` frames arrived
    /// while the reader was busy and were replaced by newer ones.
    Reading {
        list: Screen,
        fps: f32,
        dropped: u32,
    },
    /// The window cannot be captured.
    Failed { message: String },
    /// The user paused the reading.
    Paused,
}

/// What can be read: a list, the student screen or the shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    Items,
    Equipment,
    Students,
    Shops,
}

/// A student seen, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StudentRow {
    pub id: i64,
    pub name: String,
    pub training: Training,
}

/// What changed in a tally, in the link to a phone, or in waiting for a
/// streaming app.
#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    List(List, Vec<Row>),
    Students(Vec<StudentRow>),
    Credits(u64),
    Shops(Vec<ShopRow>),
    Phone(Link),
    Cast(Cast),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Game {
    #[default]
    Absent,
    /// Where the game cannot run (see `start`).
    #[cfg_attr(windows, allow(dead_code))]
    NoSource,
    Failed(String),
    Captured,
}

/// What the two threads know, from which the status follows.
#[derive(Debug, Clone, PartialEq, Default)]
struct Seen {
    game: Game,
    /// What the last frame read showed.
    on_list: Option<Screen>,
    fps: f32,
    dropped: u32,
    /// Whether the user paused the reading.
    paused: bool,
    /// Whether a phone's screen comes in (see `android`), read as the game's
    /// window is.
    phone: bool,
    /// Whether a streaming app's stream comes in (see `broadcast`), read so too.
    cast: bool,
}

fn status(seen: &Seen) -> Status {
    if seen.paused {
        return Status::Paused;
    }
    let game = if seen.phone || seen.cast {
        &Game::Captured
    } else {
        &seen.game
    };
    match game {
        Game::Absent => Status::NoGame,
        Game::NoSource => Status::NoSource,
        Game::Failed(message) => Status::Failed {
            message: message.clone(),
        },
        Game::Captured => match seen.on_list {
            Some(list) => Status::Reading {
                list,
                fps: seen.fps,
                dropped: seen.dropped,
            },
            None => Status::OtherScreen,
        },
    }
}

type Report = dyn Fn(&Status, Option<Update>) + Send + Sync;

/// Keeps what is seen and reports the status whenever it changes.
struct Board {
    seen: Mutex<(Seen, Option<Status>)>,
    report: Box<Report>,
}

impl Board {
    fn update(&self, change: impl FnOnce(&mut Seen)) {
        let mut guard = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut guard.0);
        let now = status(&guard.0);
        if guard.1.as_ref() != Some(&now) {
            (self.report)(&now, None);
            guard.1 = Some(now);
        }
    }

    fn changed(&self, update: Update) {
        let guard = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        (self.report)(&status(&guard.0), Some(update));
    }
}

/// The newest frame, replaced by every new one until the reader takes it.
#[derive(Default)]
struct Latest {
    frame: Mutex<Option<RgbImage>>,
    ready: Condvar,
    /// Frames replaced before the reader took them.
    dropped: AtomicU32,
    /// Whether a phone's screen comes in, by ADB or from a streaming app:
    /// then the game's window is left out, one game read at a time.
    phone: AtomicBool,
    cast: AtomicBool,
}

impl Latest {
    /// Puts a frame of the game's window, unless a phone's screen comes in.
    fn put_window(&self, frame: RgbImage) {
        if !self.phone.load(Ordering::Relaxed) && !self.cast.load(Ordering::Relaxed) {
            self.put(frame);
        }
    }

    fn put(&self, frame: RgbImage) {
        let mut slot = self.frame.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.replace(frame).is_some() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.ready.notify_one();
    }

    fn take(&self) -> RgbImage {
        let mut slot = self.frame.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(frame) = slot.take() {
                return frame;
            }
            slot = self
                .ready
                .wait(slot)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Reports the shops after a frame changed them. A coin that is an item takes
/// the count its shop shows, and the item list with it.
fn shops_changed(recognizer: &Recognizer, shelves: &mut Shelves, items: &mut Tally, board: &Board) {
    let mut taken = false;
    for (shop, coin, count) in shelves.fresh_held() {
        if let (Some(id), Ok(count)) = (recognizer.coin_item(shop, coin), u32::try_from(count)) {
            taken |= items.take(id, Count::Exact(count));
        }
    }
    if taken {
        board.changed(Update::List(
            List::Items,
            recognizer.rows(List::Items, items),
        ));
    }
    board.changed(Update::Shops(
        recognizer.shop_rows(shelves.shelves(), items.counts()),
    ));
}

/// Frames between two readings of the credits (about two a second).
const CREDITS_EVERY: u32 = 15;

/// The credits read, taken once `CONFIRM` readings in a row agree.
struct Credits {
    known: Option<u64>,
    streak: Option<(u64, u8)>,
}

impl Credits {
    /// Take one reading; the credits when they are taken and differ from the
    /// known ones.
    fn observe(&mut self, read: Option<u64>) -> Option<u64> {
        let read = read?;
        let count = match &mut self.streak {
            Some((value, count)) if *value == read => {
                *count = count.saturating_add(1);
                *count
            }
            _ => {
                self.streak = Some((read, 1));
                1
            }
        };
        (count == CONFIRM && self.known != Some(read)).then(|| {
            self.known = Some(read);
            read
        })
    }
}

/// The students `known`, in id order, with their names.
pub fn student_rows(recognizer: &Recognizer, known: &BTreeMap<i64, Training>) -> Vec<StudentRow> {
    known
        .iter()
        .map(|(&id, training)| StudentRow {
            id,
            name: recognizer
                .students()
                .iter()
                .find(|(s, _)| *s == id)
                .map(|(_, name)| name.clone())
                .unwrap_or_default(),
            training: *training,
        })
        .collect()
}

/// Start watching from what was `kept`; `report` hears every change of status,
/// and every change of a tally with the rows to show. A phone's link keeps its
/// key and the phone paired in `phone_dir`; a streaming app is waited for
/// when the user asks.
pub fn start(
    recognizer: Arc<Recognizer>,
    kept: Kept,
    phone_dir: PathBuf,
    report: impl Fn(&Status, Option<Update>) + Send + Sync + 'static,
) -> Watcher {
    let board = Arc::new(Board {
        seen: Mutex::new((Seen::default(), None)),
        report: Box::new(report),
    });
    let latest = Arc::new(Latest::default());
    let frames = Arc::new(AtomicU32::new(0));
    let paused = Arc::new(AtomicBool::new(false));
    let phone = {
        let (latest, windowed, board) = (latest.clone(), latest.clone(), board.clone());
        Arc::new(Phone::new(
            phone_dir,
            paused.clone(),
            move |frame| latest.put(frame),
            move |link| {
                let streaming = matches!(link, Link::Streaming { .. });
                windowed.phone.store(streaming, Ordering::Relaxed);
                board.update(|seen| seen.phone = streaming);
                board.changed(Update::Phone(link.clone()));
            },
        ))
    };
    let watcher = Watcher {
        paused,
        board: board.clone(),
        phone: phone.clone(),
        cast: receiver(&latest, &board),
    };

    // The reader: the newest frame into the tally.
    {
        let (board, latest, frames) = (board.clone(), latest.clone(), frames.clone());
        let paused = watcher.paused.clone();
        thread::spawn(move || {
            let mut tallies = HashMap::from([
                (List::Items, Tally::seeded(kept.items.into_iter().collect())),
                (
                    List::Equipment,
                    Tally::seeded(kept.equipment.into_iter().collect()),
                ),
            ]);
            let mut roster = Roster::seeded(kept.students);
            let mut credits = Credits {
                known: kept.credits,
                streak: None,
            };
            let mut shelves = Shelves::seeded(kept.shops);
            loop {
                let shot = latest.take();
                // Paused, frames are taken and dropped unread.
                if paused.load(Ordering::Relaxed) {
                    continue;
                }
                // A phone's frame as the game lays it out (a window's as it is).
                let shot = screen::normalize(&shot);
                let read = recognizer.read_frame(&shot);
                // The credits change seldom.
                if frames.fetch_add(1, Ordering::Relaxed) % CREDITS_EVERY == 0
                    && let Some(changed) = credits.observe(recognizer.read_credits(&shot))
                {
                    board.changed(Update::Credits(changed));
                }
                board.update(|seen| {
                    seen.on_list = read.as_ref().map(|frame| match frame {
                        Shown::List(List::Items, ..) => Screen::Items,
                        Shown::List(List::Equipment, ..) => Screen::Equipment,
                        Shown::Student(..) => Screen::Students,
                        Shown::Shop(_) => Screen::Shops,
                    });
                });
                match read {
                    Some(Shown::List(list, cells, start)) => {
                        let tally = tallies.entry(list).or_default();
                        let gone = recognizer.catalog(list).gaps(&cells, start);
                        if tally.observe(&cells, &gone) {
                            board.changed(Update::List(list, recognizer.rows(list, tally)));
                            // The shops show the coins held as the item list counts them.
                            if list == List::Items {
                                let rows = recognizer.shop_rows(shelves.shelves(), tally.counts());
                                board.changed(Update::Shops(rows));
                            }
                        }
                    }
                    Some(Shown::Student(Some(id), training)) if roster.observe(id, training) => {
                        let rows = student_rows(&recognizer, roster.students());
                        board.changed(Update::Students(rows));
                    }
                    Some(Shown::Shop(seen)) if shelves.observe(&seen, &now()) => {
                        let items = tallies.entry(List::Items).or_default();
                        shops_changed(&recognizer, &mut shelves, items, &board);
                    }
                    _ => {}
                }
            }
        });
    }

    phone.start();
    // The watcher: the game's window into a capture, and the frame rate.
    #[cfg(windows)]
    thread::spawn(move || window::look(&board, &latest, &frames));
    #[cfg(not(windows))]
    board.update(|seen| seen.game = Game::NoSource);
    watcher
}

/// Waits for a streaming app, its frames read as a phone's.
fn receiver(latest: &Arc<Latest>, board: &Arc<Board>) -> Receiver {
    let (latest, windowed, board) = (latest.clone(), latest.clone(), board.clone());
    Receiver::new(
        move |frame| latest.put(frame),
        move |cast| {
            let receiving = matches!(cast, Cast::Receiving { .. });
            windowed.cast.store(receiving, Ordering::Relaxed);
            board.update(|seen| seen.cast = receiving);
            board.changed(Update::Cast(cast.clone()));
        },
    )
}

/// The watcher as the window controls it: its reading paused or not, the
/// link to a phone, and waiting for a streaming app.
pub struct Watcher {
    paused: Arc<AtomicBool>,
    board: Arc<Board>,
    phone: Arc<Phone>,
    cast: Receiver,
}

impl Watcher {
    /// Pauses the reading, frames dropped unread, or resumes it.
    pub fn pause(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
        self.board.update(|seen| seen.paused = paused);
    }

    /// The link to a phone (see `android::Phone`).
    pub fn phone(&self) -> &Phone {
        &self.phone
    }

    /// Waiting for a streaming app (see `broadcast::Receiver`).
    pub fn cast(&self) -> &Receiver {
        &self.cast
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, Screen, Seen, Status, status};

    /// One game is read at a time: the phone's while its screen comes in, by
    /// ADB or from a streaming app.
    #[test]
    fn the_window_waits_while_a_phone_streams() {
        for cast in [false, true] {
            let latest = super::Latest::default();
            let source = if cast { &latest.cast } else { &latest.phone };
            source.store(true, std::sync::atomic::Ordering::Relaxed);
            latest.put_window(image::RgbImage::new(2, 1));
            latest.put(image::RgbImage::new(3, 1));
            assert_eq!(latest.take().width(), 3);
            source.store(false, std::sync::atomic::Ordering::Relaxed);
            latest.put_window(image::RgbImage::new(2, 1));
            assert_eq!(latest.take().width(), 2);
        }
    }

    #[test]
    fn the_credits_are_taken_once_readings_agree_and_change() {
        let mut credits = super::Credits {
            known: Some(100),
            streak: None,
        };
        let reads = [Some(250), None, Some(250), Some(250), Some(250), Some(250)];
        let taken: Vec<Option<u64>> = reads.into_iter().map(|r| credits.observe(r)).collect();
        assert_eq!(taken, [None, None, None, Some(250), None, None]);
        let again: Vec<Option<u64>> = [Some(300), Some(250), Some(250), Some(250)]
            .into_iter()
            .map(|r| credits.observe(r))
            .collect();
        assert_eq!(again, [None, None, None, None]);
    }

    #[test]
    fn the_status_follows_the_game_and_the_screen() {
        let seen = |game, on_list| Seen {
            game,
            on_list,
            fps: 28.5,
            dropped: 4,
            paused: false,
            phone: false,
            cast: false,
        };
        let items = Some(Screen::Items);
        assert_eq!(status(&seen(Game::Absent, items)), Status::NoGame);
        assert_eq!(status(&seen(Game::NoSource, None)), Status::NoSource);
        assert_eq!(
            status(&seen(Game::Failed("denied".into()), None)),
            Status::Failed {
                message: "denied".into()
            }
        );
        assert_eq!(status(&seen(Game::Captured, None)), Status::OtherScreen);
        assert_eq!(
            status(&seen(Game::Captured, Some(Screen::Students))),
            Status::Reading {
                list: Screen::Students,
                fps: 28.5,
                dropped: 4
            }
        );
        // A phone's screen coming in reads without the game's window.
        for game in [Game::Absent, Game::NoSource] {
            for phone in [
                Seen {
                    phone: true,
                    ..seen(game.clone(), Some(Screen::Shops))
                },
                Seen {
                    cast: true,
                    ..seen(game.clone(), Some(Screen::Shops))
                },
            ] {
                assert!(matches!(
                    status(&phone),
                    Status::Reading {
                        list: Screen::Shops,
                        ..
                    }
                ));
            }
        }
        // Paused, whatever the game shows.
        for game in [Game::Captured, Game::Absent] {
            let paused = Seen {
                paused: true,
                ..seen(game, Some(Screen::Items))
            };
            assert_eq!(status(&paused), Status::Paused);
        }
    }
}
