# Architecture Decision Records

Each ADR captures a decision with its **reason** — the part not derivable from reading the code. Format: short, dated, immutable. Supersede via a new ADR; do not edit accepted ones.

## Index

| ID | Title | Status |
|---|---|---|
| [0001](ADR-0001-reuse-vala-panel-appmenu-registrar.md) | Reuse vala-panel-appmenu as Registrar | Accepted |
| [0002](ADR-0002-no-pid-on-toplevel-use-niri-ipc.md) | No `pid` on Quickshell `Toplevel` — use niri-IPC bridge | Accepted |
| [0003](ADR-0003-rust-sidecar-bridge.md) | Rust sidecar bridge over pure-QML | Accepted |
| [0004](ADR-0004-resolve-registrar-by-pid.md) | Resolve registrar entries by D-Bus connection PID | Accepted |
| [0005](ADR-0005-niri-only-v1.md) | niri-only in v1 | Accepted |
| [0006](ADR-0006-graceful-degradation.md) | Graceful degradation when no menu / no registrar | Accepted |
| [0007](ADR-0007-fixed-proxy-vs-quickshell-pr.md) | Fixed proxy from bridge over upstream Quickshell PR | Accepted |
| [0008](ADR-0008-popup-window-for-submenus.md) | `PopupWindow` for submenu rendering | Accepted |
| [0009](ADR-0009-debouncing-policy.md) | Focus debounce 75 ms, registrar churn 250 ms | Accepted |
| [0010](ADR-0010-no-keybind-intercept-v1.md) | No global Alt-F mnemonic intercept in v1 | Accepted |
| [0011](ADR-0011-home-manager-module.md) | Home-Manager submodule, not flake module | Accepted |
| [0012](ADR-0012-self-hosted-runner-only.md) | Self-hosted runner only; no public-CI matrix | Accepted (refined by 0013) |
| [0013](ADR-0013-runner-agnostic-ci.md) | Runner-agnostic labels + multi-runner pool | Accepted |
| [0014](ADR-0014-local-first-ci.md) | Local-first CI ("prechew") via lefthook | Accepted |
| [0015](ADR-0015-v01-fallback-only-shipping.md) | v0.1 ships fallback-only; menu rendering deferred to v0.2 | Accepted |
| [0016](ADR-0016-niri-event-stream-schema.md) | niri event-stream JSON schema | Accepted |
| [0017](ADR-0017-plugin-manifest-schema.md) | Plugin manifest schema (noctalia-shell v1) | Accepted |
| [0018](ADR-0018-bar-widget-api-contract.md) | Bar-widget API contract | Accepted |
| [0019](ADR-0019-always-visible-bar-widget.md) | Bar widget must always claim layout space | Accepted |
| [0020](ADR-0020-fixed-width-slot.md) | Bar widget slot must be fixed-width | Accepted |
| [0021](ADR-0021-fileview-text-call.md) | Quickshell.Io.FileView exposes content via `text()` (call) | Accepted |
| [0022](ADR-0022-bridge-owns-registrar.md) | Bridge owns the AppMenu Registrar bus name | Accepted |
| [0023](ADR-0023-dbusmenu-fetch-on-focus.md) | Fetch DBusMenu trees on focus change | Accepted |
| [0024](ADR-0024-atspi-substrate.md) | Replace DBusMenu/Registrar with AT-SPI menubar walker | Accepted |
| [0025](ADR-0025-cognitive-complexity-waiver.md) | Cognitive-complexity waiver for `find_app_for_pid` + `fetch_menu_tree` | Accepted (time-boxed) |
| [0026](ADR-0026-cyclonedx-1.6-syft-constraint.md) | CycloneDX 1.6 in v1.0.0-rc.x releases (syft constraint) | Accepted |
| [0027](ADR-0027-no-osconfig-in-hm-module.md) | Drop osConfig from HM module to avoid eval recursion | Accepted |
| [0028](ADR-0028-fr-003-accelerator-deferred.md) | FR-003 accelerator dispatch deferred (niri-ipc 26.4.0 gap) | Accepted |
| [0029](ADR-0029-learned-no-menubar-skip.md) | Learned no-menubar skip replaces hardcoded list | Accepted |
| [0030](ADR-0030-frame-scoped-menu-resolution.md) | Frame-scoped menu resolution by focused-window title | Accepted |

## Format

```
# ADR-NNNN — Title (max 60 chars)

Status: Proposed | Accepted | Superseded by ADR-XXXX
Date: YYYY-MM-DD

## Context
… 2-4 paragraphs framing the decision space …

## Decision
… 1-2 paragraphs stating the call …

## Consequences
- Positive: …
- Negative / cost: …
- Mitigation: …

## Alternatives considered
- A: …
- B: …

## References
- [link](…)
```
