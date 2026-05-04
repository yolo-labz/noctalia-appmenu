//! Divan-based wall-clock benchmarks for the bridge's hot paths.
//!
//! Run with `just bridge-bench` (or `cargo bench --bench hotpath`).
//! Targets: ≤ 100 µs per iteration on the developer's 7950X3D —
//! anything above suggests a regression worth investigating.
//!
//! For deterministic-instruction-count CI gating, see `iai.rs`
//! (cargo bench --bench iai --features iai).

use divan::{black_box, Bencher};
use noctalia_appmenu_bridge::{
    active::{snapshot, ActiveSnapshot},
    niri::{handle_event, FocusEvent, MapOp, NiriEvent, NiriWindow},
    registrar::MenuMap,
};
use std::collections::HashMap;
use zbus::zvariant::ObjectPath;

fn main() {
    divan::main();
}

fn make_window(id: u64, pid: Option<u32>) -> NiriWindow {
    NiriWindow {
        id,
        app_id: Some(format!("app-{id}")),
        title: Some(format!("title-{id}")),
        pid,
        workspace_id: Some(1),
        is_focused: Some(false),
    }
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
    let mut cache = HashMap::new();
    for i in 0..50 {
        cache.insert(i, make_window(i, Some(1000 + i as u32)));
    }
    bencher.bench(|| {
        let evt = NiriEvent::WindowFocusChanged { id: Some(black_box(7)) };
        handle_event(black_box(evt), black_box(&cache))
    });
}

#[divan::bench]
fn focus_unknown_window_resyncs(bencher: Bencher) {
    let cache: HashMap<u64, NiriWindow> = HashMap::new();
    bencher.bench(|| {
        let evt = NiriEvent::WindowFocusChanged { id: Some(black_box(99)) };
        handle_event(black_box(evt), black_box(&cache))
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
fn upsert_window(bencher: Bencher) {
    let cache: HashMap<u64, NiriWindow> = HashMap::new();
    let win = make_window(42, Some(1234));
    bencher.bench(|| {
        let evt = NiriEvent::WindowOpenedOrChanged { window: win.clone() };
        match handle_event(black_box(evt), black_box(&cache)) {
            MapOp::Upsert(_, _) => {}
            other => panic!("unexpected: {other:?}"),
        }
    });
}

#[divan::bench]
fn empty_active_snapshot_construction(bencher: Bencher) {
    bencher.bench(|| black_box(ActiveSnapshot::empty()));
}
