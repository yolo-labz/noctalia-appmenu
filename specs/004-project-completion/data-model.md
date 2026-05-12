# Data model: project completion roadmap (v0.3.0 → v1.0.0)

**Spec:** `specs/004-project-completion/spec.md`
**Generated:** 2026-05-12

This document enumerates the entities, types, traits, files, and configuration surfaces that change between `v0.3.0` and `v1.0.0`. Entities are listed in dependency order (lowest-level types first). Each entry lists its owner module, fields, validation rules, and state transitions (where applicable).

---

## 1. Bridge — focus tracker

### Entity: `FocusEvent`

**Owner:** `bridge/src/focus.rs` (new — moved from `niri.rs`).
**Kind:** plain struct (Send + Sync + Clone).

| Field | Type | Notes |
|---|---|---|
| `id` | `u64` | niri window id (or compositor-specific opaque handle) |
| `pid` | `i32` | resolved by the focus sink before emit; mandatory |
| `app_id` | `String` | reverse-DNS-style app identifier as reported by the compositor |
| `title` | `String` | window title; may be empty |
| `timestamp` | `Instant` | monotonic; used by the debounce stage |

**Validation:**

- `pid > 0` — invariant; emission with `pid == 0` is a bug
- `app_id` may be empty; downstream falls back to `.desktop` lookup
- `timestamp` strictly monotonic per source — out-of-order events are dropped

**State transitions:** none; immutable.

### Entity: `FocusOp`

**Owner:** `bridge/src/focus.rs` (new).
**Kind:** enum.

```
enum FocusOp {
    Emit(FocusEvent),
    Clear,                       // no focused window
    NoOp,                        // duplicate / debounced
}
```

**Validation:** exhaustive match guaranteed by enum-typed transducer.

### Trait: `FocusSink`

**Owner:** `bridge/src/focus.rs` (new).

```
#[async_trait::async_trait]
trait FocusSink: Send + Sync + 'static {
    async fn run(
        &mut self,
        tx: tokio::sync::watch::Sender<Option<FocusEvent>>,
        cfg: crate::config::Config,
    ) -> anyhow::Result<()>;
}
```

**Contract:**

- Implementor owns its compositor-specific event source (niri-IPC socket, future hyprctl, future swayipc).
- On clean shutdown (`tokio::select!` cancel), implementor must close any sockets cleanly.
- Reconnect / backoff is the implementor's responsibility; FR-001 contractually requires backoff to reset after ≥ 30 s of connected session.
- A successful ack of the event-stream subscription must complete within 5 s of connect; otherwise return `Err` and let the supervisor retry.

**Implementors at v1.0.0:** `crate::niri::NiriSink` only.

### Module: `bridge/src/niri.rs`

**Owner:** existing module, refactored.

**Changes for v1:**

- Implements `FocusSink` (no longer free-standing `run` function).
- Backoff state machine: `Floor (250 ms) → Doubling cap 30 s → Reset to floor after ≥ 30 s connected session`.
- Ack-path covered by a new integration test (`bridge/tests/niri_reconnect.rs`).
- No new public types beyond the trait impl.

---

## 2. Bridge — AT-SPI walker

### Module: `bridge/src/atspi.rs`

**Owner:** existing module, modified.

**Persistent connection (FR-006):** module-level `Arc<Mutex<Option<zbus::Connection>>>` or a long-lived task that owns the `Connection` and accepts `Action(pid, oneshot::Sender)` requests on a channel. The latter is preferred — eliminates lock contention on hot paths.

**New helpers:**

- `async fn watch_is_enabled(conn: &Connection) -> anyhow::Result<()>` — subscribes to `org.a11y.Status` `PropertiesChanged`; on `IsEnabled` flipping to `false`, calls `set_is_enabled(true)`.
- `async fn refetch_then_action(pid: i32, path: ObjectPath) -> anyhow::Result<()>` — re-fetches the menubar subtree, locates the addressed item by path, invokes `DoAction(0)`. On path-not-found, returns `MenuError::Stale` and the caller emits a snapshot refresh.

**Validation:**

- `fetch_menu_tree` returns `MenuItem { children: [] }` only when an empty children array is structurally meaningful; the GTK4 `GtkPopoverMenuBar` case (FR-004) triggers a synthetic-menu fallback at the call site, not inside the walker.
- `find_app_for_pid` distinguishes "no a11y registration" (returns `Ok(None)` + log warning) from "PID mismatch through subprocess launcher chain" (returns `Ok(None)` + log info with both PIDs).

### Entity: `MenuItem`

**Owner:** `bridge/src/atspi.rs` (single source-of-truth after FR-009 deletes `dbusmenu.rs`).

| Field | Type | Notes |
|---|---|---|
| `id` | `u32` | walker-assigned monotonic; not stable across walks |
| `label` | `String` | accessible Name minus accelerator markers |
| `item_type` | `enum { Standard, Separator, Submenu }` | derived from AT-SPI role + children |
| `enabled` | `bool` | derived from AT-SPI states `ENABLED` ∧ `SENSITIVE` |
| `visible` | `bool` | derived from AT-SPI states `VISIBLE` ∧ `SHOWING` |
| `icon_name` | `String` | freedesktop icon-theme name; empty when absent |
| `toggle_type` | `Option<enum { Checkmark, Radio }>` | from role: `CHECK_MENU_ITEM = 8`, `RADIO_MENU_ITEM = 45` |
| `toggle_state` | `Option<bool>` | from AT-SPI `CHECKED` state |
| `service` | `String` | AT-SPI bus connection name (`:1.123`) |
| `path` | `ObjectPath` | AT-SPI object path |
| `children` | `Vec<MenuItem>` | DFS subtree (empty for `Standard` leaves) |

**Wire compatibility:** field names + JSON shape are stable per `active.json` schema v=1 (PR #59).

---

## 3. Snapshot file

### Entity: `active.json`

**Path:** `~/.cache/noctalia-appmenu/active.json` (per ADR-0023).
**Producer:** bridge (`bridge/src/active.rs`).
**Consumer:** plugin (`plugin/BarWidget.qml` via `FileView`) + bridge `IpcHandler` push for steady-state.

| Field | Type | Notes |
|---|---|---|
| `v` | `1` | schema version (PR #59; increment in v1.x if breaking) |
| `pid` | `i32` | focused process |
| `app_id` | `String` | as resolved by the focus sink |
| `title` | `String` | window title |
| `menu` | `MenuItem` (root) | walker output OR synthetic-menu fallback |
| `source` | `enum { atspi, synthetic, empty }` | new field at v1.0.0; lets the plugin display a "fallback menu" hint |

**Validation:**

- `pid >= 0`; `pid == 0` is reserved for "no focus" + `source = "empty"`
- `menu.children` must be empty when `source = "empty"`
- `app_id` empty + `source = "synthetic"` is forbidden (must have at least a name to render)

**State transitions:**

- `empty` → `atspi`: focus event fires, walker succeeds with non-empty subtree
- `empty` → `synthetic`: focus event fires, walker returns empty / `MENU_BAR` not found / app not registered
- `atspi` → `atspi`: focus shifts to a different app whose walker also succeeds
- `atspi` → `synthetic`: bus crash + restart before next walk; FR-005 path
- any → `empty`: focus cleared (no window focused; rare)

---

## 4. Plugin — QML components

### Entity: `BarWidget` (modified)

**File:** `plugin/BarWidget.qml`.
**Owner:** existing.

**New responsibilities at v1:**

- Render `toggle_state` indicator in top-level strip items if their `item_type` is `Checkmark` or `Radio` (rare for top-level strip but legal).
- Render `icon_name` via Qt icon-theme lookup for top-level strip items.
- Pass `focused-output ShellScreen` to `AppmenuPopupWindow` instances (FR-013); reject popup-open requests where `screen != focusedScreen`.

### Entity: `AppmenuPopupWindow` (modified)

**File:** `plugin/AppmenuPopupWindow.qml`.
**Owner:** existing.

**New responsibilities at v1:**

- Row delegate renders `toggle_state` indicator (FR-011).
- Row delegate renders `icon_name` (FR-012).
- `onClicked` handler for `hasChildren = true` opens a `SubmenuPopup` instance instead of the current no-op.
- `screen` property guard: refuses to open if the popup's screen mismatches the focused window's output.

### Entity: `SubmenuPopup` (new)

**File:** `plugin/SubmenuPopup.qml` (new).
**Owner:** Lane B.

**Contract:**

- Sibling top-level `PanelWindow` with `WlrLayershell.layer: WlrLayer.Top`, `WlrLayershell.keyboardFocus: WlrKeyboardFocus.None`, `WlrLayershell.exclusionMode: ExclusionMode.Ignore` (matches ADR-0008 + spec 003 FR-005..FR-007).
- Anchored to the right edge of the parent menu item's screen rectangle; falls back to the left edge if the right edge would clip off-screen.
- Renders the same row delegate as `AppmenuPopupWindow` — DRY via a shared `MenuRow.qml` component (optional refactor, Lane B's call).
- Click on a leaf row: invokes `DoAction(0)` via the bridge's `atspi-click` subcommand, closes both popups, sets `_failedState = false`.
- Outside-click: full-screen `MouseArea` inside the popup `PanelWindow`, NOT `xdg_popup.grab` (spec 003 FR-006).

**State transitions:**

- closed → open: parent popup's `hasChildren` row fires `onClicked`
- open → closed: leaf click, outside click, parent popup closes, focus changes

---

## 5. Nix surface

### Entity: `programs.noctalia.plugins.appmenu` HM option

**File:** `nix/module.nix`.
**Owner:** existing module, modified.

| Option | Type | Default | Changes at v1 |
|---|---|---|---|
| `enable` | `bool` | `false` | unchanged |
| `registrar` | `enum [ "vala-panel" "none" ]` | `"vala-panel"` → **`"none"` in v1.0.0** | deprecated; `lib.warn` on non-default; option removed in v1.1 |
| `hideInWindowMenubar` | `bool` | `false` | option preserved; env writes replaced with AT-SPI-correct equivalents (FR-017) |
| `widgetPlacement` | `enum [ "left" "right" ]` or null | null | unchanged at v1; plugins.json wiring resolved per FR-020 |

**New writes:**

- `home.sessionVariables.QT_ACCESSIBILITY = "1"` unconditionally when `enable = true`.
- `assertions += [ { assertion = config.services.gnome.at-spi2-core.enable or false; message = "..."; } ]` OR `warnings += [...]` if assertions cannot fire in HM-only scope.

### Entity: `flake.nix` derivation

**File:** `flake.nix`.
**Owner:** existing flake, modified.

**Changes:**

- `version` attribute reads from a single source (`bridge/Cargo.toml` parsed via `lib.importTOML` OR a new `nix/version.nix` file).
- `SOURCE_DATE_EPOCH` passed via `passthru` or derived from `self.lastModified`; `preBuild` no longer shells out to `git log`.
- Optional: `passthru.tests = { reproducibility = ...; }` for `nix flake check` enforcement.

---

## 6. CI / quality surfaces

### Entity: Repository Ruleset on `main`

**File:** `.github/rulesets/main.json` (export of the live GH config).
**Owner:** Lane D.

**Required status checks at v1.0.0** (FR-025):

- `Lint & format`
- `bridge-test`
- `plugin-lint`
- `reproducibility`
- `osv-scanner`
- `scorecard`
- `codeql`
- `SonarQube standalone scan`
- `AT-SPI integration test`
- `attestation verify (dry-run)`

### Entity: `sonar-project.properties`

**File:** `sonar-project.properties`.
**Owner:** Lane D.

**Changes at v1:**

- `sonar.coverage.minimum=65` (was 60; FR-026).
- `sonar.rust.cognitive.maximumComplexityPerFunction=15` (existing; FR-027 brings code under).
- `sonar.cpd.exclusions=` no longer excludes `bridge/src/dbusmenu.rs` (file deleted per FR-009).
- New: `sonar.coverage.newCode.minimum=80` (PR-gate, also configured server-side).

### Entity: `.github/workflows/release.yml`

**File:** as above.
**Owner:** Lane D.

**Change:** line 77 `cyclonedx-json@1.6` → `cyclonedx-json@1.7` (FR-021). Attestation step's claim now matches the emitted document.

### Entity: `.github/workflows/ci.yml`

**File:** as above.
**Owner:** Lane D.

**New job:** `atspi-integration` — runs Lane A's fake-AT-SPI-registry integration test (`bridge/tests/atspi_integration.rs`) under `cargo test`. Required by the ruleset.

**New step in `plugin-lint`:** qmllint over both QML files + new `SubmenuPopup.qml`; output piped through jq SARIF transform; uploaded via `github/codeql-action/upload-sarif`.

### Entity: `.github/workflows/actionlint.yml`

**File:** as above.
**Owner:** Lane D.

**Change:** `runs-on: [self-hosted, Linux, X64, noctalia-appmenu]` (drop `desktop` label per FR-023).

---

## 7. Documentation

### Entity: `README.md` — "Verify the install" section

**File:** `README.md`.
**Owner:** Lane D.

**Structure (new section):**

1. Prerequisites checklist (NixOS, niri ≥ 25.04, `services.gnome.at-spi2-core.enable = true`).
2. HM config snippet to enable the plugin.
3. `nh os switch` step.
4. Verification commands: `busctl --user list`, `journalctl --user -u noctalia-appmenu-bridge.service`, `niri msg windows`.
5. Expected behaviour (Anki menubar appears in the bar within 200 ms of focus).
6. Documented caveats (Firefox `accessibility.force_disabled = 0`, Electron `--force-accessibility`, multi-monitor not duplicated).

---

## 8. Cross-entity invariants

- **Schema-version consistency.** `active.json` `v=1` is the source-of-truth; bridge and plugin both validate `v` on read. A version bump requires updating both lanes A and B in lockstep within a single PR.
- **Theme tokens only in QML.** All colour / spacing references in `plugin/*.qml` must use `Color.m*` / `Style.*` tokens; raw hex / rgb is a Lane B reviewer-rejection.
- **No re-tag of any release.** Constitution Outscope / Governance invariant. If a defect ships in `v1.0.0`, `v1.0.1` follows; the original tag is never moved.
- **No Cargo `version` drift.** After FR-018 lands, `flake.nix` and `bridge/Cargo.toml` agree on version; `nix flake check` enforces.
