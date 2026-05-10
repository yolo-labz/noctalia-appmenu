# Changelog

All notable changes to noctalia-appmenu are documented here. Format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and adheres to
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0-alpha.1] — 2026-05-06

### Features — AT-SPI substrate (replaces v0.2 DBusMenu mirror)

- **Bridge: AT-SPI menubar walker (`bridge/src/atspi.rs`).** Replaces v0.2's
  DBusMenu/Registrar approach. The DBusMenu protocol's auto-registration
  only fires on KWin Wayland (it relies on `org_kde_kwin_appmenu_manager`);
  every other compositor — niri, Hyprland, Sway, COSMIC — silently fails to
  register Qt/GTK apps. AT-SPI bypasses protocol cooperation entirely by
  walking the accessibility tree that Qt and GTK expose for free with
  `QT_ACCESSIBILITY=1` / GTK's ATK bridge.
- **`enable_a11y()` flips `org.a11y.Status.IsEnabled = true`** at bridge
  startup. niri ships no AT (Orca etc), so without this Qt's accessibility
  bridge polls `false` at QApplication construction and never registers.
- **`fetch_menubar_for_pid()` resolves PID → AT-SPI app → MenuBar** by
  walking `org.a11y.atspi.Registry`, matching app PIDs via
  `GetConnectionUnixProcessID` on the a11y bus, then DFS-searching for
  role 34 (`MENU_BAR`). Recursion bounded at 8 levels (find) and 6 levels
  (fetch).
- **Popup-wrapper flattening**: Qt wraps every MENU_ITEM's popup in an
  unnamed MENU child; the walker detects the shape (1 unnamed child with
  grandchildren) and pulls the grandchildren up so the QML widget renders
  the actual menu items, not an empty placeholder.
- **`atspi-click <service> <path>` subcommand** in `bridge/src/main.rs`
  forwards clicks via `org.a11y.atspi.Action.DoAction(0)` (qtatspi
  convention: action 0 = "click"). Replaces v0.2's `click <busName>
  <menuPath> <itemId>` subcommand.
- **Plugin: `BarWidget.qml` `fireClick(item)` swapped to atspi-click**,
  passing AT-SPI `(service, path)` coordinates carried in each menu item
  (added to the JSON shape — same keys, different addressing).
- **Bridge `examples/atspi_probe.rs`** — manual probe binary for live
  verification: `cargo run --example atspi_probe -- <pid>` prints the
  parsed menubar JSON for any focused app.

### Behavior changes

- DBusMenu/Registrar code paths in `bridge/src/dbusmenu.rs` and
  `bridge/src/registrar.rs` are still compiled but no longer feed the
  active proxy — `proxy.rs::run()` now calls `atspi::fetch_menubar_for_pid`
  with the focus PID directly. The dead modules will be retired in
  v0.3.x once we're sure no app needs the DBusMenu fallback.
- `active.json` `menu` field is now sourced from AT-SPI; per-item
  `service` and `path` now address AT-SPI accessibles, not DBusMenu
  items. The QML widget treats them as opaque — no semantic change.

### Documentation

- ADR-0024 records the AT-SPI substrate decision (Path A) and the
  DBusMenu retirement rationale.

### Verified live (2026-05-06, desktop / niri / Qt 6.11)

- okular menubar walked: 9 top-level items (File / View / Edit / Go /
  Bookmarks / Tools / Settings / sep / Help) with deep submenus
  (e.g. File → Open Recent → 30 recent documents).
- Wire-level role enum confirmed: MENU_BAR=34, MENU=33, MENU_ITEM=35,
  SEPARATOR=50 (NOT pyatspi's 50/51/52/67 — different enum).
- State bitmask: ENABLED=bit 8, SENSITIVE=bit 24 (NOT bits 20/37 as
  v0.3.0-alpha pre-release scaffold assumed).

[0.3.0-alpha.1]: https://github.com/yolo-labz/noctalia-appmenu/compare/v0.2.0-alpha.1...v0.3.0-alpha.1

## [0.2.0-alpha.1] — 2026-05-06

### Features

- Initial scaffold: bridge + plugin + Nix module + CI/CD + speckit constitution.

### Bug fixes

- Bridge hard-fails at startup when `niri msg windows` is unreachable, instead
  of silently rendering an invisible menubar (audit P1).
- `default_config_path()` no longer falls back to `/tmp` when both
  `XDG_CONFIG_HOME` and `HOME` are unset; refuses with a clear error
  (audit P0).
- Registrar bus-name parsing now reports the offending name on failure
  (audit P1).

### Refactor

- `niri::handle_event` extracted as a pure transducer (cache-read, op-emit)
  so the schema-drift path is unit-testable without spawning `niri`.
- `active::snapshot` extracted as a pure reducer.
- `proxy::ActiveProxy` switched from `RwLock` to `Mutex` (no concurrent
  readers; lower acquisition cost).
- Named constants for `FOCUS_DEBOUNCE_DEFAULT_MS`, `REGISTRAR_DEBOUNCE_DEFAULT_MS`,
  `PUBLISH_SERVICE_DEFAULT`, `PUBLISH_PATH_DEFAULT`.

### Tests

- 14 new unit tests across `niri::tests`, `config::tests`, `active::tests`
  (was 1 integration test only).

### Tooling

- `cargo-deny` config (advisories / bans / licenses / sources).
- `typos` ecosystem allow-list.
- `semgrep` 5 custom Rust rules (no-unwrap-outside-tests, no-raw-env-var,
  tokio-spawn-must-handle-error, no-println-in-lib, no-todo-fixme).
- `gitleaks` 5 explicit rules (github PAT fine-grained, sonar token,
  Tailscale MagicDNS, internal LAN IPs, generic bearer).
- New CI workflows: `cargo-deny`, `typos`, `cargo-machete`, `dco`, `semgrep`.
- `.vscode/`, `.helix/` editor configs (force-added past the user's
  global git ignore).
- `.github/release.yml` grouped release-notes config.
- Expanded `justfile`: `audit`, `unused-deps`, `typos`, `semgrep`, `sbom`,
  `gitleaks`, `loc`, `ci-local`, `release-dry-run`, `verify-release`,
  `bridge-bench`, `bridge-flame`, `qmlformat`.
- `lib.rs`: `#![forbid(unsafe_code)]`, intra-doc-link warnings, ADR
  cross-references in module-level docs.

[Unreleased]: https://github.com/yolo-labz/noctalia-appmenu/compare/HEAD...HEAD
