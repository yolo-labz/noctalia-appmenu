//! Iai-callgrind deterministic instruction-count benchmarks.
//!
//! Wall-clock benchmarks (`hotpath.rs`, divan) are unreliable in CI —
//! shared runners introduce jitter. iai-callgrind runs the bench
//! under callgrind and counts cycles deterministically; CI can gate
//! on >5% regressions.
//!
//! Run: `cargo bench --bench iai --features iai` (requires valgrind
//! on the host — devShell ships it).
//!
//! Post-PR-54 update: niri::handle_event was retired when the bridge
//! adopted niri-ipc::Socket + EventStreamState. The replacement pure
//! transducer is `niri::detect_focus_change(event, state)`. Benches
//! below exercise it against a populated EventStreamState seeded from
//! a WindowsChanged event with N synthetic windows.

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Event, Window, WindowLayout};
use noctalia_appmenu_bridge::{
    active::snapshot,
    niri::{detect_focus_change, FocusEvent},
    registrar::MenuMap,
};
use std::collections::HashMap;
use zbus::zvariant::ObjectPath;

fn make_window(id: u64, pid: Option<i32>, focused: bool) -> Window {
    Window {
        id,
        app_id: Some(format!("app-{id}")),
        title: Some(format!("title-{id}")),
        pid,
        workspace_id: Some(1),
        is_focused: focused,
        is_floating: false,
        is_urgent: false,
        layout: WindowLayout {
            pos_in_scrolling_layout: None,
            tile_size: (0.0, 0.0),
            window_size: (0, 0),
            tile_pos_in_workspace_view: None,
            window_offset_in_tile: (0.0, 0.0),
        },
        focus_timestamp: None,
    }
}

fn seeded_state(n: u64) -> EventStreamState {
    let windows: Vec<Window> = (0..n)
        .map(|i| make_window(i, Some(1000 + i as i32), false))
        .collect();
    let mut state = EventStreamState::default();
    let _ = state.apply(Event::WindowsChanged { windows });
    state
}

fn build_menus(n: u32) -> MenuMap {
    let mut by_pid = HashMap::new();
    for i in 0..n {
        by_pid.insert(
            1000 + i,
            (
                format!("org.example.App{i}"),
                ObjectPath::try_from(format!("/org/example/App{i}/menu"))
                    .unwrap()
                    .into(),
            ),
        );
    }
    MenuMap { by_pid }
}

#[library_benchmark]
fn detect_focus_known() {
    let state = seeded_state(50);
    let evt = Event::WindowFocusChanged { id: Some(7) };
    let _ = std::hint::black_box(detect_focus_change(&evt, &state));
}

#[library_benchmark]
fn detect_focus_unknown() {
    let state = seeded_state(50);
    let evt = Event::WindowFocusChanged { id: Some(99) };
    let _ = std::hint::black_box(detect_focus_change(&evt, &state));
}

#[library_benchmark]
fn snapshot_match() {
    let menus = build_menus(100);
    let focus = FocusEvent {
        winid: 1,
        pid: 1042,
        app_id: "App".into(),
        title: "Title".into(),
    };
    let _ = std::hint::black_box(snapshot(Some(&focus), &menus));
}

#[library_benchmark]
fn snapshot_no_match() {
    let menus = build_menus(100);
    let focus = FocusEvent {
        winid: 1,
        pid: 99999,
        app_id: "Firefox".into(),
        title: "Title".into(),
    };
    let _ = std::hint::black_box(snapshot(Some(&focus), &menus));
}

library_benchmark_group!(
    name = hot;
    benchmarks = detect_focus_known, detect_focus_unknown, snapshot_match, snapshot_no_match
);

main!(library_benchmark_groups = hot);
