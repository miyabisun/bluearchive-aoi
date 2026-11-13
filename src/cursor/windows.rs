//! Putting the highlighted arrow in for Windows' arrow and for the arrow it shows while
//! a program starts, and Windows' own back.

use std::{
    ffi::c_void,
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint},
    System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::{
            GetCursorPos, HCURSOR, IMAGE_CURSOR, LR_LOADFROMFILE, LoadImageW, OCR_APPSTARTING,
            OCR_NORMAL, SPI_SETCURSORS, SetSystemCursor, SystemParametersInfoW,
        },
    },
};

use super::Palette;

/// How often the monitor the cursor is on is looked at, for the arrow's size there.
const POLL: Duration = Duration::from_millis(150);

/// The arrow chosen, and the one Windows shows: its size and palette.
#[derive(Default)]
struct Shown {
    wanted: Option<Palette>,
    drawn: Option<(u32, Palette)>,
}

/// Keeps the cursor chosen in the menu in for Windows' arrow while the app runs.
pub struct Highlight {
    shown: Arc<Mutex<Shown>>,
}

impl Highlight {
    /// Leaves the cursor as it is until one is chosen; then keeps it at the size for
    /// the monitor the cursor is on, looked at on a thread of its own.
    pub fn start() -> Self {
        let shown = Arc::new(Mutex::new(Shown::default()));
        let kept = Arc::clone(&shown);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(POLL);
                keep(&mut kept.lock().unwrap_or_else(PoisonError::into_inner));
            }
        });
        Self { shown }
    }

    /// The arrow in `palette` in for Windows' arrow, or Windows' own back (`None`), at
    /// once. Windows' own are put back only where the app put its arrow in, so another
    /// program's cursor is left alone.
    pub fn set(&self, palette: Option<Palette>) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        shown.wanted = palette;
        keep(&mut shown);
    }
}

/// Windows' arrow as `shown` wants it.
fn keep(shown: &mut Shown) {
    if let Some(palette) = shown.wanted {
        let wanted = (size_here(), palette);
        if shown.drawn != Some(wanted) {
            if let Err(err) = apply(wanted.0, palette) {
                eprintln!("cannot change the cursor: {err}");
            }
            shown.drawn = Some(wanted);
        }
    } else if shown.drawn.take().is_some() {
        restore();
    }
}

/// Windows' own cursors back, as the user's scheme has them.
fn restore() {
    // SAFETY: SPI_SETCURSORS reloads the system cursors and takes no buffer, so passing no pointer
    // is valid.
    if unsafe { SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0) } == 0 {
        eprintln!("cannot put Windows' cursors back");
    }
}

/// The arrow in `palette`, `size` pixels square, in for Windows' arrow, and the same
/// arrow with a gear beside it, its colours going round, for the arrow Windows shows
/// while a program starts (Windows' own is smaller than the arrow, so the arrow looked
/// gone while the game loaded, the user's of 2026-11-13).
fn apply(size: u32, palette: Palette) -> Result<(), String> {
    let arrow = super::cur(&super::picture(size, palette), size, super::hotspot(size));
    put("cur", &arrow, size, OCR_NORMAL)?;
    // A cursor is at most 256 pixels square, and this one wider than its arrow by the gear.
    let small = size.min(157);
    let width = super::busy_side(small);
    let frames: Vec<Vec<u8>> = super::busy_frames(small, palette)
        .iter()
        .map(|frame| super::cur(frame, width, super::hotspot(small)))
        .collect();
    put(
        "ani",
        &super::ani(&frames, super::TURN_JIFFIES),
        width,
        OCR_APPSTARTING,
    )
}

/// The cursor `file` (of the kind its extension `kind` names), `size` pixels square, in
/// for Windows' cursor `id`. Windows loads it from a file, written afresh each time and
/// removed once loaded.
fn put(kind: &str, file: &[u8], size: u32, id: u32) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!(
        "bluearchive-aoi-cursor-{}.{kind}",
        std::process::id()
    ));
    std::fs::write(&path, file).map_err(|e| e.to_string())?;
    let cursor = load(&path, size);
    let _ = std::fs::remove_file(&path);
    // SAFETY: `cursor` is a cursor `load` just created and nothing else owns; SetSystemCursor takes
    // it over.
    if unsafe { SetSystemCursor(cursor?, id) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn load(path: &Path, size: u32) -> Result<HCURSOR, String> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let pixels = i32::try_from(size).map_err(|e| e.to_string())?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call, and with
    // LR_LOADFROMFILE no module handle is needed.
    let handle = unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            wide.as_ptr(),
            IMAGE_CURSOR,
            pixels,
            pixels,
            LR_LOADFROMFILE,
        )
    };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(handle)
}

/// As big as Windows draws its arrow on the monitor the cursor is on: the pointer size
/// the user chose (`CursorBaseSize`, 32 by default) at that monitor's DPI.
fn size_here() -> u32 {
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (key, value) = (wide(r"Control Panel\Cursors"), wide("CursorBaseSize"));
    let mut base = 0u32;
    let mut len = u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4);
    // SAFETY: `key` and `value` are NUL-terminated and outlive the call; `base` and `len` are live
    // locals, `len` holding the byte size of `base`, which is all a REG_DWORD value can write.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut base).cast::<c_void>(),
            &raw mut len,
        )
    };
    if read != 0 || base == 0 {
        base = 32;
    }
    let size = base.min(256) as f32 * dpi_here().min(960) as f32 / 96.0;
    super::to_byte_range(size, 255.0).max(32)
}

/// The DPI of the monitor the cursor is on (96 when it cannot be read).
fn dpi_here() -> u32 {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a live local the call writes the cursor position into.
    if unsafe { GetCursorPos(&raw mut point) } == 0 {
        return 96;
    }
    // SAFETY: MonitorFromPoint only reads the point; with MONITOR_DEFAULTTONEAREST it always
    // returns a monitor.
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: `monitor` came from MonitorFromPoint, and `x` and `y` are live locals the call writes
    // into.
    let read = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &raw mut x, &raw mut y) };
    if read >= 0 && x > 0 { x } else { 96 }
}

/// The size Windows loads the cursor file at `path` in, asked for `size` (for tests).
#[cfg(test)]
pub fn loaded_size(path: &Path, size: u32) -> Option<u32> {
    use windows_sys::Win32::{
        Graphics::Gdi::{BITMAP, DeleteObject, GetObjectW},
        UI::WindowsAndMessaging::{DestroyCursor, GetIconInfo, ICONINFO},
    };
    let cursor = load(path, size).ok()?;
    // SAFETY: ICONINFO is plain data, all zeros a valid value the call overwrites.
    let mut info: ICONINFO = unsafe { std::mem::zeroed() };
    // SAFETY: `cursor` is the live cursor `load` returned, and `info` is a live local the call
    // fills in.
    if unsafe { GetIconInfo(cursor, &raw mut info) } == 0 {
        return None;
    }
    // SAFETY: BITMAP is plain data, all zeros a valid value the call overwrites.
    let mut bitmap: BITMAP = unsafe { std::mem::zeroed() };
    // SAFETY: `hbmColor` is the bitmap GetIconInfo returned, and the size passed is that of
    // `bitmap`, the live local written into.
    let read = unsafe {
        GetObjectW(
            info.hbmColor,
            i32::try_from(std::mem::size_of::<BITMAP>()).unwrap_or(0),
            (&raw mut bitmap).cast::<c_void>(),
        )
    };
    // SAFETY: the two bitmaps GetIconInfo created and the cursor `load` created belong to this
    // function alone and are freed exactly once, after their last use.
    unsafe {
        DeleteObject(info.hbmColor);
        DeleteObject(info.hbmMask);
        DestroyCursor(cursor);
    }
    (read > 0).then(|| u32::try_from(bitmap.bmWidth).unwrap_or(0))
}
