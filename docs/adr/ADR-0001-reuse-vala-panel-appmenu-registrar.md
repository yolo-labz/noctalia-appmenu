# ADR-0001 — Reuse vala-panel-appmenu as Registrar

Status: Accepted
Date: 2026-05-04

## Context

A working `com.canonical.AppMenu.Registrar` daemon is required: GTK and Qt apps publish their menubars to it, and the bridge consumes from it. We have three realistic registrar choices:

1. `vala-panel-appmenu`'s `appmenu-registrar` (standalone, GPL-3, packaged in nixpkgs).
2. KDE's `appmenu-kded` module (drags `kf6/kded6` into the closure — heavy).
3. Roll our own minimal Rust registrar inside the bridge.

## Decision

Reuse `vala-panel-appmenu`'s `appmenu-registrar` as a separate user systemd unit. Do not reimplement.

## Consequences

- **Positive:** Battle-tested. Compatible with every Qt/GTK client that already speaks the Canonical AppMenu protocol. Zero protocol-bug surface for us.
- **Negative:** Adds a runtime dependency on a project whose upstream is dormant (last release 25.04 Debian, May 2025). One more service in the user's session.
- **Mitigation:** ADR-0007 leaves the door open to a built-in registrar in v0.2 if `vala-panel-appmenu` becomes unmaintained. The bridge does not couple itself to anything specific to vala-panel — it only speaks the standard Registrar interface.

## Alternatives considered

- **`appmenu-kded`:** Pulls KF6 + kded6 (~30 MB closure). Tightly coupled to KDE session bus conventions. Rejected.
- **Built-in registrar in our bridge:** Tempting (saves a process), but expands our bug surface to "every Qt/GTK client compatibility quirk ever filed against `appmenu-registrar`" on day one. Rejected for v1; reconsidered in v0.2.

## References

- [vala-panel-appmenu](https://github.com/rilian-la-te/vala-panel-appmenu)
- [tetzank/qmenu_hud Registrar reimpl](https://github.com/tetzank/qmenu_hud) — reference for protocol semantics
- [com.canonical.AppMenu.Registrar.xml](https://github.com/KDE/plasma-workspace/blob/master/appmenu/com.canonical.AppMenu.Registrar.xml)
