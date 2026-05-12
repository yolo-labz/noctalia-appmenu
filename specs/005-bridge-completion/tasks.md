# Tasks: bridge completion (Lane A)

**Spec:** `specs/005-bridge-completion/spec.md`
**Plan:** `specs/005-bridge-completion/plan.md`
**Cap:** ≤25 tasks (constitution Development Workflow §3).

Each task ends with `cargo test --all-features --locked` green and a
DCO-signed Conventional Commits commit.

| # | Task | FR | File(s) |
|---|---|---|---|
| T-01 | Write spec.md + plan.md + tasks.md for sub-spec 005 | — | `specs/005-bridge-completion/*` |
| T-02 | Delete `bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs`; drop `mod` declarations from `lib.rs` | FR-009 | `bridge/src/lib.rs`, `bridge/src/dbusmenu.rs` (del), `bridge/src/registrar.rs` (del) |
| T-03 | Drop registrar task spawn + `Cmd::Click` + `handle_click` from `main.rs`; drop `MenuMap` arm from `active.rs` | FR-009 | `bridge/src/main.rs`, `bridge/src/active.rs` |
| T-04 | Create `bridge/src/focus.rs` with `FocusEvent`, `FocusOp`, `FocusSink`; export under `pub mod focus` in `lib.rs` | FR-003 | `bridge/src/focus.rs` (new), `bridge/src/lib.rs` |
| T-05 | Move `FocusEvent` + `FocusOp` from `niri.rs` to `focus.rs`; introduce `NiriFocusSink` implementing `FocusSink`; rewire `main.rs` to call `NiriFocusSink::new().run(...)` | FR-003 | `bridge/src/niri.rs`, `bridge/src/main.rs`, `bridge/src/active.rs` |
| T-06 | Reset backoff to `BACKOFF_FLOOR` (250 ms) after a `run_once` session of wall-clock duration ≥ 30 s. Unit test in `niri::tests::backoff_reset_after_long_session` | FR-001 | `bridge/src/niri.rs` |
| T-07 | New `bridge/tests/niri_reconnect.rs` — integration test using `UnixListener` fixture: drives `run_once` happy-path ack + malformed ack (asserts `Err`) | FR-002 | `bridge/tests/niri_reconnect.rs` (new) |
| T-08 | Introduce `AtspiClient { conn: Mutex<Option<Connection>> }`; `connection()` lazy-fill, `invalidate()` reset. Migrate `fetch_menubar_for_pid` to take `&AtspiClient`; main.rs constructs Arc once and shares | FR-006 | `bridge/src/atspi.rs`, `bridge/src/active.rs`, `bridge/src/main.rs` |
| T-09 | `watch_a11y_status` task: subscribe to PropertiesChanged on `org.a11y.Bus`/`org.a11y.Status` on session bus; on `IsEnabled=false` re-call `enable_a11y()` and invalidate the `AtspiClient` cache | FR-005 | `bridge/src/atspi.rs`, `bridge/src/main.rs` |
| T-10 | `do_action` re-fetches addressed accessible via `GetRole`; introduce `MenuError::Stale { service, path }` (typed enum); on stale, the `atspi-click` CLI exits 2 with stderr message. Unit test covers `MenuError` Display | FR-007 | `bridge/src/atspi.rs`, `bridge/src/main.rs` |
| T-11 | `fetch_menubar_for_pid_inner` returns `None` when the post-walk `MenuItem` has empty `children` (GTK4 `GtkPopoverMenuBar` quirk). `active.rs` falls back to `synthetic_menu(app_id)` on `None` with `source = "synthetic"` | FR-004 | `bridge/src/atspi.rs`, `bridge/src/active.rs` |
| T-12 | Unit tests for `normalize_app_id`: `org.kde.kate` → `kate`, `org.kde.dolphin` → `dolphin`, Anki subprocess-launcher fuzzy-match (`anki` ⊂ `anki.bin` → match) | FR-008 | `bridge/src/atspi.rs` |
| T-13 | Bump `active.json` schema rev to `v=1.1`: add `source: "atspi" \| "synthetic" \| "empty"` field; update producer-side dedup hash to include the field; document the bump in the file-header comment of `active.rs` | — | `bridge/src/active.rs` |
| T-14 | Acceptance gates: `cargo test --all-features --locked`, `cargo clippy -- -D warnings`, `cargo fmt --check` all green. Push branch `74-bridge-completion`. | — | (validation) |
