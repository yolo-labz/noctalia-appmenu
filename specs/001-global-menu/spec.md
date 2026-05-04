# Specification: Global menu MVP

**ID:** 001-global-menu
**Created:** 2026-05-04
**Author:** @phsb5321
**Constitution version:** 1.0.0

## Why

Users on niri running noctalia-shell expect parity with macOS / KDE Plasma's global menu UX: the focused application's `File / Edit / View / …` menubar should appear in the top bar, not inside each window. The capability is available in pieces (Quickshell DBusMenu consumer, niri-IPC, vala-panel-appmenu's registrar daemon) but no one has wired them together; users currently see an empty bar slot when the focused app is Anki / kate / qutebrowser.

The MVP delivers a working v0.1.0 — Anki's menubar in the top bar, focus-following — and validates the end-to-end architecture (sidecar bridge + QML widget + standalone registrar). It deliberately does not chase Firefox / Electron support (out of scope per constitution) or implement the bridge's own minimal-registrar fallback (deferred to v0.2).

## User scenarios

### Scenario 1: Single Qt6 app focused

**Given** Anki is running and `appmenu-registrar` + `noctalia-appmenu-bridge` are up
**When** Anki receives keyboard focus
**Then** Within ≤ 200 ms (75 ms debounce + render), Anki's `File / Edit / View / Tools / Help / Ankimon / AnKing` menu appears in the noctalia top bar; clicking `File` opens a popup matching Anki's in-window menu

### Scenario 2: Focus switch between two Qt apps

**Given** Anki and kate are both running, both registered with the registrar
**When** focus moves from Anki to kate
**Then** Within 200 ms, the bar's menubar updates from Anki's to kate's menu tree; the previous popup (if any) closes

### Scenario 3: Focus to an unregistered app

**Given** Firefox is running (does not register a DBusMenu)
**When** Firefox receives focus
**Then** The bar shows a single button labelled `Firefox` (from the `.desktop` entry); clicking it opens a minimal pseudo-menu with `About / Quit`; no error is logged

### Scenario 4: Bridge crash / restart

**Given** `noctalia-appmenu-bridge.service` is killed
**When** systemd-user restarts it within 5 s
**Then** The bar widget hides during the gap; on restart, it re-attaches to the active proxy and resumes rendering

### Scenario 5: Submenu navigation

**Given** Anki is focused and `File` menu is open
**When** the user hovers `File → Export…`
**Then** The submenu popup opens to the right; clicking `Export…` activates the menu item via DBusMenu and closes all popups

## Functional requirements

- **FR-001** Bridge owns `org.noctalia.AppMenu` on the user session bus and exposes `/org/noctalia/AppMenu/Active` with properties `bus_name`, `object_path`, `app_id`, `title`.
- **FR-002** Bridge subscribes to `niri msg --json event-stream` and updates the published properties on `WindowFocusChanged` events with a 75 ms trail-edge debounce.
- **FR-003** Bridge resolves `(busName, objectPath)` per process by listening to `com.canonical.AppMenu.Registrar` `WindowRegistered` signals and calling `org.freedesktop.DBus.GetConnectionUnixProcessID` on the signal sender.
- **FR-004** Bridge ignores the registrar's `windowId` argument (X11-only).
- **FR-005** QML widget reads `org.noctalia.AppMenu.Active` properties and renders a horizontal menubar bound to `Quickshell.DBusMenu.DBusMenuHandle`.
- **FR-006** QML widget renders a `.desktop`-derived pseudo-menu when no menu is registered and the bridge publishes a non-empty `app_id`.
- **FR-007** The plugin loads via noctalia's plugin discovery (drop folder under `~/.config/noctalia/plugins/noctalia-appmenu/` + manifest.json + entry in `plugins.json`).
- **FR-008** The Home-Manager module installs and starts both `appmenu-registrar` and `noctalia-appmenu-bridge` as `systemd --user` units bound to `graphical-session.target`.
- **FR-009** When `programs.noctalia.plugins.appmenu.hideInWindowMenubar = true`, the module sets `QT_QPA_PLATFORMTHEME=appmenu-qt5` and `GTK_MODULES=appmenu-gtk-module` in the session env.
- **FR-010** The bridge exits non-zero (so systemd restarts) on niri-IPC unreachable; logs the cause to stderr.

## Non-functional requirements

- **NFR-001 Performance.** Focus-change → menubar-render P95 ≤ 200 ms on the desktop host (Ryzen 7950X3D); P99 ≤ 400 ms.
- **NFR-002 Reliability.** Bridge survives ≥ 7 days of uninterrupted use without leaking memory > 50 MB RSS.
- **NFR-003 Security.** Bridge runs hardened (`NoNewPrivileges`, `ProtectSystem=strict`, `RestrictAddressFamilies=AF_UNIX`); does not exec subprocesses other than `niri msg` (path resolved at startup).
- **NFR-004 Testability.** Unit tests cover the focus + registrar joiners with mocked traits; integration test boots a fake registrar publishing a canned menu and asserts the published proxy mirrors it.
- **NFR-005 Reproducibility.** Both bridge derivations produce byte-identical binaries with identical `SOURCE_DATE_EPOCH`.

## Out of scope

- Firefox / Electron / Chromium global-menu support
- Multi-monitor menubar duplication
- Alt-letter mnemonics + global Alt-F intercept
- Hosting `com.canonical.AppMenu.Registrar` ourselves (delegated to vala-panel-appmenu)
- Hyprland / Sway compositor support

## Constraints / dependencies

- Quickshell ≥ 0.3.0
- noctalia-shell ≥ 1.0.0
- niri (any IPC-1.x compatible build)
- vala-panel-appmenu (registrar daemon) on the user's session
- Qt6 with `appmenu-qt5` platform theme (or Qt5)
- For GTK: `appmenu-gtk-module-wayland` fork (mainline broken on Wayland — KDE bug 424485)

## Success criteria

- **SC-001** Anki's menubar renders in the bar with all three Scenario-1 menus (`File`, `Edit`, `View`) navigable via mouse.
- **SC-002** Switching focus between Anki and kate updates the bar within the NFR-001 latency budget on the developer host.
- **SC-003** `gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz` succeeds against the v0.1.0 release artefact.
- **SC-004** All required CI checks (`ci`, `sonar`, `codeql`, `osv-scanner`, `scorecard`, `reproducibility`, `actionlint`) green on the main branch at v0.1.0.
- **SC-005** README's "Verify the install" section runs clean on a fresh NixOS box (boot → `nh os switch` → focus Anki → menubar appears).
