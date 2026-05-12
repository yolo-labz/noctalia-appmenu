# Implementation plan: bridge completion (Lane A)

**Spec:** `specs/005-bridge-completion/spec.md`
**Parent plan:** `specs/004-project-completion/plan.md`
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

## Approach

Nine FRs across two source files (`niri.rs`, `atspi.rs`) plus three
new files (`focus.rs`, `tests/niri_reconnect.rs`, the spec itself) plus
two deletions (`dbusmenu.rs`, `registrar.rs`). Implemented as
incremental commits — each ends with `cargo test --all-features
--locked` green so reviewers can bisect cleanly.

Strategy: deletions first (reduces the surface every subsequent commit
has to reason about), then FocusSink extraction (changes the type
plumbing every other Lane A file imports from), then per-FR fixes in
ADR-cited order.

The FocusSink trait uses `Pin<Box<dyn Future<Output = Result<()>> +
Send>>` return + `self: Sized` consumed receiver instead of
`#[async_trait]` macro. Rationale: avoids a new dependency while
staying compatible with `tokio::spawn` (the consumer requires
`Send + 'static`). RTN (return-type-notation) is not yet stable as of
Rust 1.81 — boxing the future is the smallest stable equivalent and
imposes ~0 runtime overhead for a single long-running spawn.

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | `FocusSink` is an *abstraction door*; the only implementor at v1 is `NiriFocusSink`. No Hyprland/Sway/KWin sink lands. |
| II — Sidecar by default | PASS | Lane A keeps bridge surface intact. No D-Bus / bus-name acquisition moves to QML. |
| III — Worktree-first git | PASS | All edits under `noctalia-appmenu-74-bridge-completion/`. Branch off `origin/main`. |
| IV — Conventional Commits + DCO | PASS | Every commit subject is `type(scope): summary` and DCO sign-off via `git commit -s`. |
| V — Speckit-driven | PASS | Sub-spec + plan + tasks live under `specs/005-bridge-completion/`. |
| VI — Release-engineering compliance | PASS | No workflow / SBOM / supply-chain change in Lane A. |
| VII — Graceful degradation | PASS | FR-004 (GTK4 empty → synthetic), FR-005 (bus restart → re-flip), FR-007 (stale path → typed error, not panic) all add new degradation paths instead of new error-out paths. |

All seven gates green.

## Architecture sketch

```
                 main.rs
                    │ owns NiriFocusSink, AtspiClient (Arc)
                    │
   ┌────────────────┴───────────────────┐
   │                                    │
focus.rs (FR-003)             atspi.rs (FR-004..007)
   │ FocusEvent, FocusOp,        │ AtspiClient (FR-006)
   │ FocusSink                   │ watch_a11y_status (FR-005)
   │                             │ do_action re-fetch (FR-007)
   ▼                             │ synthetic-on-empty (FR-004)
niri.rs (FR-001, FR-002)         │ normalize tests (FR-008)
   impl FocusSink for            │
   NiriFocusSink                 ▼
   run_once + ack-path test    active.rs
   backoff reset                  uses crate::focus::FocusEvent
                                  uses synthetic_menu on None
                                  writes source field (v=1.1)

  bridge/src/dbusmenu.rs ─── deleted (FR-009)
  bridge/src/registrar.rs ── deleted (FR-009)
```

## Affected files

### New

- `bridge/src/focus.rs` — `FocusSink` trait + `FocusEvent` +
  `FocusOp`.
- `bridge/tests/niri_reconnect.rs` — integration tests for FR-002.
- `specs/005-bridge-completion/spec.md` (this spec, see above)
- `specs/005-bridge-completion/plan.md` (this file)
- `specs/005-bridge-completion/tasks.md`

### Modified

- `bridge/src/lib.rs` — drop `mod dbusmenu` + `mod registrar`; add
  `mod focus`.
- `bridge/src/main.rs` — drop `registrar` import + task + `Cmd::Click`
  + `handle_click`; wire `NiriFocusSink` via `FocusSink::run`; spawn
  the `watch_a11y_status` task; construct + share `AtspiClient` via
  `Arc`.
- `bridge/src/niri.rs` — re-import `FocusEvent`/`FocusOp` from
  `crate::focus`; add `NiriFocusSink` struct + `impl FocusSink`; reset
  backoff to `BACKOFF_FLOOR` (250 ms) after ≥30 s connected session.
- `bridge/src/active.rs` — drop `MenuMap` arm from `snapshot`; drop
  `menus_rx` from `run`; import `FocusEvent` from `crate::focus`; emit
  `source` field; fall back to `synthetic_menu(app_id)` when AT-SPI
  walker returns `None` or empty children (FR-004).
- `bridge/src/atspi.rs` — `AtspiClient` struct (FR-006);
  `watch_a11y_status` task (FR-005); `do_action` re-fetch +
  `MenuError::Stale` (FR-007); GTK4 empty-children sentinel returned
  from `fetch_menubar_for_pid_inner` so `active.rs` synthesizes
  (FR-004); new `tests` for `normalize_app_id` covering Anki +
  KDE double-prefix + Anki subprocess launcher fuzzy-match (FR-008).
- `bridge/src/proxy.rs` — no functional change; only follow-on edits
  if compilation breaks from the schema field rename.

### Deleted

- `bridge/src/dbusmenu.rs`
- `bridge/src/registrar.rs`

## Risks

- **R-A1** Deleting registrar.rs leaves `ActiveSnapshot::menu_service`
  and `.menu_path` as forever-empty fields. *Mitigation*: keep the
  fields (proxy.rs publishes them as D-Bus properties; QML still reads
  them); they go cold but the D-Bus contract stays. Removal scheduled
  for a coordinated schema-v2 PR in v1.1.
- **R-A2** `Cmd::Click` was reachable from QML when DBusMenu substrate
  was live (v0.2). After delete, any old QML caller invoking
  `noctalia-appmenu-bridge click ...` exits with "unrecognised
  subcommand". *Mitigation*: the spec-003 QML migration (PR #57) moved
  callers to `atspi-click`. Reviewing git log confirms no remaining
  `click` references. Hard-fail on launch is louder than silent no-op.
- **R-A3** `watch_a11y_status` adds a third long-running task to
  `main.rs`. *Mitigation*: it is structured identically to the existing
  niri / active / proxy tasks; uses the same `tokio::select!` shutdown
  primitive; failures are logged + non-fatal (the existing focus task
  already keeps running without IsEnabled monitoring).
- **R-A4** `AtspiClient` introduces a shared `Mutex` around `zbus::Connection`.
  zbus::Connection internally is `Arc<...>` so we never block long, but
  *if* two focus events fire rapidly the Mutex serialises connect
  attempts. *Mitigation*: connection setup is rare (lazy fill, then
  hot for hours); the contended path is `.clone()` of the inner
  Connection which is cheap.
- **R-A5** Spec 004 contract `active-json-schema.md` shows `pid` (not
  `focus_pid`) and omits `menu_service`/`menu_path`. *Mitigation*:
  Lane A keeps existing field names + adds `source` only. The rename
  is documented as a future coordinated change. Spec 005 §Assumptions
  §1 records the decision.

## Rollout

Single PR per Lane A (this one). Merged before Lane B (#75) starts so
Lane B's `SubmenuPopup.qml` can consume the v=1.1 `source` field
without forward-reference. Lane D (CI integration test) depends on
Lane A's deliverable; it merges last.

## Open questions

1. **`async_trait` vs boxed-future return**: chosen boxed-future to
   avoid a new dependency. If a future Lane A maintainer prefers the
   macro, adding `async-trait = "0.1"` to `[dependencies]` would be a
   one-line dep bump. Decision: stay dep-free at v1.
2. **`watch_a11y_status` retry**: currently the task logs + exits on
   signal subscription failure. Should it retry indefinitely?
   Decision: yes, with exponential backoff; if signal stream fails, we
   risk missing a bus restart. Implemented in `atspi::watch_a11y_status`
   with the same backoff philosophy as `niri::run`.
3. **`Cmd::Click` removal**: removing the subcommand is a CLI-surface
   break. Constitution principle VII (graceful degradation) suggests
   keeping a no-op stub that prints a deprecation message. Decision:
   delete cleanly — DBusMenu substrate has been off the v0.3 hot path
   for 30+ days, and no documented integrations call it now.
