# noctalia-appmenu

macOS-style global menu for [noctalia-shell](https://github.com/noctalia-dev/noctalia-shell) on [niri](https://github.com/YaLTeR/niri).

When you focus a Qt or GTK application, its menubar (`File`, `Edit`, `View`, …) appears in noctalia's topbar instead of inside the window. The behaviour mirrors macOS and Plasma's `appmenu` applet.

> **Status:** alpha. niri only. Qt6 support primary; GTK3/4 secondary; Electron / Firefox unsupported by design.

## How it works

Three pieces collaborate:

| Component | Role |
|---|---|
| **`appmenu-registrar`** (vala-panel-appmenu) | Standard `com.canonical.AppMenu.Registrar` D-Bus service. Apps register their menus here. |
| **`noctalia-appmenu-bridge`** (this repo, Rust) | Sidecar daemon. Subscribes to niri's IPC event-stream for focus changes, resolves registrar entries by D-Bus connection PID (because the registrar's `windowId` is an X11 XID — meaningless on Wayland), and re-exports the *active* window's menu at a stable D-Bus address: `org.noctalia.AppMenu.Active`. |
| **`noctalia-appmenu` plugin** (this repo, QML) | A noctalia bar widget that consumes `org.noctalia.AppMenu.Active` and renders the menu tree using Quickshell's existing `DBusMenu` consumer types. |

The bridge exists because Quickshell's `DBusMenuHandle` is `QML_UNCREATABLE` — there is no public way to bind QML to an arbitrary `(busName, objectPath)` pair. The bridge mirrors the active app's menu at a fixed path that the QML widget can attach to. See [ADR-0007](docs/adr/ADR-0007-fixed-proxy-vs-quickshell-pr.md).

## Install (NixOS / Home-Manager)

```nix
# flake.nix
{
  inputs.noctalia-appmenu.url = "github:yolo-labz/noctalia-appmenu";

  outputs = { home-manager, noctalia-appmenu, ... }: {
    homeConfigurations."pedro@desktop" = home-manager.lib.homeManagerConfiguration {
      modules = [
        noctalia-appmenu.homeManagerModules.default
        {
          programs.noctalia.plugins.appmenu = {
            enable = true;
            hideInWindowMenubar = true;   # sets QT_QPA_PLATFORMTHEME + GTK_MODULES
            registrar = "vala-panel";     # or "internal" for built-in (planned v0.2)
          };
        }
      ];
    };
  };
}
```

The module installs `appmenu-registrar`, the bridge binary, the QML plugin, and a `systemd --user` unit that brings the bridge up with `graphical-session.target`.

## Compatibility

| Toolkit | Status | Notes |
|---|---|---|
| Qt6 (KDE Frameworks apps, Anki, Telegram, Krita, qutebrowser) | Works | Requires `QT_QPA_PLATFORMTHEME=appmenu-qt5` in session env. |
| GTK3 / GTK4 | Works | Requires [`appmenu-gtk-module-wayland`](https://github.com/guiodic/appmenu-gtk-module-wayland) — mainline `appmenu-gtk-module` is broken on Wayland ([KDE bug 424485](https://bugs.kde.org/show_bug.cgi?id=424485)). |
| XWayland Qt5/GTK | Works | `xprop`-style XID is registered; bridge ignores it and matches by PID anyway. |
| Electron / Chromium | Unsupported | Chromium's [DBus appmenu code](https://github.com/chromium/chromium/commit/9c30fb37950c6a0a7ab2875f38ca3953e27963ae) exists but is flag-gated and brittle. |
| Firefox | Unsupported | No DBusMenu integration upstream. |

## Verify the install

```bash
# 1. Registrar is up
busctl --user list | grep com.canonical.AppMenu.Registrar

# 2. Bridge is up and publishing
busctl --user list | grep org.noctalia.AppMenu

# 3. Active proxy reflects focus
busctl --user introspect org.noctalia.AppMenu /org/noctalia/AppMenu/Active

# 4. Build provenance (pre-install)
gh attestation verify result/lib/libnoctalia-appmenu-bridge --owner yolo-labz
```

## Develop

```bash
nix develop                       # devShell: rust, cargo, alejandra, lefthook, gitleaks, qmllint
just bridge.test                  # unit tests
just plugin.lint                  # qmllint
just integration                  # niri --headless + fake registrar end-to-end
```

The `tools/fake-registrar/` Python helper publishes a canned DBusMenu tree at a known `(service, path)`, so tests don't need a real Qt or GTK app.

## Verification (release artefacts)

Every tagged release ships:

- Rust bridge binary built reproducibly with `SOURCE_DATE_EPOCH`, attested via `actions/attest-build-provenance@v2`.
- CycloneDX 1.7 + SPDX 2.3 SBOMs (via syft + `cyclonedx-rust-cargo`).
- Sigstore-keyless cosign signature on the binary blob.

Verify before installing manually:

```bash
gh attestation verify noctalia-appmenu-bridge --owner yolo-labz
```

See [SECURITY.md](SECURITY.md) for the full release-engineering posture and vulnerability-reporting process.

## Project layout

```
noctalia-appmenu/
├── plugin/                         # noctalia plugin (QML, ships as-is to ~/.config/noctalia/plugins/)
│   ├── manifest.json
│   └── BarWidget.qml
├── bridge/                         # Rust sidecar (com.canonical.AppMenu.Registrar consumer + active-app re-publisher)
│   ├── Cargo.toml
│   └── src/
├── nix/                            # flake-modules: package, devShell, HM module
├── specs/001-global-menu/          # speckit spec / plan / tasks
├── docs/adr/                       # architecture decision records
├── .specify/memory/constitution.md # project constitution
├── .claude/agents/                 # specialised agents (qml-architect, dbusmenu-expert, …)
└── .github/workflows/              # CI: ci, release, sonar, codeql, osv-scan, scorecard, reproducibility, actionlint, zizmor
```

## Acknowledgements

- [Quickshell](https://quickshell.org) by `outfoxxed` — DBusMenu consumer primitives.
- [noctalia-shell](https://github.com/noctalia-dev/noctalia-shell) — bar plugin host.
- [vala-panel-appmenu](https://github.com/rilian-la-te/vala-panel-appmenu) — registrar daemon.
- [niri](https://github.com/YaLTeR/niri) by Ivan "YaLTeR" Molodetskikh — IPC event-stream.

## License

[Apache-2.0](LICENSE).
