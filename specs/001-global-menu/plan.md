# Implementation plan: Global menu MVP

**Spec:** `specs/001-global-menu/spec.md`
**Constitution version:** 1.0.0

## Approach

Three layers wired in sequence:

1. **`bridge/src/`** (Rust). Subsystems run as cancellation-safe tokio tasks under a single `main`. `niri.rs` long-pipes `niri msg event-stream` and emits `(winid, pid, app_id, title)` updates on a `watch` channel. `registrar.rs` subscribes to `com.canonical.AppMenu.Registrar` and emits `pid → (bus, path)` on a second `watch` channel. `active.rs` joins them with a 75 ms debounce. `proxy.rs` owns `org.noctalia.AppMenu` and republishes properties to the QML side.

2. **`plugin/`** (QML). `BarWidget.qml` binds `DBusObject` to the active proxy and `DBusMenuHandle` to the dynamic `(bus, path)`. `MenuButton.qml` and `SubmenuPopup.qml` render the menubar tree. Fallback text is the focused app's `app_id` when no menu is registered.

3. **`nix/module.nix`** (Home-Manager). Installs both packages, lays out the plugin payload under `~/.config/noctalia/plugins/`, writes `~/.config/noctalia-appmenu-bridge/config.toml`, and starts two `systemd --user` services (`noctalia-appmenu-bridge` + `noctalia-appmenu-registrar`).

Reference: ADR-0001 (registrar reuse), ADR-0003 (Rust sidecar), ADR-0004 (PID-keyed mapping), ADR-0006 (graceful degradation), ADR-0007 (fixed-proxy via bridge), ADR-0008 (PopupWindow), ADR-0009 (debounce policy), ADR-0011 (HM module).

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | Explicit; no compositor abstraction layer. |
| II — Sidecar by default | PASS | DBus server-side work is in Rust; QML only consumes a fixed proxy. |
| III — Worktree-first git | PASS | All implementation work happens in `noctalia-appmenu-NN-slug` worktrees. |
| IV — Conventional Commits + DCO | PASS | Lefthook + commitlint enforced from commit one. |
| V — Speckit-driven | PASS | This very plan; tasks.md follows. |
| VI — Release-engineering compliance | PASS | Workflows lifted from yolo-labz/wa with action SHAs preserved. |
| VII — Graceful degradation | PASS | FR-006 + FR-010 specify the fallback paths. |

## Architecture sketch

```
                              user session bus
+---------+   event-stream   +-----------------+
|  niri   | ---------------> |     bridge      |
+---------+                  | (Rust, tokio)   |
                             +-----------------+
+----------+ WindowRegistered    |  (pid → bus,path)
| registrar|---------------------|
| (vala-   |                     v
|  panel)  |              org.noctalia.AppMenu
+----------+              /org/noctalia/AppMenu/Active
                                 |
                                 |  bus_name, object_path, app_id, title
                                 v
                          +-----------------+
                          |  noctalia plugin |
                          | (BarWidget.qml)  |
                          |  DBusObject ─────────► reads properties
                          |  DBusMenuHandle ──────► renders menu
                          +-----------------+
```

## Affected files

- `bridge/src/{main,config,niri,registrar,active,proxy}.rs` (created)
- `plugin/{manifest.json,BarWidget.qml,components/{MenuButton,SubmenuPopup}.qml}` (created)
- `nix/module.nix` + `flake.nix` (created)
- `tests/{bridge/*,integration/*}` (created)
- `tools/fake-registrar/registrar.py` (created)

## Risks

- **R1** niri's `event-stream` JSON schema is pre-1.0 and may drift. *Mitigation*: pin `niri-ipc` crate version in `Cargo.lock`; bridge logs schema mismatch and exits.
- **R2** Quickshell's `DBusMenuHandle` may bind to a `(service, path)` only when consumed via `SystemTrayItem`. *Mitigation*: ADR-0007 fallback — bridge re-publishes the upstream menu under `/org/noctalia/AppMenu/Active/menu` so QML attaches to a constant address. Tracked as v0.2 work; v0.1 attempts dynamic binding first.
- **R3** GTK menu export is broken on mainline `appmenu-gtk-module` on Wayland (KDE bug 424485). *Mitigation*: HM module installs the `appmenu-gtk-module-wayland` fork; documented in README.
- **R4** `vala-panel-appmenu` upstream is dormant. *Mitigation*: ADR-0001 explicitly documents this; v0.2 evaluates a built-in registrar.
- **R5** VM 103 disk at 82% — `_work/` accumulation could fail builds. *Mitigation*: GC after each release; document in CLAUDE.md.

## Rollout

- Dev cycle: `nix develop` → `just bridge.test` + `just plugin.lint` + `just flake-check`.
- Smoke: bring up bridge in foreground, focus Anki, verify menubar appears.
- PR with worktree-based feature branch.
- CI green → squash-merge to main.
- Repeat per `tasks.md` task.
- When SC-001..SC-005 all pass, tag `v0.1.0`.

## Open questions

1. Does Quickshell ≥ 0.3.0 expose any way to construct `DBusMenuHandle` from QML, or is the bridge re-publication of the menu mandatory? [Investigate during T-007].
2. Should the bridge poll for stale PIDs or rely on `NameOwnerChanged` only? [Decide during T-005 implementation].
