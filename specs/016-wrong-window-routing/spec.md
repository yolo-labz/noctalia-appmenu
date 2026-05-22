# Spec 016 — Wrong-window menu routing (same-PID multi-window)

- **Status:** implemented (branch `132-wrong-window-routing`)
- **Date:** 2026-05-21
- **Author:** Pedro H S Balbino
- **Drift trigger:** I (user-reported failure persists across ≥ 2 deploys)
- **ADR:** [ADR-0030](../../docs/adr/ADR-0030-frame-scoped-menu-resolution.md)

This is the **redesign spec owed by CLAUDE.md drift-trigger-I**. It cites
every prior patch on this symptom, explains why each failed, then
specifies the resolution-layer fix.

## Problem

A process with several top-level windows (Firefox: one process, N windows;
okular: N PDFs; LibreOffice: Writer + Calc) reports one `pid` to niri.
Focusing one window — or opening a new tab in it — showed a **different
window's menu** in the bar. Pedro re-reported this on 2026-05-21 against
v1.0.24: *"When I open a new tab in firefox it still opens on the wrong
instance."*

## Prior attempts (why each failed)

| Tag | Patch | Axis | Why it did not fix the symptom |
|---|---|---|---|
| v1.0.20 | focus settle | timing | Delayed the walk; the walk still resolved by PID → app root → first menubar. Window identity never entered the decision. |
| v1.0.21 | focus telemetry | observability | Added logging only. No behaviour change. |
| v1.0.22 | 150 ms focus-settle floor | timing | Debounced rapid focus flips; the resolved *window* was still arbitrary for a same-PID app. |
| (deferred) ADR-0028 | FR-003 accelerator dispatch | input routing | The intended structural fix, but niri-ipc exposes no keyboard-input synthesis primitive — deferred, not shipped. |

Every shipped patch was on the **timing/settle axis**. None touched the
**resolution axis** — `find_app_for_pid` returning the AT-SPI application
root on a PID match, with `find_menubar` then first-hit-DFS-ing an
arbitrary window's `MENU_BAR`. That is the actual defect.

## Root cause (codex-confirmed against AT-SPI source)

- `bridge/src/atspi.rs` `find_app_for_pid` pass 1 returned the app root and
  short-circuited; frame-scoping (`find_active_app_via_state`) was stranded
  in pass 2 (PID-miss path) so native-Wayland Firefox never reached it.
- `find_menubar` first-hit DFS from the app root picks an arbitrary window.
- The menu cache keyed `(app_id, pid)` reused one window's menu for another.
- `STATE_ACTIVE` is not deterministic for same-PID multi-window
  (at-spi2-core `constants.h`); no AT-SPI API maps a frame to a window id;
  the frame's accessible **Name (= window title)** is the only stable
  discriminator.

## Requirements

- **FR-016-1** Menu resolution MUST scope to the niri-focused window's
  AT-SPI frame, not the application root, for multi-window processes.
- **FR-016-2** The focused-window **title** MUST be the primary
  discriminator (exact, trimmed); `STATE_ACTIVE` is a tiebreaker only.
- **FR-016-3** A multi-window app whose focused frame cannot be identified
  MUST serve the placeholder (`None`), never an arbitrary window's menu.
- **FR-016-4** Single-window and zero-frame apps MUST be unchanged.
- **FR-016-5** The menu cache MUST be keyed by `(app_id, pid, winid)`.

## Approach

Frame-scoped resolution per ADR-0030: thread niri `{winid, title}`
(already on `ActiveSnapshot`) into `fetch_menubar_for_pid`; pass 1 collects
window frames and selects via the pure `choose_frame` policy
(title → STATE_ACTIVE → placeholder); re-key the cache by `winid`.

Files: `bridge/src/atspi.rs` (resolution, cache key, `choose_frame` +
`title_matches` + unit tests), `bridge/src/proxy.rs` (thread title + winid).

## Verification

- Unit: `choose_frame` (single/title/active/none) + `title_matches`.
- Runtime falsification (passed): one Firefox pid, two window titles →
  distinct menubar frame paths (`/…/26` vs `/…/239`); identical under the
  old behaviour. See ADR-0030 §Verification.
- **Ship gate (trigger-I):** do NOT tag/deploy a release until Pedro
  confirms on his desktop that focusing each Firefox window shows that
  window's menu. Prior patches shipped on plausibility and recurred; this
  one ships only on user confirmation.

## Out of scope

Timing/settle changes; FR-003 accelerator (still deferred, ADR-0028);
Component-extents geometry fallback (ADR-0030 alt A, deferred).
