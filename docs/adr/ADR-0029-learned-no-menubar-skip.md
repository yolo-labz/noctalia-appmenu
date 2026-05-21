# ADR-0029 — Learned no-menubar skip replaces hardcoded list

- **Status:** accepted
- **Date:** 2026-05-21
- **Deciders:** Pedro H S Balbino
- **Supersedes:** none (the skip-list was an undocumented impl detail)
- **Tracking PR / branch:** `129-atspi-learned-skip`

## Context

ADR-0024 made the bridge walk a focused app's AT-SPI accessibility
tree looking for a `MENU_BAR` (role 34) node, then re-export it to the
fixed proxy. Apps that publish no menu fall into two runtime classes
that look identical to the caller — both return `Ok(None)` — but cost
wildly different wall-clock time:

- **On-bus, no menubar** (Chrome, Chromium, GTK apps with no menu):
  the app *is* on the a11y bus, so the depth-bounded DFS finds the
  root, sees no `MENU_BAR`, and returns in **< 50 ms**.
- **Not on the a11y bus** (terminals — ghostty, alacritty, kitty,
  foot — and `xwayland-satellite`): there is no connection to query,
  so the walk scans every registrar entry and exhausts the
  `FETCH_BUDGET` (up to ~2500 ms) before giving up.

The not-on-bus class drained the budget on **every focus change**,
freezing the bar each time the user alt-tabbed to a terminal. The
original mitigation was a hardcoded `KNOWN_NO_MENUBAR_APPS` const plus
`is_known_no_menubar()`: skip the walk entirely for listed `app_id`s.

That list is maintenance debt. Every newly observed no-menubar app
(chromium-browser, google-chrome, the next terminal) required a
source edit, a review, and a release before the freeze stopped. When
offered "add chromium + google-chrome to the skip-list", Pedro's
directive was explicit:

> I would prefer that the skips are automatic and not list dependent.

## Decision

**Replace the static list with a skip verdict learned at runtime from
the observed walk cost.** The cost-split above is the discriminator
the hardcoded list was a proxy for — so measure it directly instead of
enumerating app_ids.

`atspi::learned_skip` keeps a process-lifetime `(app_id → Verdict)`
map, where `Verdict { learned_at: Instant, expensive: bool }`:

- `EXPENSIVE_WALK = 750 ms` — a no-menubar walk slower than this
  drained most of the budget, i.e. the app is not on the a11y bus.
- `RECHECK_TTL = 300 s` — how long a *cheap* no-menu verdict
  suppresses re-walks.

Decision logic (`proxy.rs` tier-1, before the `(app_id, pid)`
menu-cache and before any walk):

1. Walk returns a **menubar** → `forget(app_id)`. Never skip an app
   that has a real menu.
2. Walk returns **no menu** → `record_negative(app_id, elapsed)`;
   `expensive = elapsed >= EXPENSIVE_WALK`.
3. Walk returns **transient `Err`** → record nothing (a hung bus or a
   mid-launch app is not a "no menubar" signal).
4. On the next focus, `should_skip(app_id)` returns true when
   `expensive || age < RECHECK_TTL`.

So an **expensive** verdict skips for the rest of the process's life
(a not-on-bus app will not grow a menubar mid-flight), while a
**cheap** verdict expires after the TTL and triggers exactly one
re-walk — which lets a lazily-built menubar (some Electron/GTK apps
register their a11y tree after first paint) self-heal into the bar.

## Why these knobs

- **750 ms threshold.** Healthy on-bus apps answer in < 50 ms; the
  budget-drain class is ≫ 750 ms. The gap between the two classes is
  ~1.5 orders of magnitude, so the exact threshold is not delicate.
  The asymmetry of misclassification is what makes the heuristic safe:
  a cheap app *misclassified* expensive only loses lazy self-heal
  (harmless — it has no menu anyway), while the reverse cannot happen
  because a not-on-bus app has no menu to lazily build.
- **300 s TTL on cheap verdicts only.** Re-walking a cheap no-menu app
  every 5 minutes costs one < 50 ms walk and catches a late-registering
  menubar. Expensive verdicts are never re-checked because doing so
  would re-introduce the budget-drain freeze this ADR exists to kill.
- **In-memory, process-lifetime.** No persistence: a bridge restart
  re-learns each verdict on first focus at negligible cost, and the
  cost-split is environment-dependent (a11y bus state) so a stale
  on-disk verdict would be a liability, not a saving.

## Alternatives considered

### A. Add chromium + google-chrome to the list

The rejected starting point. Stops *one* freeze but perpetuates the
maintenance debt and does not generalise to the next app. Directly
contradicts the directive.

### B. Probe a11y-bus membership directly

Ask AT-SPI whether the focused pid owns an a11y connection before
walking. A "more correct" signal, but it needs a reliable
pid → a11y-connection map and an extra D-Bus round-trip on every
focus. The cost-split derives the same answer for free from the walk
the bridge already performs — no new round-trip, no new map.

### C. Static timeout, no learning

Keep the per-walk `FETCH_BUDGET` but drop the skip entirely. Every
terminal focus still pays one full budget drain before bailing; the
bar freezes on every alt-tab. Learning amortises that to one drain per
process per app.

## Consequences

### Positive

- Zero maintenance: a newly observed no-menubar app auto-skips after a
  single observation; no source edit, no release.
- Freeze protection (the reason the list existed) is preserved
  mechanically rather than by an enumerated allow-list.
- Lazily-built menubars self-heal via the cheap-verdict TTL.

### Negative

- The **first** focus of a not-on-bus app still drains the budget once
  — unavoidable, because that drain *is* the signal being measured.
  Subsequent focuses of the same app within the process are instant.
- The verdict map is lost on bridge restart (re-learned cheaply).

### Neutral

- The `(app_id, pid)` menu-cache (30 s TTL, ADR-0023 fetch-on-focus
  lineage) is untouched — it is an orthogonal positive-result cache
  layered above this negative-result skip.

## Cross-references

- ADR-0023 — dbusmenu fetch-on-focus (positive-cache lineage)
- ADR-0024 — AT-SPI substrate (the walk whose cost is measured here)
- Code: `bridge/src/atspi.rs::learned_skip`, `bridge/src/proxy.rs`
  tier-1 decision block
