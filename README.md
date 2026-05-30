# noctalia-appmenu

macOS-style global menu for [noctalia-shell](https://github.com/noctalia-dev/noctalia-shell) on [niri](https://github.com/YaLTeR/niri).

When you focus a Qt or GTK application, its menubar (`File`, `Edit`, `View`, …) appears in noctalia's topbar instead of inside the window. The behaviour mirrors macOS and Plasma's `appmenu` applet.

> **Status:** v1.0.0 release candidate. AT-SPI substrate ([ADR-0024](docs/adr/ADR-0024-atspi-substrate.md)) replaces the v0.1 DBusMenu/Registrar pipeline. Qt6 primary; GTK4 secondary. Firefox/Electron supported via documented toolkit flags — see [Caveats](#caveats). niri only by design ([ADR-0005](docs/adr/ADR-0005-niri-only-v1.md)); compositor-abstraction door is open but unwired ([spec 004 FR-003](specs/004-project-completion/spec.md)).

## How it works

Three pieces collaborate:

| Component | Role |
|---|---|
| **`at-spi2-core`** (`services.gnome.at-spi2-core.enable = true`) | System-wide accessibility bus. Qt and GTK toolkits export their menu structure here when `QT_ACCESSIBILITY=1` / GTK a11y is active. |
| **`noctalia-appmenu-bridge`** (this repo, Rust) | Sidecar daemon. Subscribes to niri's IPC event-stream for focus changes, walks the focused application's AT-SPI accessibility tree to extract its menubar, and writes the snapshot to `~/.cache/noctalia-appmenu/active.json` (schema v=1, [ADR-0023](docs/adr/ADR-0023-dbusmenu-fetch-on-focus.md)) plus a fixed D-Bus address (`org.noctalia.AppMenu /org/noctalia/AppMenu/Active`). |
| **`noctalia-appmenu` plugin** (this repo, QML) | A noctalia bar widget that subscribes to the snapshot file and renders the menu strip in the topbar. Click events are forwarded back to the bridge, which invokes `AT-SPI DoAction` against the original accessible. |

The bridge exists because Quickshell's `DBusMenuHandle` is `QML_UNCREATABLE` — there is no public way to bind QML to an arbitrary `(busName, objectPath)` pair. The bridge mirrors the active app's menu at a fixed path that the QML widget can attach to. See [ADR-0007](docs/adr/ADR-0007-fixed-proxy-vs-quickshell-pr.md) for the original constraint and [ADR-0024](docs/adr/ADR-0024-atspi-substrate.md) for the substrate decision.

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
            # `registrar` is deprecated in v1.0.0 — the AT-SPI substrate
            # does not use it. The option is recognised for one cycle
            # so existing configs do not break; it is removed in v1.1.
          };
        }
      ];
    };
  };
}
```

The Home-Manager module installs the bridge binary, the QML plugin payload, the hardened `systemd --user` unit (`noctalia-appmenu-bridge.service`), and exports `QT_ACCESSIBILITY=1` into the session environment. System-level prerequisites (the AT-SPI bus) must be enabled at the NixOS layer separately — see [Verify the install](#verify-the-install) §1.

## Compatibility

| Toolkit | Status | Notes |
|---|---|---|
| Qt6 (KDE Frameworks apps, Anki, Telegram, Krita, qutebrowser) | Works | Requires `QT_ACCESSIBILITY=1` in session env (set automatically by the HM module). |
| GTK3 / GTK4 | Works | GTK4 `GtkPopoverMenuBar` (Nautilus 45+) exposes `MENU_BAR` with zero children when the menu is closed; the bridge then serves the [desktop fallback](#app-menu-fallback) (`source = "desktop-fallback"`). |
| XWayland Qt5/GTK | Works | AT-SPI walker is toolkit-agnostic; X11 windowing does not interfere. |
| Electron / Chromium | Fallback (full menu via flag) | No native menubar by default → the bridge serves the [desktop fallback](#app-menu-fallback) (app actions + window controls). Launch with `--force-accessibility` to expose the real menubar. |
| Firefox / Thunderbird | Fallback (full menu via flag) | No native menubar by default → [desktop fallback](#app-menu-fallback). Set `accessibility.force_disabled = 0` in `about:config` and restart for the real menubar. |
| libcosmic / Iced (`cosmic-files`, …) | Fallback only | No upstream AT-SPI export → [desktop fallback](#app-menu-fallback) always. Tracked at [#157](https://github.com/yolo-labz/noctalia-appmenu/issues/157). |

## Verify the install

Reproduces [`specs/004-project-completion/quickstart.md`](specs/004-project-completion/quickstart.md) condensed for a fresh-NixOS user. Time budget: ≤ 10 min from a clean shell to a working Anki menubar.

### 1. System prerequisites

You need:

- NixOS 25.05 or newer (any channel with `niri >= 25.04` and `at-spi2-core >= 2.50`).
- `niri` running as your Wayland compositor.
- `noctalia-shell >= 1.0.0` running.
- A login session that activates the `graphical-session.target` systemd user target (the default under `niri`).

In your NixOS configuration:

```nix
{
  services.gnome.at-spi2-core.enable = true;
}
```

Rebuild + activate:

```bash
sudo nixos-rebuild switch
```

Verify:

```bash
niri msg version | head -1
# expect: niri 25.xx
qs --version | head -1
# expect: quickshell 0.3.0 or newer
systemctl --user is-active graphical-session.target
# expect: active
systemctl --user is-active at-spi-dbus-bus.service
# expect: active
busctl --user list | grep org.a11y.Bus
# expect: org.a11y.Bus  :1.NN  ...
```

If any check fails, fix the underlying prerequisite before proceeding.

### 2. Plugin enablement (Home-Manager)

Add the input + module as shown in [Install](#install-nixos--home-manager). Rebuild Home-Manager:

```bash
home-manager switch
# or, for a flake-bound HM-on-NixOS host:
nh os switch .
```

The rebuild output should mention:

- `noctalia-appmenu-bridge` binary installed under `~/.nix-profile/bin/` (or equivalent).
- Plugin payload at `~/.config/noctalia/plugins/noctalia-appmenu/`.
- Systemd user unit `noctalia-appmenu-bridge.service` enabled.
- `QT_ACCESSIBILITY=1` exported in your session env.

If you forgot to enable `services.gnome.at-spi2-core` system-wide, the HM module emits an assertion error (or `lib.warn` at evaluation) telling you which knob to set.

### 3. Start the bridge + reload noctalia

```bash
systemctl --user start noctalia-appmenu-bridge.service
systemctl --user status noctalia-appmenu-bridge.service
# expect: Active: active (running)

qs -c noctalia-shell ipc reload
# OR:
systemctl --user restart noctalia-shell.service
```

You should now see the appmenu slot in the noctalia topbar (initially empty when no a11y-aware app is focused).

### 4. Verify with a real Qt6 app

```bash
anki &
```

Within ≤ 200 ms of Anki receiving keyboard focus, the appmenu slot renders Anki's menu strip (`File`, `Edit`, `View`, `Tools`, `Help`, `Ankimon`, `AnKing`). Clicking `File` opens a popup matching Anki's in-window menu; clicking `File → Export…` activates the action in Anki, exactly as if the in-window menu had been clicked.

If the menu does not appear:

```bash
journalctl --user -u noctalia-appmenu-bridge.service -n 100 --no-pager
# Look for [atspi] lines: "found app for pid", "fetched menubar", "no app for pid", …
```

| Symptom | Diagnosis | Fix |
|---|---|---|
| `[atspi] no app found for pid` | App did not register on the a11y bus | Verify `QT_ACCESSIBILITY=1` is set in the app's environment: `tr '\0' '\n' < /proc/$(pidof anki)/environ \| grep QT_`. |
| Bridge log shows menu fetched but bar is empty | Plugin not loaded by noctalia-shell | `qs -c noctalia-shell ipc reload`; check `journalctl --user -u noctalia-shell.service` for plugin-load errors. |
| Submenu (`File → Open Recent`) does not open | spec 004 FR-010 regression | Capture `journalctl` output and file a bug. |

### 5. Verify the release artefact (optional, recommended)

After upgrading to `v1.0.0` (or installing from a release tarball):

```bash
gh release download v1.0.0 --repo yolo-labz/noctalia-appmenu --pattern 'noctalia-appmenu-bridge*'
gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz
# expect: Loaded digest sha256:...
# expect: ✓ Verification succeeded!

gh release download v1.0.0 --repo yolo-labz/noctalia-appmenu --pattern 'sbom.cdx.json'
jq '.bomFormat, .specVersion' sbom.cdx.json
# expect: "CycloneDX"
# expect: "1.7"
```

A second build from source should produce a byte-identical binary:

```bash
nix build github:yolo-labz/noctalia-appmenu/v1.0.0#noctalia-appmenu-bridge
sha256sum result/bin/noctalia-appmenu-bridge ./noctalia-appmenu-bridge
# expect: identical hashes
```

## App-menu fallback

Most modern apps expose **no machine-readable menubar** on Wayland: libcosmic/Iced
(`cosmic-files`), Electron without `--force-accessibility` (Obsidian, VS Code, Slack),
Chromium/Chrome, Firefox, and GTK4 popover-only apps all register nothing usable on
the AT-SPI bus. For these the bridge does **not** go blank — it serves an honest,
identity-derived **fallback menu** (`source = "desktop-fallback"` in `active.json`),
built from:

- the app's freedesktop `.desktop` entry — display **Name** and any `[Desktop Action]`s
  (e.g. Chrome's *New Window* / *New Incognito Window*, Firefox's *Profile Manager*),
- a **New Window** launch item when the entry declares no actions,
- a **Window** submenu of real niri controls (Close / Toggle Fullscreen / Toggle
  Floating / Move to Next-or-Previous Workspace),
- **Quit**, mapped to niri *close-window* (never `SIGKILL`).

Every item maps to a real action — `.desktop` actions launch the app's own `Exec`
(parsed to argv, **never** via a shell; field codes stripped), window controls call
`niri msg action`. It is honest about *not* being the app's in-window menu: the
`source` field says `desktop-fallback`, not `atspi`. This **supersedes** the v1.0.2
"honest-or-hidden" behaviour (the bar used to collapse to nothing); see
[ADR-0031](docs/adr/ADR-0031-desktop-fallback.md).

Apps that **do** expose a native menubar via AT-SPI (Qt6 / GTK with the a11y bridge
loaded — Anki, Okular, Kate, Krita, GIMP, LibreOffice) are unaffected: they always
get the real menu (`source = "atspi"`); the fallback never shadows a native menubar.

To opt back into blank-when-no-native-menu, set `desktop_fallback = false` in
`~/.config/noctalia-appmenu-bridge/config.toml`.

## Caveats

Known limitations. Each item is tracked against a follow-up spec or ADR.

- **The fallback is not the app's real menu.** `desktop-fallback` surfaces launch
  actions + window controls, not the app's File/Edit/View tree. For the real menubar
  on Electron/Chromium/Firefox, use the per-app flags below. Native, machine-readable
  menus are an upstream-toolkit responsibility the bridge cannot synthesise.
- **Firefox / Thunderbird.** A native menubar surfaces only when `accessibility.force_disabled = 0` is set in `about:config` and the browser is restarted; otherwise the [desktop fallback](#app-menu-fallback) applies. Mozilla iterated on the Wayland a11y export through 2025–2026; the regression-free default is still partial. See [`specs/004-project-completion/research.md` §7](specs/004-project-completion/research.md) for the upstream status.
- **Electron apps.** VS Code, Slack, Discord, etc. expose a native menubar only when launched with `--force-accessibility`; otherwise the [desktop fallback](#app-menu-fallback) applies. Wrap the launch command or set the flag in your `.desktop` file. Chromium's native AT-SPI export is "quite good" but flag-gated.
- **Multi-monitor menubar duplication.** `v1.0.0` renders the focused-output menu only — no duplication across monitors. Deferred to v2 ([spec 004 §Out of scope](specs/004-project-completion/spec.md)).
- **Alt-letter mnemonics / global Alt-F intercept.** Pressing `Alt-F` does NOT open the File menu via the appmenu. The in-window menu (if visible) still receives the keystroke. Deferred to v2 per [ADR-0010](docs/adr/ADR-0010-no-keybind-intercept-v1.md) — no clean Quickshell hook exists for global keybind interception at v1.
- **GTK4 popover menubars.** GTK4 apps using `GtkPopoverMenuBar` (Nautilus 45+, some GNOME apps) expose menu structure only when the menu is open in-window. When the walk finds an empty menubar the bridge serves the [desktop fallback](#app-menu-fallback) instead.
- **libcosmic / Iced apps.** System76's libcosmic toolkit (`cosmic-files`, `cosmic-edit`, `cosmic-term`, `cosmic-settings`) and standalone Iced apps have no AT-SPI implementation upstream. They register on the session bus but never join `org.a11y.atspi.Registry`, so the bridge cannot enumerate their menus and serves the [desktop fallback](#app-menu-fallback). Tracked at [#157](https://github.com/yolo-labz/noctalia-appmenu/issues/157) / [pop-os/libcosmic accessibility](https://github.com/pop-os/libcosmic/issues?q=accessibility+OR+atspi); revisit when libcosmic ships AccessKit/AT-SPI export.
- **AT-SPI bus restart.** If `at-spi-bus-launcher` crashes and is restarted by D-Bus activation, the bridge re-flips `org.a11y.Status.IsEnabled = true` on its next focus-change attempt and resumes within ≤ 5 s. The QML widget collapses to a zero-paint stable slot during the gap (no error spam, no crash) — see [spec 004 Scenario 5](specs/004-project-completion/spec.md).
- **niri reload.** `niri msg reload-config` may produce a ≤ 2 s blank-bar gap while the bridge reconnects; the backoff resets to its floor after any cleanly-EOF'd session ≥ 30 s, so successive reloads do not compound ([spec 004 FR-001](specs/004-project-completion/spec.md)).
- **Compositor support.** niri is the only supported compositor at v1.0.0. Hyprland / Sway / KWin / COSMIC focus tracking is deferred to v2 ([ADR-0005](docs/adr/ADR-0005-niri-only-v1.md)); the bridge's focus-tracker abstraction door (`FocusSink` trait) is open but unwired.

## Develop

```bash
nix develop                       # devShell: rust, cargo, alejandra, lefthook, gitleaks, qmllint
just bridge.test                  # cargo test --all-features --locked
just plugin.lint                  # qmllint (SARIF emit + upload runs in CI — FR-024)
just integration                  # niri --headless + AT-SPI fixture end-to-end (Lane A)
```

The bridge integration test (`bridge/tests/atspi_integration.rs`, FR-022) walks a fake AT-SPI registry stub and asserts the JSON snapshot shape end-to-end. CI runs it on every PR; locally you can run `cargo test --test atspi_integration` from `bridge/`.

## Verification (release artefacts)

Every tagged release ships:

- Rust bridge binary built reproducibly with `SOURCE_DATE_EPOCH`, attested via [`actions/attest-build-provenance`](https://github.com/actions/attest-build-provenance) (v4 family).
- CycloneDX 1.7 + SPDX 2.3 SBOMs (via `syft` + [`cyclonedx-rust-cargo`](https://github.com/CycloneDX/cyclonedx-rust-cargo)).
- GitHub-native build-provenance attestation. Verify with a single command:

```bash
gh attestation verify noctalia-appmenu-bridge --owner yolo-labz
```

See [SECURITY.md](SECURITY.md) for the full release-engineering posture and vulnerability-reporting process.

## Project layout

```
noctalia-appmenu/
├── plugin/                         # noctalia plugin (QML; ships to ~/.config/noctalia/plugins/)
│   ├── manifest.json
│   ├── BarWidget.qml
│   └── AppmenuPopupWindow.qml
├── bridge/                         # Rust sidecar (AT-SPI walker + fixed-proxy publisher)
│   ├── Cargo.toml
│   ├── src/
│   └── tests/
├── nix/                            # flake modules: package, devShell, HM module
├── specs/004-project-completion/   # v0.3 → v1.0.0 roadmap (umbrella)
├── specs/008-ci-quality-docs/      # Lane D — CI + quality gate + docs
├── docs/adr/                       # architecture decision records (1–25)
├── .specify/memory/constitution.md # project constitution
├── .claude/agents/                 # specialised agents (qml-architect, dbusmenu-expert, …)
└── .github/workflows/              # CI: ci, release, sonar, codeql, osv-scan, scorecard, reproducibility, actionlint, zizmor
```

## Acknowledgements

- [Quickshell](https://quickshell.org) by `outfoxxed` — QML widget toolkit.
- [noctalia-shell](https://github.com/noctalia-dev/noctalia-shell) — bar plugin host.
- [at-spi2-core](https://gitlab.gnome.org/GNOME/at-spi2-core) — Linux accessibility bus.
- [niri](https://github.com/YaLTeR/niri) by Ivan "YaLTeR" Molodetskikh — IPC event-stream.

## License

[Apache-2.0](LICENSE).
