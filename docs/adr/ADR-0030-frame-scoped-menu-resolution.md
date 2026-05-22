# ADR-0030 — Frame-scoped menu resolution by focused-window title

- **Status:** accepted
- **Date:** 2026-05-21
- **Deciders:** Pedro H S Balbino
- **Amends:** ADR-0004 (resolve registrar by PID), ADR-0024 (AT-SPI substrate)
- **Tracking PR / branch:** `132-wrong-window-routing`

## Context

A single process can own several top-level windows: Firefox (one
`firefox` process, N windows), okular (N PDFs), LibreOffice (Writer +
Calc). niri reports the same `pid` for all of them.

The bridge resolved the focused app's menu by PID (ADR-0004) then walked
its AT-SPI tree (ADR-0024). `find_app_for_pid` pass 1 returned the AT-SPI
**application root** on a PID match and short-circuited; `find_menubar`
then did a first-hit DFS for `MENU_BAR` from that root — picking the first
window's menubar, which is frequently **not** the niri-focused window.
The frame-scoping that selects the active window frame
(`find_active_app_via_state`) was stranded in pass 2, reachable only when
the PID *missed* (xwayland/flatpak), so native-Wayland Firefox never hit
it. The menu cache compounded the error: keyed `(app_id, pid)`, it served
one window's menu for another until its 30 s TTL.

User-visible symptom: focusing one Firefox window (or opening a new tab in
it) showed a different window's menu. This **recurred across v1.0.20
(focus settle), v1.0.21 (telemetry), v1.0.22 (150 ms focus-settle floor)**
— all timing/settle patches that never touched the resolution path. The
"correct" fix (FR-003 accelerator dispatch) was deferred in ADR-0028
because niri-ipc exposes no keyboard-input synthesis. This is the
CLAUDE.md drift-trigger-I case: the architecture, not the timing, was
wrong.

A codex review reading at-spi2-core source confirmed the root cause and
two framework constraints (see below).

## Decision

**Resolve the menu to the niri-focused window's AT-SPI frame, using the
window title as the deterministic discriminator. Re-key the menu cache by
window id.**

In `find_app_for_pid` pass 1, on a PID match the bridge no longer returns
the app root. It enumerates the application's top-level window frames and
selects (`atspi::choose_frame`, a pure, unit-tested policy):

1. **0 or 1 frame** → the app root, unchanged. Single-window apps (the
   overwhelming majority) keep their proven v1.0.x behaviour — zero blast
   radius.
2. **exact title match** — the frame whose accessible Name equals the niri
   focused-window title.
3. **`STATE_ACTIVE`** — tiebreaker only.
4. **otherwise** → `None` (the QML placeholder), never the app root.
   For a multi-window app we cannot identify, a placeholder is correct;
   an arbitrary window's menu is the bug.

The cache key becomes `(app_id, pid, winid)` so per-window menus never
cross-contaminate. The niri `winid` + `title` were already plumbed into
`ActiveSnapshot` (`focus_winid`, `title`); this change finally consumes
them in resolution (`proxy.rs`) instead of dropping them.

## Why title, not STATE_ACTIVE or a window-id join (codex framework review)

- **`ATSPI_STATE_ACTIVE` is not deterministic for same-PID multi-window.**
  at-spi2-core documents that a previously-active top-level frame may
  retain `STATE_ACTIVE` while another window exists
  (`at-spi2-core-2.58.3 …/atspi/atspi-constants.h:839-845`); our own code
  warned the same (`atspi.rs` pass-2 comment). Hence tiebreaker, not
  primary selector.
- **No AT-SPI API maps a frame back to a `wl_surface` or niri window id.**
  `AccessibleId` is a static gtkbuilder id, not runtime window identity;
  PID is process-level. The frame's **accessible Name (= window title)**
  is the only runtime-stable discriminator AT-SPI exposes.
- **Exact match only.** A containment/fuzzy match would mispair Firefox
  windows that share the long `" — Firefox Nightly"` suffix. On a title
  miss we fall back rather than guess.

## Alternatives considered

- **A. Component geometry (`atspi-component.h` extents) vs niri window
  geometry.** A heavier fallback if titles ever collide or are empty.
  Deferred — not needed for the observed cases; revisit if a runtime case
  defeats title matching.
- **B. Keep returning the app root, fix `find_menubar` to prefer the
  active frame's subtree.** Rejected: pushes window-selection into the DFS
  and still leans on `STATE_ACTIVE`; the title signal lives at the
  resolution layer where niri focus is known.
- **C. FR-003 accelerator dispatch (ADR-0028).** Still deferred — niri-ipc
  has no keyboard-input synthesis. Orthogonal to this fix.

## Consequences

### Positive

- The focused Firefox/okular/LibreOffice window's own menu is served.
- Single- and zero-window apps are byte-for-byte unchanged (n ≤ 1 → app
  root).
- Cache cannot serve a stale sibling-window menu (winid in the key).

### Negative

- Correctness depends on niri's window title equalling the toolkit's frame
  Name. True for GTK/Qt/Firefox today; an app that diverges falls back to
  `STATE_ACTIVE` then placeholder (degraded, not wrong).
- An exact-title miss on a multi-window app yields the placeholder rather
  than a (possibly-correct) guess — a deliberate correct-or-nothing trade.

### Neutral

- `ATSPI_STATE_ACTIVE` logic is retained as the tiebreaker, not removed.
- The Component-extents geometry fallback (alt A) remains available if
  needed later.

## Verification

Probe (`examples/atspi_probe`) against the live Firefox process (one pid,
3 windows): the resolved menubar path tracks the supplied title —
`"… Outlook — Firefox Nightly"` → `/org/a11y/atspi/accessible/26`,
`"Claude — Firefox Nightly"` → `/org/a11y/atspi/accessible/239`. Identical
paths under the old app-root behaviour; distinct under this ADR. Pure
policy covered by `choose_frame` / `title_matches` unit tests.

## Cross-references

- ADR-0004 (resolve-by-PID — amended: PID selects the app, title selects
  the window)
- ADR-0024 (AT-SPI substrate)
- ADR-0028 (FR-003 accelerator deferred — the prior, unavailable fix)
- Spec: `specs/016-wrong-window-routing/spec.md`
- Drift triggers: CLAUDE.md trigger-I (this is the owed redesign).
