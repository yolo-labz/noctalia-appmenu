# Specification: Bridge completion (v1.0.0 Lane A)

**ID:** 005-bridge-completion
**Parent:** [004-project-completion](../004-project-completion/spec.md)
**Created:** 2026-05-12
**Author:** @phsb5321 (Lane A worker)
**Constitution version:** 1.0.0

## Why

Lane A of spec 004's four-lane split. The umbrella spec already derived
requirements + ADR refs (FR-001..FR-009); this sub-spec restates the
Lane A surface, defines the acceptance criteria, and lists the affected
files so the PR reviewer can audit traceability in one place.

The bridge audit (`research.md` §1, §2) surfaced five user-visible
defects (backoff regrowth, ack-path untested, GTK4 empty menu bars,
AT-SPI bus restart not recovered, click race against rebuilt widget
trees), one architectural cleanup (compositor abstraction door), and
one substrate-retirement chore (delete `dbusmenu.rs` + `registrar.rs`).
Landing these is necessary for spec 004 §SC-001/SC-002/SC-005/SC-007 to
go green at the v1.0.0 tag.

## User scenarios

Inherits scenarios 1, 2, 5, 6 from spec 004 verbatim. Quick recap:

- **Scenario 1** Fresh-NixOS install delivers Anki menubar (Anki PID
  is the wrapper script, not Qt — Lane A's FR-008 fuzzy-match
  guarantees pass-3 picks up the right Application).
- **Scenario 2** Focus tracking between three Qt6 apps via Mod+Tab
  (Lane A's FR-006 persistent connection keeps the walker hot).
- **Scenario 5** AT-SPI bus crash + restart recovery (Lane A's FR-005
  re-flips `IsEnabled = true` on the new bus instance).
- **Scenario 6** `niri msg reload-config` doesn't compound backoff
  (Lane A's FR-001 resets to floor after ≥30 s connected session).

Scenarios 3 (nested submenus), 4 (checkable items), and 7 (release
artefact verify) are owned by Lanes B and D respectively.

## Functional requirements

Restated from spec 004 §Bridge focus tracker + §Bridge AT-SPI walker.

### Focus tracker (`bridge/src/niri.rs` + new `bridge/src/focus.rs`)

- **FR-001** Backoff resets to floor (`250 ms`) after any cleanly-EOF'd
  niri session of duration ≥ 30 s. Verifiable via wall-clock measurement
  around `run_once` and a unit test driving three EOF cycles.
- **FR-002** The connect/handshake/ack path carries an integration test
  asserting a successful `Response::Handled` ack is parsed AND that a
  malformed ack returns `Err`, not silent backoff.
- **FR-003** A `FocusSink` trait + the `FocusEvent` / `FocusOp` types
  live at `bridge/src/focus.rs`; `niri.rs` provides one concrete
  implementor (`NiriFocusSink`). No compositor-specific type leaks
  beyond `niri.rs`. The trait is an abstraction *door* — v1 ships
  niri-only per constitution principle I; Hyprland/Sway implementors
  arrive in v2.

### AT-SPI walker (`bridge/src/atspi.rs`)

- **FR-004** GTK4 `GtkPopoverMenuBar` quirk: when the walker finds a
  `MENU_BAR` accessible whose post-walk subtree is empty, falls back
  to `synthetic_menu(app_id)` (the `.desktop`-derived pseudo-menu).
  Documented as `source = "synthetic"` in the active.json v=1.1 schema.
- **FR-005** The bridge subscribes to `org.a11y.Status` `PropertiesChanged`
  on the session bus and re-invokes `enable_a11y()` whenever `IsEnabled`
  flips to `false` — including after a fresh a11y bus instance comes
  up post-crash. Scenario 5 is the acceptance gate.
- **FR-006** AT-SPI connection lifetime: `AtspiClient` owns a long-lived
  `zbus::Connection` cached behind a `tokio::sync::Mutex<Option<Connection>>`.
  Per-focus-event walks reuse it. On bus restart the cache is invalidated
  and re-established lazily on next access.
- **FR-007** `do_action(service, path)` re-fetches the addressed
  accessible (`GetRole` round-trip) before invoking `DoAction(0)`. A
  stale path returns `MenuError::Stale` (typed error variant) with
  exit-code 2 from the `atspi-click` subcommand. Lane B (plugin) reacts
  to exit-code 2 by re-reading active.json on its next focus tick.
- **FR-008** App-matching covers the three v1 reference apps:
  - **Anki** via subprocess-launcher PID — pass-3 fuzzy match
    (`normalize_app_id("anki")` vs `normalize_app_id("anki.bin")` →
    short-contained-in-long ≥ 3 chars) succeeds.
  - **kate / dolphin** via KDE double-prefix — `normalize_app_id`
    round-trip strips `org.kde.` and returns the bare app name.
  - Both have unit tests in `atspi.rs::tests`; no real AT-SPI bus
    required.
- **FR-009** Delete `bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs`.
  Remove module declarations from `lib.rs`. Update `main.rs` to drop the
  registrar task spawn + retired `Cmd::Click` subcommand (DBusMenu
  substrate retired per ADR-0024). Update `active.rs` to drop the
  `MenuMap` join arm.

## Schema change

`active.json` bumps minor schema rev: `v = 1` payload gains a new
top-level `source: "atspi" | "synthetic" | "empty"` field. Documented
in [contracts/active-json-schema.md](../004-project-completion/contracts/active-json-schema.md).
Lane B (plugin) consumes this in the same release. Wire-compat
guarantee: consumers that ignore the field still parse the rest of the
payload (additive field).

## Constraints / dependencies

- **MSRV** preserved at Rust 1.81 (per `bridge/Cargo.toml`).
- **No new dependencies** — `FocusSink` trait uses
  `Pin<Box<dyn Future + Send>>` to stay dyn-compatible without
  `async-trait`.
- **Worktree-first.** All Lane A edits live in
  `noctalia-appmenu-74-bridge-completion/`. Branch off `origin/main`.
- **DCO sign-off + Conventional Commits** enforced by lefthook.
- **No PR creation.** Parent reviews and opens the PR.

## Out of scope (deferred to v2 or other lanes)

- Hyprland / Sway / KWin focus tracker implementors (constitution
  principle I — door only at v1).
- `children-changed` AT-SPI subscription full implementation (FR-006
  lays the prerequisite; subscription itself is a separate spec).
- Snapshot refresh triggered by atspi-click stale-error — Lane B owns
  the plugin-side reaction; Lane A ships the typed error + exit code.
- AT-SPI integration test in CI (Lane D — FR-022).
- Plugin / Nix / docs surfaces (Lanes B / C / D).
- Cognitive-complexity refactor of `find_app_for_pid` / `fetch_menu_tree`
  (spec 004 FR-027, deferred to ADR-0025 if needed).

## Success criteria

- **SC-A1** `cargo test --all-features --locked` passes on the lane
  branch.
- **SC-A2** `cargo clippy -- -D warnings` clean.
- **SC-A3** `cargo fmt --check` clean.
- **SC-A4** `bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs` no
  longer exist; `git grep -E 'dbusmenu|registrar::|MenuMap'` returns no
  matches under `bridge/src/`.
- **SC-A5** `bridge/src/focus.rs` exists; `niri.rs` implements
  `FocusSink`; `active.rs` + `main.rs` import `FocusEvent` from
  `crate::focus`, not from `crate::niri`.
- **SC-A6** New tests cover FR-001 (backoff reset), FR-002 (ack-path
  happy + malformed), FR-008 (normalize_app_id Anki + KDE).
- **SC-A7** `active.json` payload contains `"source": "<atspi|synthetic|empty>"`
  on every write.

## Key entities

- **`FocusEvent`** — `{winid: u64, pid: u32, app_id: String, title: String}`.
  Moved from `niri.rs` to `focus.rs`. Wire shape unchanged from v0.3.
- **`FocusOp`** — `{Emit(FocusEvent), Defocus, NoChange, UnknownWindow(u64)}`.
  Moved from `niri.rs` to `focus.rs`.
- **`FocusSink`** — `fn run(self, tx, cfg) -> Pin<Box<dyn Future<...> + Send>>
  where Self: Sized`. The abstraction door.
- **`NiriFocusSink`** — zero-state struct implementing `FocusSink`;
  delegates to the existing `niri::run` free function.
- **`AtspiClient`** — owns a `Mutex<Option<Connection>>`, exposes
  `connection()` (lazy fill) + `invalidate()` (reset on bus restart).
  Owned by `main.rs`, cloned via `Arc` into the active loop + the
  IsEnabled monitor.
- **`MenuError`** — typed error enum: `Stale { service, path }` +
  catch-all variants. Returned from `do_action` on `GetRole` failure.

## Assumptions

- The contract `active-json-schema.md` v=1.1 (additive `source` field)
  is accepted by the Lane B worker without rename of `focus_pid` → `pid`
  or removal of legacy `menu_service` / `menu_path` top-level fields.
  Renames are deferred to a coordinated v=2 schema bump.
- Lane B (plugin) reacts to atspi-click exit-code 2 by treating the
  click as a no-op + scheduling a re-fetch on next focus tick. Lane A
  ships the typed error + exit code only; the plugin side is documented
  as a hand-off.
