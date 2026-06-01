# ADR-0033 — Liveness-keyed caches must self-heal (no permanent verdict on a recoverable condition)

- **Status:** accepted
- **Date:** 2026-06-01
- **Deciders:** Pedro H S Balbino
- **Related:** ADR-0029 (learned no-menubar skip), ADR-0024 (AT-SPI substrate), ADR-0030 (frame-scoped resolution)
- **Fixes:** [#174](https://github.com/yolo-labz/noctalia-appmenu/issues/174) — `learned_skip` expensive verdict never cleared when an app gained AT-SPI later

## Context

ADR-0029 introduced `learned_skip` in `bridge/src/atspi.rs`: a per-`app_id`
verdict that suppresses the AT-SPI menubar walk for apps shown not to have
a usable menubar. Verdicts are classified by walk cost:

- **cheap** negative (on-bus app, lazily-built/absent menu — GTK4 popover,
  Chrome hamburger): expires after a short `RECHECK_TTL` (300 s) and
  re-walks, so a lazily-built menubar self-heals.
- **expensive** negative (the walk drained `FETCH_BUDGET` — the app is not
  on the a11y bus: terminal, xwayland, or a11y disabled): was held
  **permanently for the life of the bridge process**.

The permanent verdict was a bug. On 31/05/2026 Pedro enabled Firefox's
AT-SPI export (`accessibility.force_disabled` 1 → 0) and restarted Firefox.
The already-running bridge had learned an *expensive* verdict for
`firefox-nightly` while Firefox was off-bus, so it kept serving the
`.desktop` fallback (`active.json` `source:"desktop-fallback"`) even though
Firefox now exposed a full menubar (`frame → tool bar "Menu Bar" → menu bar
→ File/Edit/View/…`). Only a `systemctl --user restart
noctalia-appmenu-bridge.service` cleared it. An hour of live debugging
traced the symptom to the permanent verdict, not to Firefox.

"App is not on the a11y bus" is a **recoverable** condition — an off-bus app
can come on-bus at any time (a pref flip, a late-starting AT, an a11y-bus
restart). A permanent verdict on a recoverable condition strands the app
forever.

## Decision

**Invariant (project-wide): any cache or verdict keyed on app/connection
liveness MUST self-heal. Never record a permanent verdict on a recoverable
condition.** A liveness-keyed verdict gets either:

1. a finite TTL after which it is re-checked, **and/or**
2. an explicit `forget()` on a positive re-observation (the app/connection
   reappears).

**Split the staleness decision into a pure function** so a regression test
probes the boundary by passing an age, without sleeping the TTL. In this
repo the pattern is `learned_skip::skip_decision(expensive, age)` +
`classify_expensive(walk)` + `ttl_for(expensive)` — each pure, each
unit-tested at the boundary.

### Applied to `learned_skip` (the #174 fix)

- Expensive verdicts now expire after a finite `EXPENSIVE_RECHECK_TTL`
  (1800 s = 30 min), not permanently. `skip_decision` is now a trivial
  `age < ttl_for(expensive)`.
- The TTL is *longer* than the cheap one because re-walking an off-bus app
  drains `FETCH_BUDGET` and briefly stalls the bar — so the worst case is a
  ~twice-an-hour re-stall per genuinely-off-bus app (a terminal), which is
  acceptable; the previous permanent verdict was not.
- Additional positive-re-observation vector: `clear_expensive()` drops all
  expensive verdicts when the a11y bus is observed restarting
  (`watch_a11y_status` `IsEnabled` flip), so a bus restart self-heals
  immediately rather than waiting out the TTL.
- The Firefox `force_disabled` flip case is invisible to the bridge (a
  Firefox-internal pref, no bus signal), so it self-heals via the TTL
  backstop (≤ 30 min) or an explicit bridge restart — both now documented
  in the README Firefox caveat.

## This is a recurring class, not a one-off

The same **connected-but-dead** failure mode appears across the yolo-labz
fleet (e.g. the `wa` WhatsApp daemon's session liveness): a peer that was
absent/dead is cached as such and never re-probed after it recovers.
Encoding the invariant here — and the pure-function-boundary test pattern —
is meant to stop the next agent re-introducing a permanent verdict under
iteration pressure.

## Consequences

- **Positive:** an app that gains a11y self-heals without a bridge restart
  (≤ 30 min worst case; immediate on a11y-bus restart). #174 closed.
- **Positive:** the boundary is unit-tested without sleeping — fast, sleep
  -free, deterministic regression coverage (`skip_decision` /
  `ttl_for` / `classify_expensive` tests in `atspi.rs`).
- **Negative / cost:** a genuinely-off-bus app (terminal, xwayland) incurs
  one expensive re-walk per `EXPENSIVE_RECHECK_TTL` instead of never. The
  bar may stall for up to `FETCH_BUDGET` on that one focus, ~twice an hour.
  Acceptable — and far cheaper than the support cost of "the menu vanished
  and only a restart fixes it".

## Note — Firefox ≥ 138 native global menu (does not change this decision)

Verified 01/06/2026 against the Firefox source tree (`mozilla-firefox/cedar`,
`widget/gtk/NativeMenuGtk.cpp` + `DBusMenu.cpp`) and Bugzilla 1883184 /
1956707: Firefox ≥ 138 ships a *native* global menu that exports
**`com.canonical.dbusmenu`** (built via libdbusmenu — **not** `org.gtk.Menus`
/ GMenuModel). It is **opt-in** (`widget.gtk.global-menu.enabled`,
`widget.gtk.global-menu.wayland.enabled`, `widget.gtk.native-context-menus`
all default `false`), and on Wayland it additionally requires the compositor
to bind `org_kde_kwin_appmenu_manager` **and** a `com.canonical.AppMenu.Registrar`
owner to be present. **niri binds neither and ships no registrar, so the
native path is a no-op on niri** — Firefox puts nothing on the session bus,
and AT-SPI (`accessibility.force_disabled = 0`, ADR-0024) remains the only
menu surface for Firefox on niri, at every version. This does **not** reopen
ADR-0032's `org.gtk.Menus` rejection (Firefox's native path is dbusmenu, not
GMenuModel, and is inert on niri regardless). It is recorded so a future
agent does not "discover" the FF-138 prefs and wrongly swap the substrate.
