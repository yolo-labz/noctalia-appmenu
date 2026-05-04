# ADR-0003 — Rust sidecar bridge over pure-QML

Status: Accepted
Date: 2026-05-04

## Context

The natural ambition is a pure-QML noctalia plugin with zero sidecar processes. Two facts kill that path:

1. Quickshell's `DBusMenuHandle` is `QML_UNCREATABLE`. From QML you cannot bind to an arbitrary `(busName, objectPath)` pair — the type only constructs internally for `SystemTrayItem`. Verified at `quickshell/src/dbus/dbusmenu/dbusmenu.hpp:121-194`.
2. Bus-name acquisition (claiming `org.noctalia.AppMenu`) and `GetConnectionUnixProcessID` calls are awkward to impossible from QML in a way that's robust to Quickshell version drift.

So we ship a sidecar. Choice of language: Rust, Go, Python, C++. Pinned by these factors:

- Standard for yolo-labz polyglot plugins. `wa` is Go; we have the Go infra running. But Go's D-Bus libraries (`godbus/dbus`) are mature but heavier API than `zbus`.
- `zbus` (Rust) is the cleanest async D-Bus library in the open-source ecosystem in 2026; supports proxy generation from XML.
- `niri-ipc` ships a Rust crate; Go has nothing comparable (would be hand-rolled JSON line parsing).
- Single-binary deploy. Rust's MSRV plus `cargo build --release` produces a small statically-linked binary; matches the bridge's runtime profile.

## Decision

Sidecar bridge written in Rust 1.81+, using `zbus`, `niri-ipc`, `tokio`, `serde`. Async runtime: tokio (matches `zbus`'s default).

## Consequences

- **Positive:** Type-safe D-Bus, type-safe niri-IPC. Statically linked release binary. Reproducible builds via `SOURCE_DATE_EPOCH`.
- **Negative:** Adds Rust as a yolo-labz language (alongside Go for `wa`, Python for `kokoro-speakd`, shell for `claude-mac-chrome`). Adds a new release-engineering pipeline branch.
- **Mitigation:** Cargo's reproducibility story is mature (cargo-deny, cargo-vet, OSV-Scanner). We piggyback on the same Scorecard / CodeQL / SonarQube pipelines.

## Alternatives considered

- **Go bridge:** Smaller learning curve (we have Go infra). Rejected because `niri-ipc` Go support is non-existent; we would maintain our own JSON parser against an unstable schema.
- **Python bridge:** `dbus-next` is fine; no compelling story for static distribution. Cold-start latency would be visible (~80 ms vs Rust's ~5 ms). Rejected.
- **C++ bridge:** Too much foot-gun for a one-person project. Rejected.

## References

- [zbus docs](https://docs.rs/zbus/latest/zbus/)
- [niri-ipc docs](https://docs.rs/niri-ipc/)
- `quickshell/src/dbus/dbusmenu/dbusmenu.hpp:121-194`
