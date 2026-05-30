//! Manual probe: resolve a Wayland `app_id` to its `.desktop` entry and
//! print the `source = "desktop-fallback"` menu the bridge would embed
//! in `active.json` when the app exposes no AT-SPI menubar (spec 016).
//! Not part of the production daemon — a live-verification aid.
//!
//! Usage:
//!   cargo run --example `desktop_probe` -- <app_id> [<app_id> ...]
//!
//! Example:
//!   cargo run --example desktop_probe -- google-chrome obsidian \
//!       com.mitchellh.ghostty firefox-nightly

use noctalia_appmenu_bridge::desktop;

fn main() {
    let app_ids: Vec<String> = std::env::args().skip(1).collect();
    if app_ids.is_empty() {
        eprintln!("usage: desktop_probe <app_id> [<app_id> ...]");
        std::process::exit(2);
    }
    for app_id in &app_ids {
        println!("==== {app_id} ====");
        match desktop::resolve(app_id) {
            Some(e) => println!(
                "resolved: id={} name={:?} exec={:?} actions={}",
                e.id,
                e.name,
                e.exec,
                e.actions.len()
            ),
            None => println!("resolved: <none> (minimal identity fallback)"),
        }
        match desktop::fallback_menu(app_id) {
            Some(m) => match serde_json::to_string_pretty(&m) {
                Ok(j) => println!("{j}"),
                Err(e) => println!("(serialise error: {e})"),
            },
            None => println!("null (no identity → source=empty)"),
        }
        println!();
    }
}
