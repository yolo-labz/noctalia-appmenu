//! Integration coverage for the spec-016 / ADR-0031 desktop fallback,
//! exercised through the crate's PUBLIC `desktop` surface (env-free —
//! resolution is pointed at a `tempfile` fixture dir via `resolve_in`).

use noctalia_appmenu_bridge::desktop;

const CHROME: &str = "\
[Desktop Entry]
Type=Application
Name=Google Chrome
Exec=/nix/store/abc-google-chrome/bin/google-chrome-stable %U
StartupWMClass=google-chrome
Actions=new-window;new-private-window;

[Desktop Action new-window]
Name=New Window
Exec=/nix/store/abc-google-chrome/bin/google-chrome-stable

[Desktop Action new-private-window]
Name=New Incognito Window
Exec=/nix/store/abc-google-chrome/bin/google-chrome-stable --incognito
";

#[test]
fn parse_and_field_codes_are_public() {
    let e = desktop::parse_entry(CHROME, "google-chrome").expect("parse");
    assert_eq!(e.name, "Google Chrome");
    assert_eq!(e.actions.len(), 2);
    // %U stripped; nix-store absolute path preserved as argv[0].
    let argv = desktop::exec_to_argv(&e.exec);
    assert_eq!(
        argv,
        vec!["/nix/store/abc-google-chrome/bin/google-chrome-stable"]
    );
}

#[test]
fn resolve_in_then_fallback_menu_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("google-chrome.desktop"), CHROME).unwrap();

    // Direct-id resolution against the fixture dir (no process env touched).
    let e = desktop::resolve_in(&[tmp.path().to_path_buf()], "google-chrome").expect("resolve");
    assert_eq!(e.id, "google-chrome");
    assert_eq!(e.actions[0].id, "new-window");

    // The public fallback builder produces an active.json-shaped menu.
    // (It resolves against the live system; assert only the invariants
    // that hold regardless of whether chrome is installed on the runner.)
    let menu = desktop::fallback_menu("definitely.not.installed.app.xyz").expect("identity menu");
    assert_eq!(menu.children.len(), 2, "App + Window top-level buttons");
    assert_eq!(menu.children[1].label, "Window");
    // Every node serialises (same shape the QML widget already renders).
    let json = serde_json::to_value(&menu).unwrap();
    assert!(json["children"].is_array());
}

#[test]
fn empty_app_id_yields_no_menu() {
    assert!(desktop::fallback_menu("").is_none());
}
