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
| [0029](ADR-0029-learned-no-menubar-skip.md) | Learned no-menubar skip replaces hardcoded list | Accepted |

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
