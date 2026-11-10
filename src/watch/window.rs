//! Capturing the game's window, on Windows, where Blue Archive runs.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use image::RgbImage;
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

use super::{Board, Game, Latest, Sight};

const GAME: &str = "BlueArchive.exe";
/// How often the game's window is looked for, and the frame rate measured.
const LOOK_INTERVAL: Duration = Duration::from_secs(1);

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
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = (Arc<Latest>, isize);
    type Error = CaptureError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let (latest, window) = ctx.flags;
        Ok(Self { latest, window })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Frames come faster than they are read (Windows before 24H2 ignores the minimum
        // update interval, and a screen not read is looked at slower): one not due is
        // left before its copy is made.
        if !self.latest.window_due() {
            return Ok(());
        }
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
        let rgb = super::client_rgb(pixels, width, (left, top, w, h));
        if let Some(image) = RgbImage::from_raw(w, h, rgb) {
            self.latest.put_window(image);
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
        MinimumUpdateIntervalSettings::Custom(super::READING),
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        (latest, window.as_raw_hwnd() as isize),
    );
    Capture::start_free_threaded(settings).map_err(|err| err.to_string())
}

/// Keeps Windows from running the app at efficiency speeds (`EcoQoS`) while its window is not in
/// front: the frames then take longer to read, and more of the CPU's time. On 2026-11-07 the
/// GPUI window (v0.4.0) was slowed so, reading the same frames at 178% of a core against 147%.
fn run_at_full_speed() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
        ProcessPowerThrottling, SetProcessInformation,
    };
    // Controlled (the mask), and off (the state).
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: 0,
    };
    #[allow(clippy::cast_possible_truncation)] // A dozen bytes.
    let size = size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32;
    // SAFETY: the state outlives the call, which reads `size` bytes of it.
    let ok = unsafe {
        SetProcessInformation(
            GetCurrentProcess(),
            ProcessPowerThrottling,
            (&raw const state).cast(),
            size,
        )
    };
    if ok == 0 {
        eprintln!(
            "Windows may slow the reading down: {}",
            std::io::Error::last_os_error()
        );
    }
}

/// Looks for the game's window every `LOOK_INTERVAL`, captures it into
/// `latest`, and tells the frame rate of the `frames` read.
pub(super) fn look(board: &Board, latest: &Arc<Latest>, frames: &AtomicU32) {
    run_at_full_speed();
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
            // The phone's screen and the pause outlast the game's window.
            board.update(|seen| {
                seen.game = Game::Absent;
                seen.sight = Sight::Other;
            });
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
}

#[cfg(test)]
mod tests {
    use super::is_game;

    #[test]
    fn the_game_is_known_by_its_exe_name() {
        assert!(is_game(r"D:\Games\BlueArchive\BlueArchive.exe"));
        assert!(is_game(r"C:\Nexon\bluearchive.EXE"));
        assert!(!is_game(r"C:\Tools\BlueArchive.exe.bak"));
        assert!(!is_game(r"C:\Tools\NotBlueArchive.exe"));
    }
}
