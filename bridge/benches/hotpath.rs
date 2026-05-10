//! Divan-based wall-clock benchmarks for the bridge's hot paths.
//!
//! Run with `just bridge-bench` (or `cargo bench --bench hotpath`).
//! Targets: ≤ 100 µs per iteration on the developer's 7950X3D —
//! anything above suggests a regression worth investigating.
//!
//! For deterministic-instruction-count CI gating, see `iai.rs`
//! (cargo bench --bench iai --features iai).
//!
//! Post-PR-54 update: niri::handle_event was retired when the bridge
//! adopted niri-ipc::Socket + EventStreamState. Benches below exercise
//! `niri::detect_focus_change(event, state)` against an EventStreamState
//! seeded from a WindowsChanged event with N synthetic windows, plus the
//! unchanged `active::snapshot` reducer.

use divan::{black_box, Bencher};
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Event, Window, WindowLayout};
use noctalia_appmenu_bridge::{
    active::{snapshot, ActiveSnapshot},
    niri::{detect_focus_change, FocusEvent},
    registrar::MenuMap,
};
use std::collections::HashMap;
use zbus::zvariant::ObjectPath;

fn main() {
    divan::main();
}

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

fn make_focus(pid: u32) -> FocusEvent {
    FocusEvent {
        winid: 1,
        pid,
        app_id: "App".into(),
        title: "Title".into(),
    }
}

fn make_menus(n: usize) -> MenuMap {
    let mut by_pid = HashMap::new();
    for i in 0..n {
        by_pid.insert(
            (1000 + i) as u32,
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

#[divan::bench]
fn focus_known_window(bencher: Bencher) {
    let state = seeded_state(50);
    bencher.bench(|| {
        let evt = Event::WindowFocusChanged {
            id: Some(black_box(7)),
        };
        detect_focus_change(black_box(&evt), black_box(&state))
    });
}

#[divan::bench]
fn focus_unknown_window_resyncs(bencher: Bencher) {
    let state = EventStreamState::default();
    bencher.bench(|| {
        let evt = Event::WindowFocusChanged {
            id: Some(black_box(99)),
        };
        detect_focus_change(black_box(&evt), black_box(&state))
    });
}

#[divan::bench(args = [10, 100, 1000])]
fn snapshot_with_n_menus(bencher: Bencher, n: usize) {
    let menus = make_menus(n);
    let focus = make_focus(1500);
    bencher.bench(|| snapshot(black_box(Some(&focus)), black_box(&menus)));
}

#[divan::bench]
fn snapshot_no_focus(bencher: Bencher) {
    let menus = make_menus(100);
    bencher.bench(|| snapshot(black_box(None), black_box(&menus)));
}

#[divan::bench]
fn snapshot_focus_no_match(bencher: Bencher) {
    let menus = make_menus(100);
    let focus = make_focus(99999);
    bencher.bench(|| snapshot(black_box(Some(&focus)), black_box(&menus)));
}

#[divan::bench]
fn empty_active_snapshot_construction(bencher: Bencher) {
    bencher.bench(|| black_box(ActiveSnapshot::empty()));
}
