//! Iai-callgrind deterministic instruction-count benchmarks.
//!
//! Wall-clock benchmarks (`hotpath.rs`, divan) are unreliable in CI —
//! shared runners introduce jitter. iai-callgrind runs the bench
//! under callgrind and counts cycles deterministically; CI can gate
//! on >5% regressions.
//!
//! Run: `cargo bench --bench iai --features iai` (requires valgrind
//! on the host — devShell ships it).

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use noctalia_appmenu_bridge::{
    active::snapshot,
    niri::{handle_event, FocusEvent, NiriEvent, NiriWindow},
    registrar::MenuMap,
};
use std::collections::HashMap;
use zbus::zvariant::ObjectPath;

fn build_cache(n: u64) -> HashMap<u64, NiriWindow> {
    let mut cache = HashMap::new();
    for i in 0..n {
        cache.insert(
            i,
            NiriWindow {
                id: i,
                app_id: Some(format!("app-{i}")),
                title: Some(format!("title-{i}")),
                pid: Some(1000 + i as u32),
                workspace_id: Some(1),
                is_focused: Some(false),
            },
        );
    }
    cache
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
fn handle_focus_known() {
    let cache = build_cache(50);
    let _ = std::hint::black_box(handle_event(
        NiriEvent::WindowFocusChanged { id: Some(7) },
        &cache,
    ));
}

#[library_benchmark]
fn handle_focus_unknown() {
    let cache = build_cache(50);
    let _ = std::hint::black_box(handle_event(
        NiriEvent::WindowFocusChanged { id: Some(99) },
        &cache,
    ));
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
    benchmarks = handle_focus_known, handle_focus_unknown, snapshot_match, snapshot_no_match
);

main!(library_benchmark_groups = hot);
