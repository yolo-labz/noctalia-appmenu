# ADR-0015 — v0.1 ships fallback-only; full menu rendering deferred to v0.2

Status: Accepted
Date: 2026-05-04
Related: ADR-0007 (fixed proxy from bridge)

## Context

ADR-0007 picked the path of "the bridge re-exports the active app's menu at a fixed `(serviceName, objectPath)` so QML can attach a `DBusMenuHandle` to a constant address." Implementation was split into two halves:

- **First half (shipped in v0.1):** the bridge owns `org.noctalia.AppMenu`, exposes `(busName, objectPath, appId, title)` properties at `/org/noctalia/AppMenu/Active`, and updates them as focus changes.
- **Second half (deferred):** the bridge actually mirrors the active app's `com.canonical.dbusmenu` interface server-side, so a fixed-path DBusMenuHandle becomes consumable from QML.

In hindsight, the "first half" is **insufficient by itself**. The QML widget cannot construct `DBusMenuHandle` from QML (`QML_UNCREATABLE`), so even with the (busName, objectPath) properties, there's no way to render the upstream menu tree.

Three real options:

1. **Ship the bridge mirror now (v0.1)** — adds ~300-400 LoC of Rust implementing `com.canonical.dbusmenu` server + protocol-fidelity testing.
2. **Submit a public `DBusMenuHandle.create(service, path)` factory upstream to Quickshell** — depends on outfoxxed's roadmap (issue #170 says "not planned soon").
3. **Ship v0.1 as fallback-only** — render the focused app's `app_id` in the bar; full menu rendering lands in v0.2 with the bridge mirror.

## Decision

Option 3 for v0.1. Cut the scope of the v0.1 plugin to render the focused app's name only. Defer the full menu tree to v0.2 (spec 002 — to be filed).

The plugin's `BarWidget.qml` is rewritten to drop the broken `DBusMenuHandle` binding and render only the bridge-published `appId`. The repository ships the full bridge plumbing (focus tracking, registrar consumer, active proxy) so v0.2 just adds a mirror module to the bridge and updates the widget.

## Consequences

- **Positive:** v0.1 actually works end-to-end (compiles, plugin loads, bar shows app names). Sets up infrastructure for v0.2.
- **Positive:** Decouples the v0.1 release from Quickshell upstream cooperation.
- **Negative:** v0.1 doesn't deliver the headline "macOS-style menubar" feature. README must be honest about this.
- **Negative:** Users on v0.1 will have a worse experience than waiting for v0.2.
- **Mitigation:** README updated to flag v0.1 as "alpha — app-name-only rendering; full menu in v0.2." Spec 002 captures the v0.2 work.

## Alternatives considered

- **Option 1 (ship mirror now):** Tempting but moves v0.1 by another 1-2 weeks. Better to ship the infrastructure and iterate.
- **Option 2 (Quickshell PR):** Out of our control. We've already filed quickshell-mirror/quickshell#170.
- **Hold v0.1 indefinitely until full feature is ready:** Rejected. Operational reality is that "in-flight forever" projects atrophy.

## Rollout

1. Land this PR — `BarWidget.qml` becomes fallback-only; submenu components stay in tree but unused.
2. File spec 002 (v0.2 — bridge DBusMenu mirror).
3. Tag v0.1.0 once all in-flight PRs merge.
4. Begin v0.2 work after v0.1.0.

## References

- ADR-0007 — Fixed proxy vs upstream Quickshell PR
- Quickshell `DBusMenuHandle` source: `src/dbus/dbusmenu/dbusmenu.hpp:121-194` (`QML_UNCREATABLE`)
- [quickshell-mirror/quickshell#170](https://github.com/quickshell-mirror/quickshell/issues/170)
