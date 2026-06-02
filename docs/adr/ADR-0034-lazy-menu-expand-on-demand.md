# ADR-0034 — Expand-on-demand for lazily-realized menus (Firefox/GTK)

- **Status:** accepted
- **Date:** 2026-06-02
- **Deciders:** Pedro H S Balbino
- **Related:** ADR-0024 (AT-SPI substrate), ADR-0029 (learned-skip), ADR-0030 (frame-scoped resolution)
- **Closes:** spec 013 FR-003 / SC-003 residue (the Firefox lazy-realization gap, R2)

## Context

Pedro reported (02/06/2026, Firefox/YouTube): clicking a top-level menu in
the bar makes "the menubar vanish and nothing happens." The journal showed
the real cause — not a popup/dismiss bug:

```
[appmenu] click on top-level: File children: 0
[appmenu] empty top-level — triggering RefreshActive retry: File
[appmenu] click on top-level: Edit children: 0 …
```

**Firefox (and GTK) expose a top-level menu over AT-SPI with zero children
until the menu is actually opened.** The bridge's passive walk
(`fetch_menu_tree`) therefore serializes `File`/`Edit`/`View` with empty
`children`, and the plugin renders an empty popup. The pre-existing
"self-heal" (`RefreshActive` → re-walk) re-walks *passively* → still zero,
then falls back to a native `DoAction` on the top-level (opens Firefox's
own menu at the Firefox window, not bar-anchored — evidently a no-op for
the user). This is the residue spec 013 FR-003 / SC-003 named and deferred.

## Measurement (live, 2026-06-02, Firefox 153 on niri)

Walking Firefox's `File` menu accessible:

- `childCount` **before** = `0`; the node has `NActions = 1`, `action[0] = "click"`.
- `DoAction(0)` returns `true` → `childCount` **after** = **17** (New Tab,
  New Window, New Private Window, Open File…, Save Page As…, …).
- **Children + their object paths persist after collapse.** Capturing a
  child path (`/org/a11y/atspi/accessible/660`, "New Tab"), then
  `DoAction(0)` to collapse, then `GetRole` on the child → still
  `menu item`; `File.childCount` stays `17`.

So a realized subtree is **click-safe after collapse** and realization is
**one-time per menu**. (Verified again end-to-end through the new
subcommand: `atspi-expand … → 17` items, each with a live `(service,
path)`.)

## Decision

Add a bridge subcommand **`atspi-expand <service> <path> [--winid] [--focus-settle-ms]`**
(`atspi::expand_and_fetch`): pre-focus the captured niri window, fire the
menu's `"click"` action to **expand**, wait `EXPAND_REALIZE_DELAY` (150 ms)
for realization, **walk** the now-realized subtree (`fetch_menu_tree`),
fire `"click"` again to **collapse**, and print the realized `children` as
a JSON array on stdout. Stale path → exit 2 + `MenuError::Stale` (same as
`atspi-click`).

The QML widget (`BarWidget.qml`), on an empty top-level group that carries
a real `(service, path)`, spawns `atspi-expand` and renders the returned
children in its **bar-anchored** popup. Clicking a child routes through the
existing `atspi-click` path — the realized leaf paths are live (persist),
so the click fires.

This replaces the old `RefreshActive`-retry + `retryTimer` machinery for
empty top-levels (it was passive and could not realize lazy children).

## Alternatives rejected

- **Eager-expand every top-level on focus/walk** — would flash *all* of
  Firefox's menus open+closed on first focus. On-click-one is minimal.
- **Just open Firefox's native menu** (`DoAction` the top-level, no mirror)
  — functional but not bar-anchored, and defeats the global-menu UX. It
  remains the last-resort fallback when expand yields nothing.
- **`org.gtk.Menus` / dbusmenu substrate** — measured dead on niri
  (ADR-0032); Firefox's native dbusmenu path is inert here (ADR-0033 note).

## Consequences

- **Positive:** Firefox/GTK lazy menus now populate + click correctly,
  bar-anchored. Closes the long-standing FR-003/SC-003 gap with evidence.
- **Negative / cost:** the on-click expand briefly opens the *real* Firefox
  menu (~150 ms) before the bar popup shows — a short visible flash, only
  on the first interaction with a given menu (children persist after).
  Accepted: a brief flash beats "click does nothing." There is no AT-SPI
  way to realize children without opening (the only action is `"click"`).
- **Minor race:** the best-effort collapse toggles `"click"`; if Firefox
  auto-closed the menu first (focus drift), the toggle could re-open it.
  Guarded by only collapsing when the walk saw children; rare and
  self-corrects on the next interaction.

## Verification

- Bridge: `atspi::expand_and_fetch` unit-compiles; live `atspi-expand`
  returns 17 click-safe items for Firefox `File`. `cargo test` + `clippy
  -D warnings` green.
- Plugin: `qmllint` clean; `StdioCollector` is the API noctalia-shell
  itself uses.
- End-to-end on the live desktop is gated behind the release Pedro runs.
