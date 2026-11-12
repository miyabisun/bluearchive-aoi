//! What the recognition tests share.

use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use image::{
    RgbImage,
    imageops::{FilterType, resize},
};

/// `image` as a screen drawing the game `width` wide shows it (see
/// `aoi::screen::drawn_width`), a window's, a tablet's or a stream's alike,
/// and as Aoi reads it (`aoi::screen::normalize`). `image`'s resize takes 80 to 100 ms a
/// screen where reading one takes 3 (2026-11-13): a test opens a screen once, and draws its
/// screens side by side (`each`).
pub fn drawn_at(image: &RgbImage, width: u32) -> RgbImage {
    let scale = width as f32 / aoi::screen::drawn_width(image);
    let (w, h) = (image.width() as f32 * scale, image.height() as f32 * scale);
    let screen = resize(
        image,
        w.round() as u32,
        h.round() as u32,
        FilterType::Triangle,
    );
    aoi::screen::normalize(&screen).into_owned()
}

/// The recognizer over `data/`, loaded once for every test that only reads with it. (A test
/// that counts what its recognizer read, or reads a shop, whose last frame the recognizer
/// keeps, loads its own.)
pub fn recognizer() -> &'static aoi::Recognizer {
    static LOADED: LazyLock<aoi::Recognizer> = LazyLock::new(|| {
        aoi::Recognizer::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
            .expect("the recognizer loads data/")
    });
    &LOADED
}

/// What `tell` tells of each of `of`, in their order, told side by side on as many threads
/// as the machine runs at once: the screens of a test are read apart from one another.
pub fn each<T: Sync, R: Send>(of: &[T], tell: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let told: Vec<Mutex<Option<R>>> = of.iter().map(|_| Mutex::new(None)).collect();
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads.min(of.len()) {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(one) = of.get(at) else { break };
                    *told[at].lock().expect("no other holds it") = Some(tell(one));
                }
            });
        }
    });
    told.into_iter()
        .map(|one| {
            one.into_inner()
                .expect("no thread failed")
                .expect("each was told")
        })
        .collect()
}
