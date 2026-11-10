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
    time::{Duration, Instant},
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
    /// The game is drawn too small to read (see `screen::too_small`).
    TooSmall,
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

/// Where the game is, for the window to follow (its work tab opens what the
/// game shows): a shop by its name, or a screen by its title (see `title`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum Place {
    Shop(&'static str),
    Screen(&'static str),
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
    /// Where the game is now, or nowhere the window follows.
    Place(Option<Place>),
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

/// What the last frame read showed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Sight {
    #[default]
    Other,
    /// The game drawn too small to read (see `screen::too_small`).
    TooSmall,
    Screen(Screen),
}

/// What the two threads know, from which the status follows.
#[derive(Debug, Clone, PartialEq, Default)]
struct Seen {
    game: Game,
    sight: Sight,
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
        Game::Captured => match seen.sight {
            Sight::Screen(list) => Status::Reading {
                list,
                fps: seen.fps,
                dropped: seen.dropped,
            },
            Sight::TooSmall => Status::TooSmall,
            Sight::Other => Status::OtherScreen,
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

/// Frames are taken at most this often while the game shows something read (30 a second).
pub(crate) const READING: Duration = Duration::from_millis(33);
/// And this often while it shows nothing read, or the reading is paused: the screens between
/// are only looked at for one that reads. Measured on 2026-11-10 on a screen not read, a
/// window 2370 by 1333 at 30 frames a second: 51% of a core, each frame 3 ms to scale and 3
/// to 4 to look at, and its copy out of the capture before that.
pub(crate) const RESTING: Duration = Duration::from_millis(150);

/// When the next frame is due: frames come faster than they are worth reading.
#[derive(Debug, Default)]
struct Pace {
    /// When the last frame was taken.
    last: Option<Instant>,
    /// Whether the last frame read showed nothing read (or the reading is paused).
    resting: bool,
}

impl Pace {
    /// Whether a frame come at `now` is taken: one far enough after the last taken.
    fn due(&mut self, now: Instant) -> bool {
        let apart = if self.resting { RESTING } else { READING };
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < apart)
        {
            return false;
        }
        self.last = Some(now);
        true
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
    pace: Mutex<Pace>,
}

impl Latest {
    /// Whether a frame come now is taken (see `Pace`): asked before the work of making it.
    fn due(&self) -> bool {
        self.pace
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .due(Instant::now())
    }

    /// Whether a frame of the game's window come now is taken: none while a phone's
    /// screen comes in.
    fn window_due(&self) -> bool {
        !self.phone.load(Ordering::Relaxed) && !self.cast.load(Ordering::Relaxed) && self.due()
    }

    /// Frames are taken slower from now (`resting`), or as fast as they are read.
    fn rest(&self, resting: bool) {
        self.pace
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .resting = resting;
    }

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

/// The credits change seldom: they are read this far apart.
const CREDITS_APART: Duration = Duration::from_millis(500);

/// Whether the credits are read from a frame come at `now`, `last` the time they were.
fn credits_due(last: &mut Option<Instant>, now: Instant) -> bool {
    if last.is_some_and(|last| now.duration_since(last) < CREDITS_APART) {
        return false;
    }
    *last = Some(now);
    true
}

/// What the frames show, taken once `CONFIRM` readings in a row agree: a
/// frame read otherwise now and then changes nothing.
struct Steady<T> {
    known: T,
    streak: Option<(T, u8)>,
}

impl<T: Clone + PartialEq> Steady<T> {
    fn new(known: T) -> Self {
        Self {
            known,
            streak: None,
        }
    }

    /// Take one reading; what is taken when it differs from the known.
    fn observe(&mut self, read: T) -> Option<T> {
        let count = match &mut self.streak {
            Some((value, count)) if *value == read => {
                *count = count.saturating_add(1);
                *count
            }
            _ => {
                self.streak = Some((read.clone(), 1));
                1
            }
        };
        (count == CONFIRM && self.known != read).then(|| {
            self.known = read.clone();
            read
        })
    }
}

/// The credits read, taken as `Steady`; a frame they read nothing from does
/// not part the readings.
struct Credits(Steady<Option<u64>>);

impl Credits {
    /// Take one reading; the credits when they are taken and differ from the
    /// known ones.
    fn observe(&mut self, read: Option<u64>) -> Option<u64> {
        self.0.observe(Some(read?)).flatten()
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
            move |frame| {
                if latest.due() {
                    latest.put(frame);
                }
            },
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
            let mut read = Read::seeded(kept);
            loop {
                let shot = latest.take();
                // Paused, frames are taken and dropped unread.
                let reads = !paused.load(Ordering::Relaxed)
                    && read.frame(&recognizer, &board, &frames, &shot);
                latest.rest(!reads);
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
        move |frame| {
            if latest.due() {
                latest.put(frame);
            }
        },
        move |cast| {
            let receiving = matches!(cast, Cast::Receiving { .. });
            windowed.cast.store(receiving, Ordering::Relaxed);
            board.update(|seen| seen.cast = receiving);
            board.changed(Update::Cast(cast.clone()));
        },
    )
}

/// What the reader has read so far.
struct Read {
    tallies: HashMap<List, Tally>,
    roster: Roster,
    credits: Credits,
    /// When the credits were last read.
    credits_at: Option<Instant>,
    shelves: Shelves,
    place: Steady<Option<Place>>,
}

impl Read {
    fn seeded(kept: Kept) -> Self {
        Self {
            tallies: HashMap::from([
                (List::Items, Tally::seeded(kept.items.into_iter().collect())),
                (
                    List::Equipment,
                    Tally::seeded(kept.equipment.into_iter().collect()),
                ),
            ]),
            roster: Roster::seeded(kept.students),
            credits: Credits(Steady::new(kept.credits)),
            credits_at: None,
            place: Steady::new(None),
            shelves: Shelves::seeded(kept.shops),
        }
    }

    /// Reads `shot` into what is read, telling `board` what it shows and
    /// what changed; `frames` counts the frames read. Whether it showed
    /// something read.
    fn frame(
        &mut self,
        recognizer: &Recognizer,
        board: &Board,
        frames: &AtomicU32,
        shot: &RgbImage,
    ) -> bool {
        if screen::too_small(shot) {
            board.update(|seen| seen.sight = Sight::TooSmall);
            return false;
        }
        // A phone's frame as the game lays it out (a window's as it is).
        let shot = screen::normalize(shot);
        let read = recognizer.read_frame(&shot);
        let reads = read.is_some();
        frames.fetch_add(1, Ordering::Relaxed);
        if credits_due(&mut self.credits_at, Instant::now())
            && let Some(changed) = self.credits.observe(recognizer.read_credits(&shot))
        {
            board.changed(Update::Credits(changed));
        }
        let place = match &read {
            Some(Shown::Shop(seen)) => Some(Place::Shop(seen.shop)),
            Some(_) => None,
            None => recognizer.read_title(&shot).map(Place::Screen),
        };
        if let Some(place) = self.place.observe(place) {
            board.changed(Update::Place(place));
        }
        board.update(|seen| {
            seen.sight = read.as_ref().map_or(Sight::Other, |frame| {
                Sight::Screen(match frame {
                    Shown::List(List::Items, ..) => Screen::Items,
                    Shown::List(List::Equipment, ..) => Screen::Equipment,
                    Shown::Student(..) => Screen::Students,
                    Shown::Shop(_) => Screen::Shops,
                })
            });
        });
        match read {
            Some(Shown::List(list, cells, start)) => {
                let tally = self.tallies.entry(list).or_default();
                let gone = recognizer
                    .catalog(list)
                    .gaps(&cells, start, |id| tally.counts().contains_key(&id));
                if tally.observe(&cells, &gone) {
                    board.changed(Update::List(list, recognizer.rows(list, tally)));
                    // The shops show the coins held as the item list counts them.
                    if list == List::Items {
                        let rows = recognizer.shop_rows(self.shelves.shelves(), tally.counts());
                        board.changed(Update::Shops(rows));
                    }
                }
            }
            Some(Shown::Student(Some(id), training)) if self.roster.observe(id, training) => {
                let rows = student_rows(recognizer, self.roster.students());
                board.changed(Update::Students(rows));
            }
            Some(Shown::Shop(seen)) if self.shelves.observe(&seen, &now()) => {
                let items = self.tallies.entry(List::Items).or_default();
                shops_changed(recognizer, &mut self.shelves, items, board);
            }
            _ => {}
        }
        reads
    }
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

/// The client area `(left, top, width, height)` of a window's frame, `frame_width` pixels wide in
/// RGBA, as RGB rows. Sized up front and copied row by row: collected from per-byte iterators it
/// took 22 to 28 ms a frame of 2372×1372 (2026-11-07), most of the capture thread's time.
pub fn client_rgb(
    frame: &[u8],
    frame_width: u32,
    (left, top, width, height): (u32, u32, u32, u32),
) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for y in top..top + height {
        let row = (y * frame_width + left) as usize * 4;
        for pixel in frame[row..row + width as usize * 4].chunks_exact(4) {
            rgb.extend_from_slice(&pixel[..3]);
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::{Game, Place, Screen, Seen, Sight, Status, Steady, client_rgb, status};

    /// The window's frame, 4×3 here, gives the client area (the 2×2 from (1, 1)) as RGB rows.
    #[test]
    fn the_client_area_comes_out_as_rgb_rows() {
        let pixel = |i: u8| [i, 100 + i, 200 - i, 255];
        let frame: Vec<u8> = (0..12).flat_map(pixel).collect();
        let rgb = client_rgb(&frame, 4, (1, 1, 2, 2));
        let wanted: Vec<u8> = [5, 6, 9, 10]
            .into_iter()
            .flat_map(|i| pixel(i)[..3].to_vec())
            .collect();
        assert_eq!(rgb, wanted, "the client area's pixels, alpha dropped");
    }

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
        let mut credits = super::Credits(Steady::new(Some(100)));
        let reads = [Some(250), None, Some(250), Some(250), Some(250), Some(250)];
        let taken: Vec<Option<u64>> = reads.into_iter().map(|r| credits.observe(r)).collect();
        assert_eq!(taken, [None, None, None, Some(250), None, None]);
        let again: Vec<Option<u64>> = [Some(300), Some(250), Some(250), Some(250)]
            .into_iter()
            .map(|r| credits.observe(r))
            .collect();
        assert_eq!(again, [None, None, None, None]);
    }

    /// Where the game is changes once frames agree on it, a shop or a screen
    /// or nowhere followed; a frame read otherwise now and then is no change.
    #[test]
    fn the_place_is_taken_once_readings_agree_and_change() {
        let (shop, title) = (
            Some(Place::Shop("総力戦")),
            Some(Place::Screen("学園交流会")),
        );
        let mut place = Steady::new(None);
        let mut taken = |reads: &[Option<Place>]| -> Vec<Option<Option<Place>>> {
            reads.iter().map(|&r| place.observe(r)).collect()
        };
        assert_eq!(
            taken(&[shop, shop, shop, shop]),
            [None, None, Some(shop), None]
        );
        assert_eq!(taken(&[None, shop, shop, shop]), [None, None, None, None]);
        assert_eq!(taken(&[None, None, None]), [None, None, Some(None)]);
        assert_eq!(taken(&[title, title, title]), [None, None, Some(title)]);
    }

    #[test]
    fn the_status_follows_the_game_and_the_screen() {
        let seen = |game, sight| Seen {
            game,
            sight,
            fps: 28.5,
            dropped: 4,
            paused: false,
            phone: false,
            cast: false,
        };
        let items = Sight::Screen(Screen::Items);
        assert_eq!(status(&seen(Game::Absent, items)), Status::NoGame);
        assert_eq!(
            status(&seen(Game::NoSource, Sight::Other)),
            Status::NoSource
        );
        assert_eq!(
            status(&seen(Game::Failed("denied".into()), Sight::Other)),
            Status::Failed {
                message: "denied".into()
            }
        );
        assert_eq!(
            status(&seen(Game::Captured, Sight::Other)),
            Status::OtherScreen
        );
        assert_eq!(
            status(&seen(Game::Captured, Sight::Screen(Screen::Students))),
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
                    ..seen(game.clone(), Sight::Screen(Screen::Shops))
                },
                Seen {
                    cast: true,
                    ..seen(game.clone(), Sight::Screen(Screen::Shops))
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
        // A screen too small to read is said so, whatever it shows.
        assert_eq!(
            status(&seen(Game::Captured, Sight::TooSmall)),
            Status::TooSmall
        );
        // Paused, whatever the game shows.
        for game in [Game::Captured, Game::Absent] {
            let paused = Seen {
                paused: true,
                ..seen(game, items)
            };
            assert_eq!(status(&paused), Status::Paused);
        }
    }

    /// Frames come every 10 ms: while the game shows something read, one is taken every 33 ms
    /// or so; while it shows nothing read, one every 150 ms.
    #[test]
    fn frames_are_taken_slower_while_nothing_reads() {
        let start = std::time::Instant::now();
        let taken = |resting: bool| -> Vec<u64> {
            let mut pace = super::Pace {
                last: None,
                resting,
            };
            (0..40)
                .map(|n| n * 10)
                .filter(|&ms| pace.due(start + std::time::Duration::from_millis(ms)))
                .collect()
        };
        assert_eq!(taken(false), [0, 40, 80, 120, 160, 200, 240, 280, 320, 360]);
        assert_eq!(taken(true), [0, 150, 300]);
    }

    /// The credits are read every half a second, however fast the frames come.
    #[test]
    fn the_credits_are_read_by_the_clock() {
        let start = std::time::Instant::now();
        let mut last = None;
        let read: Vec<u64> = [0, 33, 200, 499, 500, 650, 1000, 1001]
            .into_iter()
            .filter(|&ms| {
                super::credits_due(&mut last, start + std::time::Duration::from_millis(ms))
            })
            .collect();
        assert_eq!(read, [0, 500, 1000]);
    }
}
