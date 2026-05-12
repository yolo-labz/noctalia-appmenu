# Specification: Project completion roadmap (v0.3.0 → v1.0.0)

**ID:** 004-project-completion
**Created:** 2026-05-12
**Author:** @phsb5321
**Constitution version:** 1.0.0

## Why

`noctalia-appmenu` shipped its `v0.3.0` final release on 10/05/2026 (PR #63). At that point the project crossed three structural milestones:

1. Substrate replacement — DBusMenu/Registrar was retired in favour of an AT-SPI menubar walker (ADR-0024), unblocking Qt+GTK menu export on every Wayland compositor that runs an a11y bus.
2. Plugin fault-isolation envelope landed (spec 003, PRs #50–#57), so bugs in the appmenu widget no longer break neighbouring noctalia widgets.
3. The bridge adopted the upstream `niri-ipc` crate and grew a fixture-replay test harness (PRs #54, #60), removing the last hand-rolled niri socket code.

The constitution defines `v1.0.0` as the next ship gate: "niri Qt+GTK works on three different apps, integration tests pass on CI runner, README's 'Verify the install' recipe works clean on a fresh NixOS box." Between `v0.3.0` final and that gate sit a known set of concrete defects, untested code paths, missing CI surfaces, and packaging holes — none of them speculative, all surfaced by a parallel audit run on 12/05/2026 (`research.md`).

This specification consolidates that audit into a single roadmap. It defines what completion means for `v1.0.0`, the user-visible scenarios that must pass, the testable requirements the roadmap delivers, and the explicit non-goals that stay deferred to `v2`. It is the umbrella spec — each functional requirement either maps to an existing in-flight PR/spec, or earmarks a new follow-up spec (`005-…`, `006-…`, etc.) so the work fits the constitution's "≤25 tasks per spec" cap (Development Workflow §3).

After this spec ships, every `v1.0.0` blocker has a written acceptance test, every retired-but-undeleted code path has a removal owner, and Pedro's "fresh NixOS box" demo recipe is reproducible on the desktop host.

## User scenarios

### Scenario 1: Fresh-NixOS install delivers Anki menubar in the bar

**Given** a freshly-installed NixOS host with `services.gnome.at-spi2-core.enable = true`, `programs.noctalia.plugins.appmenu.enable = true`, and the user has rebooted into a niri+noctalia-shell session
**When** the user opens Anki for the first time and gives it keyboard focus
**Then** within ≤ 200 ms (75 ms debounce + render), Anki's `File / Edit / View / Tools / Help / Ankimon / AnKing` menu strip appears in the noctalia top bar; clicking `File` opens a popup matching Anki's in-window menu; clicking `File → Export…` activates the action in Anki, exactly as if the user had clicked the in-window menu

### Scenario 2: Focus tracking between three real Qt6 apps

**Given** Anki, kate, and dolphin are all running simultaneously
**When** focus moves from one to the next via Mod+Tab
**Then** the bar's menu strip updates within ≤ 200 ms per transition; each app shows its own complete menubar; any previously-open popup closes; no flicker or layout reflow propagates to neighbouring bar widgets

### Scenario 3: Nested submenu navigation

**Given** kate is focused and the user has opened the `File` menu
**When** the user hovers `File → Open Recent…` (a submenu containing recently-opened files)
**Then** a second-level popup opens to the right of `Open Recent…`; the secondary popup is hosted on a sibling top-level layer-shell surface (not nested inside the first popup); clicking a leaf entry activates the corresponding AT-SPI action; both popups close after activation

### Scenario 4: Checkable menu items render correctly

**Given** kate is focused and the user opens the `Tools → Spelling` menu
**When** the popup renders
**Then** menu items whose `toggle_type` is `"checkmark"` and `toggle_state` is `1` display a visible check indicator; items with `toggle_state = 0` display the unchecked state; clicking a checkable item activates the action and the next snapshot reflects the new state

### Scenario 5: AT-SPI bus crash + restart recovery

**Given** the bridge has been running for >5 min and the user has been alt-tabbing between Qt6 apps with menus rendering correctly
**When** the AT-SPI daemon (`at-spi-bus-launcher`) crashes and is restarted by D-Bus activation
**Then** the bridge detects the new a11y bus address on its next focus-change attempt, re-flips `org.a11y.Status.IsEnabled = true` so Qt apps re-register their accessibility trees, and resumes publishing menus within ≤ 5 s of the next focus event; the QML widget collapses to its zero-paint stable slot during the gap (no crash, no error spam in `journalctl`)

### Scenario 6: niri compositor reload (`niri msg reload-config`)

**Given** the bridge is running and the user invokes `niri msg reload-config` while keeping focus on a Qt6 app
**When** the niri socket disconnects and reconnects
**Then** the bridge reconnects with no growing backoff (backoff resets to floor on clean EOF after a connected session); the menu strip restores within ≤ 2 s of niri's restart; the user does not perceive a "blank bar" gap longer than that

### Scenario 7: Reproducible release artefact verifies on a fresh box

**Given** the user downloads the `v1.0.0` release tarball + the `noctalia-appmenu-bridge` binary
**When** they run `gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz`
**Then** the command returns success; the SBOM (CycloneDX 1.7 + SPDX 2.3) referenced in the attestation matches the binary's actual dependency graph; a second build from source under `nix build .#noctalia-appmenu-bridge` produces a byte-identical binary

## Functional requirements

### Bridge — focus tracker (`bridge/src/niri.rs`)

**ADR refs:** ADR-0009 (debounce policy), ADR-0016 (niri event-stream schema).

- **FR-001** Backoff resets to floor (250 ms) after any cleanly-EOF'd session of duration ≥ 30 s, so transient niri restarts (`reload-config`, session resume) do not compound into multi-second blank-bar gaps. Verifiable by a test that drives three EOF cycles, each preceded by a 30 s sleep, and asserts the post-third-EOF reconnect attempt fires within 500 ms.
- **FR-002** The connect/handshake/ack path (`run_once`) carries an integration test that asserts a successful `EventStream` ack is parsed; a regression in ack wire-format produces a test failure, not silent permanent backoff.
- **FR-003** A focus tracker abstraction lives at `bridge/src/focus.rs` exposing a `FocusSink` trait + the `FocusEvent` / `FocusOp` types; `niri.rs` is one concrete implementor. No compositor-specific type leaks beyond `niri.rs`. This unblocks (but does not implement) future Hyprland/Sway sinks.

### Bridge — AT-SPI walker (`bridge/src/atspi.rs`)

**ADR refs:** ADR-0024 (AT-SPI substrate; supersedes ADR-0022, ADR-0023).

- **FR-004** When the walker finds a `MENU_BAR` accessible whose child count is zero (GTK4 `GtkPopoverMenuBar` quirk), it falls back to the `.desktop`-derived pseudo-menu (Scenario 1 of spec 001), instead of rendering an empty bar.
- **FR-005** The bridge subscribes to `org.a11y.Status` `PropertiesChanged` (or an equivalent monitor) and re-invokes `enable_a11y()` whenever `IsEnabled` flips to false — including after a fresh a11y bus instance comes up post-crash. Scenario 5 above is the acceptance scenario.
- **FR-006** The AT-SPI connection used for walking is long-lived (held by a tokio task) rather than re-established per focus event, so a future `children-changed` subscription has a stable connection to attach to.
- **FR-007** Click forwarding (`atspi-click <service> <path>`) re-fetches the addressed accessible before invoking `DoAction(0)`. A stale path (the app rebuilt its widget tree between snapshot and click) returns a typed error and triggers an immediate snapshot refresh instead of silently failing.
- **FR-008** App-matching covers the three v1 reference apps:
  - Anki via subprocess-launcher PID — pass-2 (`app_id_hint` fuzzy match) succeeds even when the niri-reported PID is the wrapper script's PID, not Anki's Qt PID.
  - kate / dolphin via KDE's `org.kde.<app>` double-prefix `app_id` — `normalize_app_id` round-trips correctly.
  - Each match has a unit test asserting the normalisation; no real AT-SPI bus required.
- **FR-009** Both `bridge/src/dbusmenu.rs` and `bridge/src/registrar.rs` are deleted in a dedicated PR before the `v1.0.0` tag. The PR removes the modules from `lib.rs` and any consuming imports. Sonar reports zero new duplication after the deletion.

### Plugin (`plugin/*.qml`)

**ADR refs:** ADR-0008 (popup window for submenus), ADR-0018 (bar widget API contract), ADR-0019 (always-visible bar widget), ADR-0020 (fixed-width slot).

- **FR-010** A `SubmenuPopup.qml` component exists and is instantiated when a menu item's `hasChildren` is true. The submenu is hosted on a sibling top-level layer-shell surface (not nested inside the parent popup) per ADR-0008. Clicking the parent item opens the submenu; clicking a leaf in the submenu activates the AT-SPI action and closes both popups.
- **FR-011** The popup row delegate renders the `toggle_state` field — `checkmark`-typed items show a visible indicator when state is 1, an empty slot when state is 0, and continue to align with neighbouring rows in either state.
- **FR-012** The popup row delegate renders the `icon_name` field via Qt's icon theme lookup when non-empty; renders no leading space when empty. Theme icons inherit Catppuccin Mocha tinting consistent with the rest of the bar.
- **FR-013** The BarWidget opens its popup on the screen whose `ShellScreen` corresponds to the focused window's output. On a multi-monitor host, focusing an app on screen A while screen B is also showing the bar does NOT cause the popup to render on screen B.

### Nix / Home-Manager surface (`nix/module.nix`, `flake.nix`)

**ADR refs:** ADR-0011 (Home-Manager module scope — HM-only at v1), ADR-0024 (env-var rationale for AT-SPI substrate).

- **FR-014** The HM module sets `QT_ACCESSIBILITY = "1"` in `home.sessionVariables` unconditionally when `programs.noctalia.plugins.appmenu.enable = true`.
- **FR-015** The HM module emits an `assertions` entry (or a `lib.warn` at evaluation time when assertions cannot be set in HM scope) telling the user to enable `services.gnome.at-spi2-core` system-wide. The README's "Verify the install" section lists the assertion as a prerequisite check.
- **FR-016** The `registrar` option, the `noctalia-appmenu-registrar` systemd user unit, and the `vala-panel-appmenu` / `appmenu-gtk-module` package dependencies are deprecated in v1.0.0: the option warns when set to a non-default value, the unit is no longer installed by default, and the packages are no longer pulled in. Removal of the option itself is scheduled for v1.1 to give existing users one migration cycle.
- **FR-017** The stale `QT_QPA_PLATFORMTHEME=appmenu-qt5` + `GTK_MODULES=appmenu-gtk-module` env writes (the `hideInWindowMenubar` option) are removed or replaced. If `hideInWindowMenubar` survives, its semantics under AT-SPI substrate are documented in the option description.
- **FR-018** The `bridge` and `plugin` Nix derivations in `flake.nix` derive their `version` attribute from `bridge/Cargo.toml` (or from a shared `version.nix` source-of-truth). A version drift between `Cargo.toml` and the derivation triggers a `nix flake check` failure.
- **FR-019** `SOURCE_DATE_EPOCH` is passed into the bridge derivation as an environment attribute set from outside the sandbox (e.g. `self.lastModified` or a release-workflow-provided override), not derived by calling `git log` inside `preBuild`. A reproducibility CI job builds the bridge twice and asserts byte-identical output.
- **FR-020** Plugin discovery via noctalia-shell's loader works without a manual `~/.config/noctalia/plugins.json` edit. Either the HM module writes the required entry, or the spec documents an upstream noctalia-shell change that performs directory scanning.

### CI / release (`.github/workflows/*`)

**ADR refs:** ADR-0012 (self-hosted runner only), ADR-0013 (runner-agnostic CI), ADR-0014 (local-first CI).

- **FR-021** `release.yml` emits CycloneDX 1.7 (not 1.6); the attestation step's claim matches the document version.
- **FR-022** An AT-SPI integration test runs on every PR. Minimum viable scope: a `cargo test` integration module that stands up a fake AT-SPI registry stub (extending the existing `atspi_probe` example), walks it, and asserts the JSON shape end-to-end. Stretch scope: headless niri + at-spi2-bus + a Qt6 test app, gated to the self-hosted runner.
- **FR-023** `actionlint.yml`'s `runs-on` is runner-agnostic (`[self-hosted, Linux, X64, noctalia-appmenu]` — no `desktop` label hard-pin). Either the hook pollution issue cited in the inline comment is fixed at the hook level, or the workflow is conditionalised to use any available runner labelled `noctalia-appmenu`.
- **FR-024** `qmllint` runs against both `BarWidget.qml` and `AppmenuPopupWindow.qml` (plus `SubmenuPopup.qml` once it lands per FR-010); the output is converted to SARIF and uploaded via `github/codeql-action/upload-sarif`. QML findings appear in the GitHub Security tab.
- **FR-025** The Repository Ruleset on `main` requires, at minimum, the following status checks before `v1.0.0` is tagged: `Lint & format`, `bridge-test`, `plugin-lint`, `reproducibility`, `osv-scanner`, `scorecard`, `codeql`, `SonarQube standalone scan`, `AT-SPI integration test`, `attestation verify (dry-run)`.

### Quality gate (`sonar-project.properties` + SonarQube UI)

- **FR-026** The Sonar quality gate is updated: overall line coverage ≥ 65%, new-code line coverage ≥ 80%, code duplication < 3% overall, cognitive complexity per function ≤ 15, blocker/critical issues = 0.
- **FR-027** `find_app_for_pid` and `fetch_menu_tree` in `bridge/src/atspi.rs` are refactored to bring cognitive complexity below 15 each, OR an ADR (ADR-0025) documents the deviation with rationale before `v1.0.0` ships.

### Documentation

- **FR-028** The README contains a "Verify the install" section reproducible on a fresh NixOS host. The recipe lists every prerequisite (`services.gnome.at-spi2-core.enable = true`, `programs.noctalia.plugins.appmenu.enable = true`), every check the user runs after `nh os switch`, and the expected output of each check. CI executes the recipe headlessly (via the AT-SPI integration test from FR-022).
- **FR-029** Documented caveats for v1: Firefox needs `accessibility.force_disabled = 0`; Electron apps need `--force-accessibility`; multi-monitor menubar duplication is deferred to v2; Alt-letter mnemonics + global Alt-F intercept are deferred to v2.

## Non-functional requirements

- **NFR-001 Performance.** Focus-change → menubar-render P95 ≤ 200 ms on the desktop host (Ryzen 7950X3D); P99 ≤ 400 ms. Submenu open → render P95 ≤ 100 ms. Click → action P95 ≤ 50 ms.
- **NFR-002 Reliability.** Bridge survives ≥ 7 days uninterrupted use without leaking memory > 50 MB RSS. AT-SPI bus crash + restart recovery completes within ≤ 5 s. niri reload-config recovery within ≤ 2 s.
- **NFR-003 Security.** systemd hardening preserved (`NoNewPrivileges`, `ProtectSystem=strict`, `RestrictAddressFamilies=AF_UNIX`). Bridge exec'd subprocesses limited to `atspi-click` (self) and `niri msg action` (synthetic-menu fallback only). No shell-out to user-controlled strings.
- **NFR-004 Testability.** Every FR above has at least one corresponding automated test or runtime check. Untestable surfaces (e.g. multi-monitor on a single-screen runner) are documented and accepted as manual-verification gaps.
- **NFR-005 Reproducibility.** `nix build .#noctalia-appmenu-bridge` is byte-identical across two invocations with the same `SOURCE_DATE_EPOCH`; reproducibility CI job enforces this on every PR.
- **NFR-006 Observability.** Every isolation-related code path logs to `journalctl --user -u noctalia-appmenu-bridge.service` (bridge) or `journalctl --user -u noctalia-shell.service` (plugin) with structured prefixes (`[appmenu]`, `[atspi]`, `[niri]`).

## Out of scope

These items stay deferred to `v2` or beyond; including them in `v1.0.0` would block the ship:

- Hyprland / Sway / KWin / COSMIC focus tracking (constitution principle I; FR-003 only opens the door, doesn't implement)
- Firefox / Thunderbird DBusMenu-style integration (constitution Outscope; AT-SPI works via `accessibility.force_disabled = 0` — documented in FR-029)
- Electron / Chromium full integration (workaround documented; no first-class support)
- Multi-monitor menubar **duplication** (each monitor showing the same menu); v1 is focused-output only
- Alt-letter mnemonics + global Alt-F intercept (deferred to v2 — no clean Quickshell hook in v1)
- AT-SPI `children-changed` subscription full implementation (FR-006 lays prerequisites; subscription itself is a separate spec)
- AT-SPI mid-render mutation handling (app rebuilds widget tree while popup open); render-time staleness is accepted at v1.0.0, click-time staleness is handled by FR-007
- NixOS module (system-level) mirror of the HM module (ADR-0011 defers to v2)
- Plugin marketplace publication / noctalia-shell upstream registry (not blocking v1 install)
- Telemetry / opt-in usage reporting (no requirement from constitution; explicitly out)
- Output hotplug while a popup is open (popup may render on the vanished output until the next focus event); v2 follow-up

## Constraints / dependencies

- Quickshell ≥ 0.3.0 (no upper bound assumed; verify on next minor)
- noctalia-shell ≥ 1.0.0 (v4 architecture per spec 003 — full-screen PanelWindow assumption stays valid)
- niri (any IPC-1.x compatible build; current testing baseline is niri 25.04)
- `at-spi2-core` ≥ 2.50 (any system with `services.gnome.at-spi2-core` enabled)
- Qt6 ≥ 6.7 (verified menubar role on AT-SPI per `research.md` §4)
- GTK4 ≥ 4.14 (`GTK_ACCESSIBLE_ROLE_MENU_BAR` mapped) — older GTK3 apps work but may show the empty-children quirk (FR-004)
- Self-hosted runner (`vm103`) available for AT-SPI integration tests; CI must degrade gracefully when offline (FR-023)
- Existing in-flight PRs / specs that overlap:
  - Spec 003 (plugin fault-isolation) — already merged; v1.0.0 inherits invariants
  - ADR-0024 (AT-SPI substrate) — accepted; defines the substrate v1.0.0 ships
  - ADR-0011 (HM module) — accepted; v1.0.0 stays HM-only
  - PRs #64–#72 (Dependabot) — triaged in `research.md`; v1.0.0 must merge or close each
- **No re-tag of any release.** Constitution + yolo-labz release-engineering invariant restated here for visibility: a defect found post-tag rolls forward to `v1.0.1`; the `v1.0.0` tag is never moved. `slsa-verifier` validates against the commit SHA at signing time, so re-tagging produces stale provenance.

## Success criteria

- **SC-001** All 7 user scenarios above pass end-to-end on Pedro's desktop host with `noctalia-shell` running, manually verified once and CI-replayed (where feasible).
- **SC-002** `gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz` succeeds against the `v1.0.0` release artefact; the attached SBOM is CycloneDX 1.7 + SPDX 2.3 and dependency-complete.
- **SC-003** All required CI checks (per FR-025) are green on `main` at the `v1.0.0` tag.
- **SC-004** The README "Verify the install" recipe (FR-028) runs clean from a fresh NixOS box to working Anki menubar in the bar in ≤ 10 min of user time, including the `nh os switch` step.
- **SC-005** A 7-day uninterrupted-use soak passes on the desktop host with the 5-app integration set (Anki, kate, dolphin, plus 2 GTK apps from {gimp, inkscape, nautilus}) — no crash, no memory growth over 50 MB RSS, no observable focus-tracking regressions.
- **SC-006** SonarQube quality gate per FR-026 is green on `main` at the `v1.0.0` tag.
- **SC-007** No `dbusmenu.rs` or `registrar.rs` module exists in `bridge/src/` at the `v1.0.0` tag; `cargo test` and `cargo build` succeed without them.
- **SC-008** Reproducibility job (FR-019) is green across two consecutive runs on `main` at the `v1.0.0` tag.

## Key entities

- **Focus tracker** — `bridge/src/niri.rs` + new `bridge/src/focus.rs`. Owns the niri-IPC event-stream loop, debouncing, reconnect/backoff. After FR-003, exposes a `FocusSink` trait.
- **AT-SPI walker** — `bridge/src/atspi.rs`. Owns the a11y bus connection, menubar DFS, app-PID resolution, click forwarding, synthetic-menu fallback.
- **Active snapshot** — `~/.cache/noctalia-appmenu/active.json` (schema v=1, ADR-0023). Producer = bridge; consumer = plugin via `FileView` + `IpcHandler`.
- **BarWidget root** — `plugin/BarWidget.qml`. Renders the menu strip in the noctalia top bar; owns popup state, fault-isolation envelope (spec 003), theme tokens.
- **Popup surfaces** — `plugin/AppmenuPopupWindow.qml` (top-level) + new `plugin/SubmenuPopup.qml` (nested). Both are sibling layer-shell surfaces (FR-010, ADR-0008).
- **HM module** — `nix/module.nix`. Provisions the bridge systemd unit, plugin discovery, env vars, AT-SPI prerequisites.
- **Release artefact** — `noctalia-appmenu-bridge` binary + SBOMs (CDX 1.7 + SPDX 2.3) + GitHub-native build provenance attestation.
- **Repository Ruleset** — required-checks set on `main` (FR-025); blocks merges that don't satisfy the v1.0.0 quality bar.

## Assumptions

- The constitution's `v1.0.0` definition stays unchanged through this work; if it materially shifts, this spec is amended with a Sync Impact Report.
- noctalia-shell v4 (full-screen PanelWindow) remains the deployment target; v5 migration is out of scope per spec 003.
- The self-hosted runner (`vm103`) stays available; CI flake on runner outage is accepted as a temporary regression, not a ship blocker (FR-023 makes the workflow runner-agnostic).
- Pedro's desktop host (Ryzen 7950X3D, Radeon 7800 XT, AMD) is the worst-case stress profile for surface-damage propagation; passing SC-005 on his hardware is sufficient.
- The 5-app integration set (Anki, kate, dolphin, plus 2 GTK apps) is the canonical correctness gate; broader app coverage is post-v1 work.
- Each open Dependabot PR (#64–#72) is independently triaged per `research.md`; none blocks `v1.0.0` directly but each must reach a terminal state (merged or closed).
