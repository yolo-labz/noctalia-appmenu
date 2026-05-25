# Summary

[Introduction](README.md)

# Architecture

- [Overview](architecture/overview.md)
- [DBusMenu pipeline](architecture/dbusmenu.md)
- [niri-IPC contract](architecture/niri-ipc.md)

# How-to

- [Local dev loop](how-to/dev-loop.md)
- [Fake registrar](how-to/fake-registrar.md)
- [Releasing](how-to/release.md)

# Reference

- [Configuration TOML](reference/config.md)

# Architecture decision records

- [Index](adr/README.md)
- [ADR-0001 — Reuse vala-panel-appmenu Registrar](adr/ADR-0001-reuse-vala-panel-appmenu-registrar.md)
- [ADR-0002 — niri-IPC bridge for PID](adr/ADR-0002-no-pid-on-toplevel-use-niri-ipc.md)
- [ADR-0003 — Rust sidecar bridge](adr/ADR-0003-rust-sidecar-bridge.md)
- [ADR-0004 — PID-keyed registrar mapping](adr/ADR-0004-resolve-registrar-by-pid.md)
- [ADR-0005 — niri-only v1](adr/ADR-0005-niri-only-v1.md)
- [ADR-0006 — Graceful degradation](adr/ADR-0006-graceful-degradation.md)
- [ADR-0007 — Fixed proxy from bridge](adr/ADR-0007-fixed-proxy-vs-quickshell-pr.md)
- [ADR-0008 — PopupWindow for submenus](adr/ADR-0008-popup-window-for-submenus.md)
- [ADR-0009 — Debouncing policy](adr/ADR-0009-debouncing-policy.md)
- [ADR-0010 — No keybind intercept v1](adr/ADR-0010-no-keybind-intercept-v1.md)
- [ADR-0011 — Home-Manager submodule](adr/ADR-0011-home-manager-module.md)
- [ADR-0012 — Self-hosted runner only](adr/ADR-0012-self-hosted-runner-only.md)
- [ADR-0013 — Runner-agnostic labels](adr/ADR-0013-runner-agnostic-ci.md)
- [ADR-0014 — Local-first CI (lefthook)](adr/ADR-0014-local-first-ci.md)
- [ADR-0015 — v0.1 fallback-only shipping](adr/ADR-0015-v01-fallback-only-shipping.md)
- [ADR-0016 — niri event-stream JSON schema](adr/ADR-0016-niri-event-stream-schema.md)
- [ADR-0017 — Plugin manifest schema](adr/ADR-0017-plugin-manifest-schema.md)
- [ADR-0018 — Bar-widget API contract](adr/ADR-0018-bar-widget-api-contract.md)
- [ADR-0019 — Bar widget claims layout space](adr/ADR-0019-always-visible-bar-widget.md)
- [ADR-0020 — Bar widget fixed-width slot](adr/ADR-0020-fixed-width-slot.md)
- [ADR-0021 — FileView exposes content via `text()`](adr/ADR-0021-fileview-text-call.md)
- [ADR-0022 — Bridge owns Registrar bus name](adr/ADR-0022-bridge-owns-registrar.md)
- [ADR-0023 — Fetch DBusMenu trees on focus change](adr/ADR-0023-dbusmenu-fetch-on-focus.md)
- [ADR-0024 — AT-SPI menubar walker substrate](adr/ADR-0024-atspi-substrate.md)
- [ADR-0025 — Cognitive-complexity waiver](adr/ADR-0025-cognitive-complexity-waiver.md)
- [ADR-0026 — CycloneDX 1.6 syft constraint](adr/ADR-0026-cyclonedx-1.6-syft-constraint.md)
- [ADR-0027 — Drop osConfig from HM module](adr/ADR-0027-no-osconfig-in-hm-module.md)
- [ADR-0028 — FR-003 accelerator dispatch deferred](adr/ADR-0028-fr-003-accelerator-deferred.md)
- [ADR-0029 — Learned no-menubar skip](adr/ADR-0029-learned-no-menubar-skip.md)
- [ADR-0030 — Frame-scoped menu resolution](adr/ADR-0030-frame-scoped-menu-resolution.md)
