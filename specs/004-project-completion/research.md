# Research: project completion roadmap (v0.3.0 → v1.0.0)

**Spec:** 004-project-completion
**Date:** 2026-05-12
**Method:** parallel multi-agent swarm — 7 specialised agents (niri-wayland-tester, dbusmenu-protocol-expert, qml-architect, ci-engineer, nix-packager, sonar-quality-gate, deep-researcher) audited current `main` (commit `489b91d` — bridge 0.3.0 final, 10/05/2026) against the constitution's `v1.0.0` ship gate.

This document records the findings each agent surfaced, the synthesis used to define spec.md's functional requirements, and the web/ecosystem context that informs scope decisions.

---

## 1. Focus tracker (`bridge/src/niri.rs`) — niri-wayland-tester audit

### Concrete gaps for v1.0.0

| Finding | Location | Severity |
|---|---|---|
| Reconnect path (`run` outer loop, `run_once` connect/ack/shutdown) never exercised by any test. Fixture-replay harness only drives `drive_events_with` after the reader is connected. | `bridge/src/niri.rs:154–175`, `:180–233` | high |
| Backoff does not reset after a successful long-lived session. After three niri restarts the bridge waits 30 s before republishing focus. | `:155–171` | high |
| `EventStream` ack-path (`Response::Handled` parsing) has zero test coverage. A niri-side wire-format change drops the bridge into permanent backoff loop. | `:213–228` | medium |
| Socket-path discovery hard-couples to `NIRI_SOCKET` env var. No fallback for `XDG_RUNTIME_DIR/niri.<pid>.<seq>.sock`, no diagnostic before exit when env var absent. | `:181` | low |

### Integration test coverage gaps

- Reconnect-on-socket-close mid-stream (drop, verify reconnect + post-reconnect `WindowsChanged` seed).
- Debounce interaction with rapid focus changes (two events within 75 ms — assert only the final propagates).
- AT-SPI failure path with valid focus (PID has no menu — verify synthetic-menu emission, not panic).
- `UnknownWindow` recovery (focus before `WindowOpenedOrChanged` arrives, then late-arriving open event — must not double-emit focus).

### Compositor-abstraction door

Constitution principle I says "refuse abstraction until one impl ships in prod." That gate is now met. Minimal refactor that un-blocks Hyprland/Sway without churn:

- Extract `FocusOp` transducer + `FocusEvent` type into `bridge/src/focus.rs` (new module).
- Define a `FocusSink` trait: `async fn run(tx: watch::Sender<Option<FocusEvent>>, cfg: Config) -> Result<()>`.
- `niri.rs` becomes one concrete implementor; no other module imports it directly.
- `proxy.rs` / `active.rs` already consume `watch::Receiver<Option<FocusEvent>>` and are compositor-agnostic today — no churn there.

Captured in spec.md FR-003.

### Top 3 ship-blocker risks

1. **AT-SPI substrate unverified in CI on real Qt6 apps** — T-022 is manual-only; silent fallback can pass it.
2. **Backoff regrowth on multi-restart** — user-visible blank bar.
3. **Ack-path untested** — niri version drift = permanent breakage.

---

## 2. AT-SPI walker (`bridge/src/atspi.rs`) — dbusmenu-protocol-expert audit

### Gaps against the 3-app v1 gate

**Anki (Qt6 via Python wrapper).** The niri-reported PID is `anki` (wrapper); AT-SPI registers under `anki.bin` (Qt subprocess). Pass-1 (`find_app_for_pid`, lines 316–333) PID-match misses. Pass-2 (STATE_ACTIVE + fuzzy-name) at lines 363–370 / 390–403 should succeed via `normalize_app_id` (line 551–562), but no test exists for the subprocess-launcher chain. No log distinguishes "no a11y" from "PID mismatch" silently passing through.

**kate / dolphin (KDE).** App-id `org.kde.kate` / `org.kde.dolphin`; `normalize_app_id` strips the prefix and fuzzy-match at lines 390–403 succeeds. No regression test asserts the KDE double-prefix round-trip.

**GIMP / Inkscape (GTK).** Role enum matches Qt; toolkit-agnostic walker. **Known quirk not handled:** GTK4 `GtkPopoverMenuBar` (Nautilus 45+) exposes `MENU_BAR` role with zero children — `fetch_menu_tree` at `:635` returns an empty array, no fallback to synthetic, QML renders empty bar.

### Click forwarding race

`do_action` at `:815–837` builds a fresh `connect_a11y()` connection per click, addresses the path embedded in `MenuItem.path` from the most recent snapshot. If the app rebuilt its widget tree between snapshot and click (common for hover-driven menus), the path is stale → `DoAction(0)` returns `UnknownObject`. ADR-0024 acknowledges the problem but the implementation does not re-fetch before clicking.

### children-changed subscription

ADR-0024 §Consequences Negative #3 deferred subscription. Minimal viable path: persistent per-`service` `children-changed` listener on `org.a11y.atspi.Event.Object:ChildrenChanged`. Prerequisite: persistent a11y connection (currently `connect_a11y` is one-shot per focus event at `:265–284`). Eager re-walk on signal is simpler than subscription-tree management.

### AT-SPI bus lifecycle

`enable_a11y` (`:240–259`) sets `IsEnabled = true` once at startup. After a11y bus crash + restart, `GetAddress` returns the new socket and `connect_a11y` reconnects correctly. **Gap:** `enable_a11y` is never re-invoked. Qt apps launched after bridge startup but on the new bus instance silently don't register. No monitoring loop on `org.a11y.Status` PropertiesChanged.

Captured in spec.md FR-005.

### Dead code retirement

`bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs` still present at v0.3.0 final. ADR-0024 said "v0.3.x will prune" — that has not happened. Risk: shared type drift, binary bloat, reviewer confusion. Plan in spec.md FR-009.

### Compositor agnosticism

`atspi.rs` has no niri-specific calls. Synthetic-menu dispatch via `niri msg action` (`:1006–1024`) is correctly isolated as a niri-specific feature, not a substrate dependency.

---

## 3. Plugin (`plugin/*.qml`) — qml-architect audit

### High-risk gap: nested submenus not implemented

`AppmenuPopupWindow.qml:242–249` contains:

```
// Nested submenus: TODO alpha.19+. For
// now, leaf-only activation.
```

The `onClicked` handler at `:240` checks `hasChildren` but does nothing when true — the click is silently swallowed. The submenu indicator `›` is rendered (`:227`) but clicking it is a no-op. No `SubmenuPopup.qml` component exists despite ADR-0008 specifying one. Any menu item with children (Recent Files, Open With, Edit → Find submenu, etc.) is permanently inaccessible.

**v1.0.0 blocker** — real Qt6 apps (kate, dolphin) have nested menus. Captured in spec.md FR-010.

### Medium gaps

- `toggle_type` / `toggle_state` fields in `active.json` ignored by both QML files. Checked items look identical to unchecked. Spec.md FR-011.
- `icon_name` field carried in the tree, ignored. No `Image` element anywhere in `plugin/`. Spec.md FR-012.
- Multi-screen popup routing: `AppmenuPopupWindow` anchors full screen but no guard ensures it opens on the screen where focus actually lives. Whether noctalia's BarSection per-screen mounting protects this is unverifiable from QML alone. Spec.md FR-013.

### Confirmed correct (no change)

- Keyboard nav: Alt-letter mnemonics + Alt-F intercept deferred to v2 per ADR-0010 + constitution Outscope. `WlrKeyboardFocus.None` at `AppmenuPopupWindow.qml:86` is consistent.
- Theme integration: clean — `Color.mSurface` / `Color.mOnSurface` / `Style.barHeight` etc., zero raw hex. One defensive fallback at `BarWidget.qml:436` / `AppmenuPopupWindow.qml:205` (`Style.marginXS !== undefined ? Style.marginXS : 4`) is a guard for older noctalia versions, not a token violation.

---

## 4. CI / release surface — ci-engineer audit

### Ship blockers

| # | Finding | Location | Captured |
|---|---|---|---|
| 1 | `release.yml` emits `cyclonedx-json@1.6` but the attestation step claims 1.7. v0.3.0 SBOM is technically nonconforming. | `.github/workflows/release.yml:77` | FR-021 |
| 2 | No AT-SPI integration harness in CI. `bridge-test` runs unit tests only; the AT-SPI walker added per ADR-0024 has zero CI coverage. | `.github/workflows/ci.yml` (missing job) | FR-022 |
| 3 | `actionlint.yml` hard-pins `runs-on: [self-hosted, Linux, X64, noctalia-appmenu, desktop]`. Violates ADR-0013 (runner-agnostic). | `.github/workflows/actionlint.yml:33` | FR-023 |

### High severity (must land before v1 tag)

- qmllint runs but does not emit SARIF; `AppmenuPopupWindow.qml` is not linted at all (only `BarWidget.qml`). Spec.md FR-024.
- Sonar + cargo-deny + codecov not required by the Ruleset on `main`. Spec.md FR-025.

### Dependabot triage (PRs #64–#72, updated 11/05/2026)

**Merge immediately** (patch, low blast):
- #65 osv-scanner-action 2.3.5 → 2.3.8
- #66 cargo-deny-action SHA bump
- #69 tokio 1.52.2 → 1.52.3 (security group)
- #71 tempfile 3.20 → 3.27

**Review before merging** (minor, verify behaviour):
- #72 codeql-action 4.35.3 → 4.35.4 (SHA comment intact in all three workflows that pin it)
- #70 iai-callgrind 0.14.2 → 0.16.1 (bench API compatibility under `bridge/benches/`)
- #67 zbus-stack group (touches AT-SPI substrate; local `cargo test` first)

**Manual look** (major version):
- #68 codecov/codecov-action 5.5.1 → 6.0.0 (v6 dropped tokenless upload; `files` input renamed; current pin is v5.0.7 so DB is jumping two minor versions)
- #64 actions/deploy-pages 4.0.5 → 5.0.0 (verify `outputs.page_url` step output name)

---

## 5. Nix surface (`flake.nix` + `nix/module.nix`) — nix-packager audit

### Critical: AT-SPI prerequisites unwired

`nix/module.nix` has **zero AT-SPI provisioning** despite ADR-0024 (accepted 06/05/2026). Missing:

- `services.gnome.at-spi2-core.enable = true` requirement (system-level; HM cannot set but can assert / warn).
- `QT_ACCESSIBILITY = "1"` env var (HM-scope; must be unconditional when `enable = true`).
- Warning text guiding the user to enable a11y2 system-wide.

Without `QT_ACCESSIBILITY=1`, Qt apps silently don't register their accessibility trees. The bridge returns empty for all Qt targets. Spec.md FR-014, FR-015.

### Stale env vars + dead options

`module.nix:202–205` sets `QT_QPA_PLATFORMTHEME=appmenu-qt5` + `GTK_MODULES=appmenu-gtk-module`. Both are pre-ADR-0024 (registrar-based) env writes. Under AT-SPI substrate they have no effect — Qt reads directly from the a11y bus. Spec.md FR-017.

`registrar = "vala-panel"` default (`:70–78`), `noctalia-appmenu-registrar` unit (`:187–199`), and `vala-panel-appmenu` + `appmenu-gtk-module` packages (`:121–124`) are dead post-ADR-0024 but still installed unconditionally when the option is at its default. Spec.md FR-016.

### Flake hygiene

- Version mismatch: `flake.nix:42` (bridge) and `:66` (plugin) hardcode `version = "0.1.0"`; `bridge/Cargo.toml` is `0.3.0`. Spec.md FR-018.
- `SOURCE_DATE_EPOCH` fallback (`flake.nix:51`) calls `git log` inside the sandbox — git is not always available. Fallback hardcodes 2025-01-01. Spec.md FR-019.

### Plugin discovery

Spec 001 FR-007 mentioned a manual `plugins.json` edit is required. The HM module wires `xdg.configFile."noctalia/plugins/noctalia-appmenu"` (`:127–130`) but does not write any entry into `~/.config/noctalia/plugins.json`. If noctalia-shell's loader requires the manifest index rather than directory scanning, the plugin will not load without manual editing. Status: unresolved — `module.nix:94–100` comment says "wiring lands in v0.2", still pending. Spec.md FR-020.

### Decision: stay HM-only for v1

ADR-0011 deferred system-level NixOS module to v2. Reaffirmed by this audit.

### systemd hardening — already strong

`NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome=read-only`, `PrivateTmp`, `PrivateDevices`, `ProtectControlGroups`, `ProtectKernelModules`, `ProtectKernelTunables`, `RestrictAddressFamilies=AF_UNIX`, `RestrictNamespaces`, `LockPersonality`, `MemoryDenyWriteExecute`, `SystemCallArchitectures=native`. Adding `PrivateNetwork` is belt-and-suspenders, not a blocker.

---

## 6. SonarQube quality gate — sonar-quality-gate audit

### Coverage state

- `bridge/lcov.info` fed from `cargo-llvm-cov`. Floor in `sonar-project.properties`: 60% (alpha-era, never updated).
- AT-SPI async paths (`find_app_for_pid`, `find_menubar`, `fetch_menu_tree`, `fetch_menubar_for_pid`, `do_action`, `scan_for_active_frame`) have zero unit coverage — they require a live a11y bus.
- `niri.rs` has fixture-replay harness from PR #60, covering pure + async transducer paths.
- `active.rs` snapshot path has integration test, excluded from coverage counting via `sonar.coverage.exclusions=bridge/tests/**`.

### qmllint SARIF

Not wired. `ci.yml:72` runs qmllint for exit-code gating; line 68 comment claims "no native SARIF emitter" — outdated, can be done via jq transform on qmllint's stable JSON output. `AppmenuPopupWindow.qml` not linted at all. Spec.md FR-024.

### Cognitive-complexity hot paths

- `find_app_for_pid` (`atspi.rs:295–408`): three sequential passes, nested loops, fuzzy-match logic. Estimated complexity **18–22**. Threshold: 15.
- `fetch_menu_tree` (`atspi.rs:618–746`): four-property fetch, two bitmask ops, three-way `item_type` branch, `toggle_type` match, recursive `Box::pin`, grandchild-flatten block. Estimated complexity **16–20**.
- `scan_for_active_frame` (`:486–542`): recursive, shallow body — estimated **10–12** (under threshold).

Spec.md FR-027.

### Code duplication

`dbusmenu.rs` `MenuItem` and `atspi.rs` `MenuItem` are structurally nearly identical (same fields). Sonar's duplication detector flags ~30–40 lines. Likely trips the 3% threshold on a codebase this size. Resolution: delete `dbusmenu.rs` per spec.md FR-009.

### v1.0.0 gate proposal

- Overall coverage ≥ 65% (achievable with mock AT-SPI fixture)
- New-code coverage ≥ 80% (PR gate in SonarQube UI)
- Duplication < 3% overall (after FR-009 deletion)
- Cognitive complexity ≤ 15 per function (after FR-027 refactor or ADR-0025)
- Blocker/critical issues = 0

Spec.md FR-026.

---

## 7. 2026 Wayland global-menu ecosystem — deep-researcher

### `org_kde_kwin_appmenu_manager` cross-compositor adoption

Stagnant. KWin-only.

- Hyprland: [discussion #1358](https://github.com/hyprwm/Hyprland/discussions/1358) open since 2022, no PR/roadmap.
- Sway 1.12-rc1 (2026): added colour-mgmt/HDR/workspace-v1; **no** kde-appmenu. wlroots declines KDE-prefixed protocols.
- COSMIC Epoch 1 (11/12/2025) + Epoch 2/3 roadmap: no appmenu-manager.
- niri maintainer has explicitly declined KWin-specific Wayland protocols.

**Implication:** ADR-0024's decision to abandon DBusMenu remains correct. No "X lands soon" trigger to revisit before 2027.

### Competing implementations

- `appmenu-gtk-module-wayland` (guiodic fork) + `dmacvicar/appmenu-ng-gtk-module`: still rely on `gtk-shell` / X11. Broken on niri/Hyprland/Sway. KDE bug [#424485](https://bugs.kde.org/show_bug.cgi?id=424485), [#450038](https://bugs.kde.org/show_bug.cgi?id=450038) still open.
- `vala-panel-appmenu`: Xfce/MATE-only X11. No Wayland port.
- Fildem ([gonzaarcr/Fildem PR#176](https://github.com/gonzaarcr/Fildem/pull/176)): GNOME-extension scope, DBusMenu-based.

**No competing Wayland-niri global-menu daemon exists.** noctalia-appmenu has the niche.

### AT-SPI consumers — growing rapidly in 2026

AI/automation agents (`isac322/kwin-mcp`, fazm.ai 2026 landscape) walking AT-SPI trees on Wayland. Known pitfalls: Sway/wlroots intermittent event drops; Firefox-Wayland a11y regressions only partially fixed by 04/2026; `accerciser` debug tool unmaintained.

**Implication:** AT-SPI walker on strengthening trajectory — validates v0.3 substrate for 2-year horizon.

### Qt6 / GTK4 a11y export — solid

- GTK 4.14+ ([blog.gtk.org](https://blog.gtk.org/2024/03/08/accessibility-improvements-in-gtk-4-14/)): `GTK_ACCESSIBLE_ROLE_MENU_BAR` mapped on `GtkPopoverMenuBar` → AT-SPI clean.
- Qt 6.11: full QAccessible AT-SPI bridge on Unix. Menubar role complete.

No known `MENU_BAR` gaps on either toolkit.

### Electron + Firefox

- Electron/Chromium AT-SPI export "quite good"; `--force-accessibility` reliable (VS Code, Slack, Discord verified).
- Firefox Wayland default since v121 (2023); 2025/2026 regressions partially fixed by 04/2026 ([fazm.ai 2026 GUI landscape](https://fazm.ai/blog/agentic-infrastructure-landscape-2026-linux-desktop-gui)). Still needs `accessibility.force_disabled = 0` for stable export.

Documented as caveats in spec.md FR-029.

### Demand trajectory

- KDE Plasma 6.6 keeps Global Menu widget (preinstalled); active maintenance ([KDE bug #483170](https://bugs.kde.org/show_bug.cgi?id=483170) Krita).
- GNOME 47/48/49: no native global menu; community demand persists ([Discourse #24477](https://discourse.gnome.org/t/please-support-global-menu-on-gnome-47/24477)).
- COSMIC silent on appmenu.

**Demand exists, supply doesn't** — exactly noctalia-appmenu's market window.

### v1.0.0 scope recommendation

1. Stay AT-SPI walker. No protocol convergence justifies revisiting DBusMenu before 2027.
2. Document Firefox/Electron caveats in README (`--force-accessibility`, Firefox `accessibility.force_disabled = 0`).
3. Defer Hyprland/Sway/COSMIC focus tracking until niri is rock-solid — don't dilute substrate.
4. No new toolkit-specific code paths needed — GTK4/Qt6 menubar export is uniform.

---

## 8. Synthesis → spec.md mapping

| Audit finding | Spec.md FR | Severity |
|---|---|---|
| Backoff regrowth | FR-001 | high |
| Ack-path untested | FR-002 | medium |
| Compositor abstraction door | FR-003 | medium |
| GTK4 empty-children fallback | FR-004 | medium |
| AT-SPI `IsEnabled` re-flip on bus restart | FR-005 | high |
| Persistent a11y connection | FR-006 | medium |
| Click forwarding re-fetch | FR-007 | medium |
| App-id matching tests (Anki, KDE) | FR-008 | medium |
| Delete `dbusmenu.rs` + `registrar.rs` | FR-009 | medium |
| Submenu popup component | FR-010 | high (ship blocker) |
| `toggle_state` rendering | FR-011 | low |
| `icon_name` rendering | FR-012 | low |
| Multi-screen popup routing | FR-013 | medium |
| `QT_ACCESSIBILITY=1` env var | FR-014 | high |
| AT-SPI system-level assertion | FR-015 | high |
| Deprecate `registrar` option + vala-panel deps | FR-016 | medium |
| Replace stale `hideInWindowMenubar` env writes | FR-017 | medium |
| Flake version source-of-truth | FR-018 | low |
| `SOURCE_DATE_EPOCH` injection | FR-019 | medium |
| Plugin discovery (`plugins.json`) | FR-020 | medium |
| CycloneDX 1.7 | FR-021 | high (ship blocker) |
| AT-SPI integration test in CI | FR-022 | high (ship blocker) |
| Runner-agnostic actionlint | FR-023 | high (ship blocker) |
| qmllint SARIF upload | FR-024 | medium |
| Required-checks ruleset | FR-025 | medium |
| Sonar quality gate | FR-026 | medium |
| Cognitive-complexity refactor | FR-027 | medium |
| README "Verify the install" recipe | FR-028 | high |
| Documented caveats | FR-029 | low |

## 9. Dependencies + risks

**External dependencies the v1.0.0 ship depends on but does not own:**

- Quickshell stays API-stable through 0.3.x — risk of upstream churn rated low (no recent breaking changes per `research.md` §4).
- noctalia-shell v4 stays the deployment target — v5 migration is upstream's concern.
- niri stays IPC-1.x compatible — schema-drift policy is "warn-and-skip unknown variants" per ADR-0016.
- `at-spi2-core` stays D-Bus-protocol compatible — protocol has been stable since 2.0 in 2011.

**Risks that could shift v1.0.0 scope:**

1. Hyprland or Sway lands appmenu-manager unexpectedly → AT-SPI substrate becomes redundant for those compositors. Unlikely per research.md §1; if it happens, addressed in v2.
2. AT-SPI walker performance does not meet NFR-001 on Pedro's hardware → spec.md SC-001 fails; mitigation = `children-changed` subscription (currently scoped out, FR-006 lays prerequisites).
3. noctalia-shell v5 lands before v1.0.0 → spec 003 fault-isolation invariants stay valid but the v5 single-PanelWindow assumption changes; mitigation = constitution-check via `speckit.analyze` on every PR.

## 10. Sources cited

- ADR-0008 (popup window for submenus) — `docs/adr/ADR-0008-popup-window-for-submenus.md`
- ADR-0011 (HM module) — `docs/adr/ADR-0011-home-manager-module.md`
- ADR-0013 (runner-agnostic CI) — `docs/adr/ADR-0013-runner-agnostic-ci.md`
- ADR-0016 (niri event-stream schema) — `docs/adr/ADR-0016-niri-event-stream-schema.md`
- ADR-0024 (AT-SPI substrate) — `docs/adr/ADR-0024-atspi-substrate.md`
- Constitution v1.0.0 — `.specify/memory/constitution.md`
- Spec 001 (global menu MVP) — `specs/001-global-menu/spec.md`
- Spec 002 (bridge DBusMenu mirror, superseded by ADR-0024) — `specs/002-bridge-dbusmenu-mirror/spec.md`
- Spec 003 (plugin fault-isolation envelope) — `specs/003-plugin-fault-isolation/spec.md`
- PR #63 (bridge 0.3.0 final), #60 (fixture-replay harness), #59 (active.json schema v=1), #57 (isolation envelope), #54 (niri-ipc crate adoption)
- Hyprland discussion [#1358](https://github.com/hyprwm/Hyprland/discussions/1358)
- KDE bugs [#424485](https://bugs.kde.org/show_bug.cgi?id=424485), [#450038](https://bugs.kde.org/show_bug.cgi?id=450038), [#483170](https://bugs.kde.org/show_bug.cgi?id=483170)
- [GTK 4.14 a11y improvements](https://blog.gtk.org/2024/03/08/accessibility-improvements-in-gtk-4-14/)
- [fazm.ai 2026 Linux desktop GUI landscape](https://fazm.ai/blog/agentic-infrastructure-landscape-2026-linux-desktop-gui)
- [Qt 6.11 accessibility docs](https://doc.qt.io/qt-6/accessible.html)
- [GNOME Discourse global menu request](https://discourse.gnome.org/t/please-support-global-menu-on-gnome-47/24477)
- [Sway 1.12-rc1 release notes](https://github.com/swaywm/sway/releases)
- [Wayland Explorer: kde-appmenu](https://wayland.app/protocols/kde-appmenu)
