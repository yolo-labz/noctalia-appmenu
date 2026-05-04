# ADR-0007 — Fixed proxy from bridge over upstream Quickshell PR

Status: Accepted
Date: 2026-05-04

## Context

Quickshell's `DBusMenuHandle` is `QML_UNCREATABLE`. Two paths to consume an arbitrary `(busName, objectPath)` from QML:

A. Submit a PR upstream adding a public `DBusMenuHandle.create(service, path)` factory. Right thing to do for the ecosystem; takes weeks; depends on the upstream maintainer's roadmap (issue #170 says "not planned").

B. Have the bridge re-export `com.canonical.dbusmenu` proxy at a *fixed* `(serviceName, objectPath)` — `org.noctalia.AppMenu` / `/org/noctalia/AppMenu/Active` — that mirrors the active app's menu. The QML side then attaches to a constant address, which Quickshell's existing `SystemTrayItem`-style consumer pattern can handle.

## Decision

Path B for v1. The bridge re-exports a fixed proxy. v0.2 may revisit path A.

## Consequences

- **Positive:** Ships now. No upstream dependency.
- **Negative:** Bridge has to implement the full `com.canonical.dbusmenu` interface as a server, mirroring whatever the active app published. Latency on focus change is the bridge's problem, not the QML widget's.
- **Mitigation:** `zbus` macro support for `dbus_interface` makes the server-side tractable. Mirroring is mostly a forward — `LayoutUpdated` / `ItemsPropertiesUpdated` signals are passed through with rewritten paths.

## Alternatives considered

- **Path A only:** Sole dependency on outfoxxed's roadmap. Maintainer said "not planned soon." Rejected.
- **Fork Quickshell:** Owning a fork forever to maintain one factory is disproportionate. Rejected.
- **Inject QML at runtime via Quickshell's reload mechanism:** Brittle, and still doesn't unblock `DBusMenuHandle`'s C++ side. Rejected.

## References

- [quickshell-mirror/quickshell#170](https://github.com/quickshell-mirror/quickshell/issues/170)
- [zbus dbus_interface attribute](https://docs.rs/zbus/latest/zbus/attr.dbus_interface.html)
