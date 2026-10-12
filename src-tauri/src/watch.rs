//! Watches Blue Archive: captures its window while it runs and reads the item
//! list from the newest frame, dropping frames the reader could not keep up with.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicU32, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use image::RgbImage;
use serde::Serialize;
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
    window::Window,
};

use crate::{
    Frame as Shown, List, Recognizer, Row,
    keep::Kept,
    student::{Training, roster::Roster},
    tally::Tally,
};

const GAME: &str = "BlueArchive.exe";
/// Frames are taken at most this often (30 a second).
const FRAME_INTERVAL: Duration = Duration::from_millis(33);
/// How often the game's window is looked for, and the frame rate measured.
const LOOK_INTERVAL: Duration = Duration::from_secs(1);

/// What the watcher is doing, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    /// Blue Archive is not running (or shows no window).
    NoGame,
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
}

/// What can be read: a list, or the student screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    Items,
    Equipment,
    Students,
}

/// A student seen, for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StudentRow {
    pub id: i64,
    pub name: String,
    pub training: Training,
}

/// What changed in a tally.
#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    List(List, Vec<Row>),
    Students(Vec<StudentRow>),
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Game {
    #[default]
    Absent,
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
}

fn status(seen: &Seen) -> Status {
    match &seen.game {
        Game::Absent => Status::NoGame,
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
}

impl Latest {
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

type CaptureError = Box<dyn std::error::Error + Send + Sync>;

/// Where a window's client area lies in the capture's frames, in physical
/// pixels whatever DPI awareness the process runs with: left, top, width,
/// height. A frame holds the window's visible frame (DWM's extended frame
/// bounds): the title bar above the client area and a border around it.
fn client_area(window: isize) -> Option<(u32, u32, u32, u32)> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::{
            Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
            Gdi::ClientToScreen,
        },
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
            WindowsAndMessaging::GetClientRect,
        },
    };
    let empty = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let (mut client, mut frame, mut origin) = (empty, empty, POINT { x: 0, y: 0 });
    // SAFETY: the rects and the point outlive the calls; the thread's
    // awareness is restored.
    let ok = unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let ok = GetClientRect(window as _, &raw mut client) != 0
            && ClientToScreen(window as _, &raw mut origin) != 0
            && DwmGetWindowAttribute(
                window as _,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&raw mut frame).cast(),
                size_of::<RECT>() as u32,
            ) == 0;
        SetThreadDpiAwarenessContext(previous);
        ok
    };
    let offset = |a: i32, b: i32| u32::try_from(a - b).ok();
    if !ok {
        return None;
    }
    Some((
        offset(origin.x, frame.left)?,
        offset(origin.y, frame.top)?,
        offset(client.right, client.left)?,
        offset(client.bottom, client.top)?,
    ))
}

struct Capture {
    latest: Arc<Latest>,
    /// The game's window handle.
    window: isize,
    last: Instant,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = (Arc<Latest>, isize);
    type Error = CaptureError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let (latest, window) = ctx.flags;
        Ok(Self {
            latest,
            window,
            last: Instant::now(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Windows before 24H2 ignores the minimum update interval.
        if self.last.elapsed() < FRAME_INTERVAL {
            return Ok(());
        }
        self.last = Instant::now();
        // The frame holds the title bar and a border around the client area.
        // windows-capture's own `buffer_without_title_bar` miscounts the bar in
        // a DPI-aware process (the app) and keeps the border, which moves every
        // area read by a pixel.
        let mut buffer = frame.buffer()?;
        let (width, height) = (buffer.width(), buffer.height());
        let (left, top, w, h) = client_area(self.window)
            .filter(|&(l, t, w, h)| l + w <= width && t + h <= height)
            .unwrap_or((0, 0, width, height));
        let pixels = buffer.as_nopadding_buffer()?;
        let rgb: Vec<u8> = (top..top + h)
            .flat_map(|y| {
                let row = (y * width + left) as usize * 4;
                pixels[row..row + w as usize * 4]
                    .chunks_exact(4)
                    .flat_map(|p| [p[0], p[1], p[2]])
            })
            .collect();
        if let Some(image) = RgbImage::from_raw(w, h, rgb) {
            self.latest.put(image);
        }
        Ok(())
    }
}

/// Whether a process's exe path is the game's.
fn is_game(path: &str) -> bool {
    path.rsplit('\\')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(GAME))
}

/// The path of a process's exe. The game runs as administrator, which denies
/// `Window::process_name`; the limited query is allowed.
fn process_path(pid: u32) -> Option<String> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    };
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    // SAFETY: the handle is checked and closed; the buffer outlives the call and
    // `len` holds its size.
    let ok = unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &raw mut len,
        );
        CloseHandle(process);
        ok
    };
    (ok != 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]))
}

fn game_window() -> Option<Window> {
    Window::enumerate().ok()?.into_iter().find(|window| {
        window
            .process_id()
            .ok()
            .and_then(process_path)
            .is_some_and(|path| is_game(&path))
            && window.title().is_ok_and(|title| !title.is_empty())
    })
}

fn capture(
    window: Window,
    latest: Arc<Latest>,
) -> Result<CaptureControl<Capture, CaptureError>, String> {
    let settings = Settings::new(
        window,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Custom(FRAME_INTERVAL),
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (latest, window.as_raw_hwnd() as isize),
    );
    Capture::start_free_threaded(settings).map_err(|err| err.to_string())
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
/// and every change of a tally with the rows to show.
pub fn start(
    recognizer: Arc<Recognizer>,
    kept: Kept,
    report: impl Fn(&Status, Option<Update>) + Send + Sync + 'static,
) {
    let board = Arc::new(Board {
        seen: Mutex::new((Seen::default(), None)),
        report: Box::new(report),
    });
    let latest = Arc::new(Latest::default());
    let frames = Arc::new(AtomicU32::new(0));

    // The reader: the newest frame into the tally.
    {
        let (board, latest, frames) = (board.clone(), latest.clone(), frames.clone());
        thread::spawn(move || {
            let mut tallies = HashMap::from([
                (List::Items, Tally::seeded(kept.items.into_iter().collect())),
                (
                    List::Equipment,
                    Tally::seeded(kept.equipment.into_iter().collect()),
                ),
            ]);
            let mut roster = Roster::seeded(kept.students);
            loop {
                let read = recognizer.read_frame(&latest.take());
                frames.fetch_add(1, Ordering::Relaxed);
                board.update(|seen| {
                    seen.on_list = read.as_ref().map(|frame| match frame {
                        Shown::List(List::Items, _) => Screen::Items,
                        Shown::List(List::Equipment, _) => Screen::Equipment,
                        Shown::Student(..) => Screen::Students,
                    });
                });
                match read {
                    Some(Shown::List(list, cells)) => {
                        let tally = tallies.entry(list).or_default();
                        if tally.observe(&cells) {
                            board.changed(Update::List(list, recognizer.rows(list, tally)));
                        }
                    }
                    Some(Shown::Student(Some(id), training)) if roster.observe(id, training) => {
                        let rows = student_rows(&recognizer, roster.students());
                        board.changed(Update::Students(rows));
                    }
                    _ => {}
                }
            }
        });
    }

    // The watcher: the game's window into a capture, and the frame rate.
    thread::spawn(move || {
        let mut session: Option<(Window, CaptureControl<Capture, CaptureError>)> = None;
        let mut since = Instant::now();
        board.update(|_| {});
        loop {
            thread::sleep(LOOK_INTERVAL);
            if session
                .as_ref()
                .is_some_and(|(window, control)| !window.is_valid() || control.is_finished())
            {
                if let Some((_, control)) = session.take() {
                    let _ = control.stop();
                }
                board.update(|seen| *seen = Seen::default());
            }
            if session.is_none()
                && let Some(window) = game_window()
            {
                match capture(window, latest.clone()) {
                    Ok(control) => {
                        session = Some((window, control));
                        latest.dropped.store(0, Ordering::Relaxed);
                        board.update(|seen| seen.game = Game::Captured);
                    }
                    Err(message) => board.update(|seen| seen.game = Game::Failed(message)),
                }
            }
            let fps = frames.swap(0, Ordering::Relaxed) as f32 / since.elapsed().as_secs_f32();
            since = Instant::now();
            board.update(|seen| {
                seen.fps = (fps * 10.0).round() / 10.0;
                seen.dropped = latest.dropped.load(Ordering::Relaxed);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{Game, Screen, Seen, Status, is_game, status};

    #[test]
    fn the_game_is_known_by_its_exe_name() {
        assert!(is_game(r"D:\Games\BlueArchive\BlueArchive.exe"));
        assert!(is_game(r"C:\Nexon\bluearchive.EXE"));
        assert!(!is_game(r"C:\Tools\BlueArchive.exe.bak"));
        assert!(!is_game(r"C:\Tools\NotBlueArchive.exe"));
    }

    #[test]
    fn the_status_follows_the_game_and_the_screen() {
        let seen = |game, on_list| Seen {
            game,
            on_list,
            fps: 28.5,
            dropped: 4,
        };
        let items = Some(Screen::Items);
        assert_eq!(status(&seen(Game::Absent, items)), Status::NoGame);
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
    }
}
