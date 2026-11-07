//! The window opens where it was left: on the monitor it was on, at the place and size it had,
//! kept in the settings as it moves; on the primary monitor, centred, when that one is gone.

use std::time::Duration;

use gpui::{TestAppContext, point, px, size};

use super::open;
use crate::{
    keep::Kept,
    settings::{Placement, Settings},
    ui::opening_place,
};

fn placed(display: &str) -> Placement {
    Placement {
        display: display.to_owned(),
        x: -1580,
        y: 360,
        width: 734,
        height: 858,
    }
}

#[test]
fn opens_on_the_monitor_it_was_left_on_where_it_was() {
    let displays = [("primary".to_owned(), 1), ("portrait".to_owned(), 2)];
    let (display, bounds) =
        opening_place(Some(&placed("portrait")), &displays).expect("a place on the portrait one");
    assert_eq!(display, 2, "the monitor it was on");
    assert_eq!(bounds.origin, point(px(-1580.), px(360.)), "where it was");
    assert_eq!(bounds.size, size(px(734.), px(858.)), "as big as it was");
}

#[test]
fn opens_as_before_when_its_monitor_is_gone_or_none_was_kept() {
    let displays = [("primary".to_owned(), 1)];
    assert_eq!(
        opening_place(Some(&placed("portrait")), &displays),
        None,
        "monitor gone"
    );
    assert_eq!(opening_place(None, &displays), None, "nothing kept");
}

#[gpui::test]
fn keeps_its_place_once_it_stops_changing(cx: &mut TestAppContext) {
    let (_, core, vcx) = open(cx, &Kept::default());
    vcx.simulate_resize(size(px(800.), px(900.)));
    vcx.run_until_parked();
    assert_eq!(
        Settings::load(&core.places.settings).window,
        None,
        "not written while it may still be changing"
    );
    vcx.executor().advance_clock(Duration::from_secs(1));
    vcx.run_until_parked();
    let kept = Settings::load(&core.places.settings)
        .window
        .expect("the place kept");
    assert_eq!((kept.width, kept.height), (800, 900), "its size kept");
    let display = vcx.update(|window, cx| {
        window
            .display(cx)
            .and_then(|d| d.uuid().ok())
            .map(|u| u.to_string())
    });
    assert_eq!(Some(kept.display), display, "the monitor it is on");
}
