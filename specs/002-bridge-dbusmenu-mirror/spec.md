# Specification: Bridge DBusMenu mirror (v0.2)

**ID:** 002-bridge-dbusmenu-mirror
**Created:** 2026-05-04
**Author:** @phsb5321
**Constitution version:** 1.0.0

## Why

v0.1 ships `noctalia-appmenu` in **fallback-only** mode — the bar widget renders the focused application's `app_id`, not its menu tree (ADR-0015). The headline feature (macOS-style menubar in noctalia's topbar) is blocked on Quickshell's `DBusMenuHandle` being `QML_UNCREATABLE`.

This spec captures the path to v0.2: have the bridge implement `com.canonical.dbusmenu` server-side at a constant address, so the QML widget gets a stable handle it can render.

## User scenarios

### Scenario 1: Anki menubar in the bar (Qt6 app)

**Given** Anki is running with `appmenu-qt5` loaded and `noctalia-appmenu-bridge` v0.2 is up
**When** Anki receives keyboard focus
**Then** Within 200 ms, Anki's `File / Edit / View / Tools / Help / Ankimon / AnKing` menu items appear as buttons in noctalia's topbar; each is clickable to open its submenu

### Scenario 2: Submenu activation

**Given** Anki's `File` menu button is rendered and the user clicks it
**When** the popup opens with `Open / Save / Export / Quit / …`
**Then** clicking `Export…` activates the corresponding `DBusMenuItem` in Anki's process; Anki responds as if the menu was clicked in-window

### Scenario 3: Layout updates

**Given** kate is focused; user opens a new file
**When** kate emits `LayoutUpdated` because its `Window` menu now has the new buffer
**Then** Within 250 ms (debounce), the bar widget re-renders to include the new entry without flicker

### Scenario 4: Multi-window same-app

**Given** two qutebrowser windows are open
**When** focus moves between them
**Then** Each window's menu (potentially different — recent-tabs lists differ) renders correctly when its window is focused

## Functional requirements

- **FR-001** Bridge implements `com.canonical.dbusmenu` server-side at `org.noctalia.AppMenu` `/org/noctalia/AppMenu/Active/menu`.
- **FR-002** On every active-snapshot change, the bridge subscribes (as a client) to the upstream app's `(busName, objectPath)` DBusMenu.
- **FR-003** Bridge forwards `GetLayout`, `Event`, `EventGroup`, `AboutToShow`, `AboutToShowGroup` calls to the upstream menu service, with object-path rewriting.
- **FR-004** Bridge forwards `LayoutUpdated`, `ItemsPropertiesUpdated`, `ItemActivationRequested` signals downstream from upstream, with object-path rewriting.
- **FR-005** Bridge forwards property reads (`Status`, `IconThemePath`, `Version`, `TextDirection`) on the canonical interface.
- **FR-006** When focus moves, bridge cleanly transitions: send `LayoutUpdated(revision+1, 0)` so consumers re-fetch the root.
- **FR-007** When the upstream service disappears (app exited), bridge publishes an empty layout (`children=[]`) and emits `LayoutUpdated`.
- **FR-008** QML widget consumes the fixed-path mirror via Quickshell's tray-style API (`SystemTrayItem.menu`-equivalent for an arbitrary fixed address — pending Quickshell-side change OR via `MockSystemTrayItem` workaround documented in v0.2 plan).

## Non-functional requirements

- **NFR-001 Performance.** Mirror call forwarding adds ≤ 5 ms P95 latency over direct upstream calls.
- **NFR-002 Reliability.** Mirror survives apps that disappear mid-call; aborts the in-flight call cleanly without leaking the proxy.
- **NFR-003 Security.** Mirror does not execute arbitrary remote callable; only forwards canonical DBusMenu method names.
- **NFR-004 Testability.** Mirror is unit-testable with mocked upstream client; integration test under `niri --headless` + fake registrar verifies end-to-end.

## Out of scope

- Rich item types (icons, toggle states beyond `checked`/`unchecked`, type-specific renderers): defer to v0.3.
- Submenu lazy-loading optimisation (always fetch full layout for now).
- Forwarding `IconData` (full bitmap pass-through): use icon name only.

## Constraints / dependencies

- zbus 4.x `#[interface]` macro for server-side; `#[proxy]` for client-side.
- The path-rewrite is a function from `(upstream_path, _) → fixed_path/<incremented_id>`. Must be deterministic across reconnects.
- ADR-0007 second-half supersedes the v0.1 limitation.

## Success criteria

- **SC-001** Anki's full menu (3+ items) renders in the bar; each item is activatable.
- **SC-002** Switching focus between Anki and kate updates the bar within NFR-001 latency.
- **SC-003** Disposing all upstream apps results in an empty bar (no panic, no orphaned proxy).
- **SC-004** Memory stable over 7 days uninterrupted use (NFR-002 of spec 001 carried forward).
- **SC-005** Integration test green on `niri --headless` + fake-registrar covering Scenarios 1-3.

## Open questions

1. Does Quickshell's `SystemTrayItem.menu` give us a `DBusMenuHandle` we can re-target via property binding to a hard-coded fixed path? Or do we still need a Quickshell-side change to expose `DBusMenuHandle.create(service, path)`?
2. How do we test the mirror without spawning a real Qt/GTK app? Probably via the existing Python `fake-registrar` extended to publish a real DBusMenu (it already does — but assertions need expanding).
3. Stale-cache handling: should we drop the mirror state when no app is focused? Currently active-snapshot empty → no upstream connection → empty layout. Verify this edge case works.
