# Quickstart: verify the install (v1.0.0 candidate)

**Spec:** `specs/004-project-completion/spec.md` (FR-028)
**Audience:** a Pedro-class user on a fresh NixOS host
**Time budget:** ≤ 10 min from clean shell to working Anki menubar
**Generated:** 2026-05-12

> This recipe is the canonical acceptance test for `v1.0.0`. The README ships an edited version of this document; the AT-SPI integration test in CI (FR-022) replays its automatable steps headlessly.

---

## 0. Prerequisites

You need:

- NixOS 25.05 or newer (any channel with `niri >= 25.04` and `at-spi2-core >= 2.50`)
- `niri` running as your Wayland compositor
- `noctalia-shell >= 1.0.0` running
- A login session that activates the `graphical-session.target` systemd user target (default under `niri`)

Verify in a terminal:

```bash
niri msg version | head -1
# expect: niri 25.xx
qs --version | head -1
# expect: quickshell 0.3.0 or newer
systemctl --user is-active graphical-session.target
# expect: active
```

If any check fails, fix the underlying prerequisite before proceeding.

---

## 1. System-level enablement (NixOS configuration)

The AT-SPI substrate (ADR-0024) requires the system-wide accessibility bus. Edit your NixOS config (e.g. `/etc/nixos/configuration.nix` or your flake's host module):

```nix
{
  services.gnome.at-spi2-core.enable = true;
}
```

This installs `at-spi2-core` and starts `at-spi-dbus-bus.service` on boot. Required: without it the bridge has nothing to walk.

Rebuild + activate:

```bash
sudo nixos-rebuild switch
```

Verify:

```bash
systemctl --user is-active at-spi-dbus-bus.service
# expect: active
busctl --user list | grep org.a11y.Bus
# expect: org.a11y.Bus  :1.NN  ...
```

---

## 2. Home-Manager enablement (the plugin itself)

Add the flake input + module to your Home-Manager config:

```nix
{
  inputs.noctalia-appmenu.url = "github:yolo-labz/noctalia-appmenu/v1.0.0";
}
```

```nix
# In your HM config:
{
  imports = [ inputs.noctalia-appmenu.homeManagerModules.default ];

  programs.noctalia.plugins.appmenu = {
    enable = true;
    # registrar = "none";    # default in v1; older configs may still have "vala-panel" — safe to remove
    # hideInWindowMenubar = false;  # set to true if you want Qt/GTK in-window menus suppressed
  };
}
```

Rebuild + activate Home-Manager:

```bash
home-manager switch
```

You should see, in the rebuild output:

- The `noctalia-appmenu-bridge` package installed under `~/.nix-profile/bin/` (or equivalent).
- The plugin payload at `~/.config/noctalia/plugins/noctalia-appmenu/`.
- The systemd user unit `noctalia-appmenu-bridge.service` enabled.
- `QT_ACCESSIBILITY=1` exported in your session env (via `~/.bashrc.d/`, `~/.config/environment.d/`, or your shell init — depending on HM-on-NixOS vs HM-on-other).

If `services.gnome.at-spi2-core.enable` was forgotten, HM emits an assertion error or warning at this step.

---

## 3. Start (or restart) the noctalia session

The bridge runs as a user service bound to `graphical-session.target`. After `home-manager switch`, start it:

```bash
systemctl --user start noctalia-appmenu-bridge.service
systemctl --user status noctalia-appmenu-bridge.service
# expect: Active: active (running)
```

Reload noctalia-shell so the plugin payload is picked up:

```bash
qs -c noctalia-shell ipc reload
# OR restart noctalia-shell entirely:
systemctl --user restart noctalia-shell.service
```

You should now see the appmenu slot in the noctalia top bar (initially empty when no a11y-aware app is focused).

---

## 4. Verify with a Qt6 reference app

Open Anki (or kate, or dolphin):

```bash
anki &
```

Within ≤ 200 ms of Anki receiving keyboard focus, the appmenu slot in the top bar should render Anki's menu strip (`File`, `Edit`, `View`, `Tools`, `Help`, `Ankimon`, `AnKing`).

Click `File` — a popup opens with Anki's File-menu items. Hover `File → Export…`, click — Anki responds as if the in-window menu was clicked.

If the menu does not appear:

```bash
journalctl --user -u noctalia-appmenu-bridge.service -n 100 --no-pager
# Look for [atspi] log lines: "found app for pid", "fetched menubar", etc.
```

Common failure modes (all addressed in the bridge per spec 004):

| Symptom | Diagnosis | Fix |
|---|---|---|
| `[atspi] no app found for pid` | App did not register on the a11y bus | Verify `QT_ACCESSIBILITY=1` is set in the app's environment (run `tr '\0' '\n' < /proc/$(pidof anki)/environ \| grep QT_`). |
| Bridge log shows menu fetched but bar is empty | Plugin not loaded by noctalia-shell | `qs -c noctalia-shell ipc reload`; check `journalctl --user -u noctalia-shell.service` for plugin-load errors. |
| Submenu (`File → Open Recent`) does not open | Spec 004 FR-010 regression | Capture `journalctl` output + filed as a bug. |

---

## 5. Verify the release artefact (optional, recommended)

After upgrading to `v1.0.0` (or installing from a release tarball):

```bash
# Download the binary + attestation
gh release download v1.0.0 --repo yolo-labz/noctalia-appmenu --pattern 'noctalia-appmenu-bridge*'

# Verify the GitHub-native build provenance
gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz
# expect: Loaded digest sha256:...
# expect: ✓ Verification succeeded!

# Inspect the SBOM (CycloneDX 1.7)
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

---

## 6. Documented caveats (FR-029)

- **Firefox.** Firefox menus surface only when `accessibility.force_disabled = 0` is set in `about:config` AND Firefox is restarted. The default is partial in 2026 — Mozilla is iterating; see [fazm.ai 2026 Linux GUI landscape](https://fazm.ai/blog/agentic-infrastructure-landscape-2026-linux-desktop-gui).
- **Electron apps.** VS Code, Slack, Discord, etc. expose menus only when launched with `--force-accessibility`. Set this via a wrapper script or your app's launch command.
- **Multi-monitor.** v1.0.0 shows the focused-output menu only — no duplication across monitors. Deferred to v2.
- **Alt-key mnemonics.** Pressing `Alt-F` does NOT open the File menu via the appmenu. The in-window menu (if visible) still receives the keystroke. Deferred to v2 per ADR-0010.
- **GTK4 popover menubars.** GTK4 apps using `GtkPopoverMenuBar` (Nautilus 45+) expose menu structure only when the menu is open in-window. The bridge falls back to a `.desktop`-derived pseudo-menu for these. FR-004.

---

## 7. Soak test (manual; SC-005)

Run for 7 days of normal use with at least the 5-app reference set running occasionally (Anki, kate, dolphin, plus 2 GTK apps from {gimp, inkscape, nautilus}). At day 7:

```bash
# Memory check
ps -o pid,rss,cmd -p $(pgrep -f noctalia-appmenu-bridge)
# expect: rss < 51200 (50 MB)

# Crash count
journalctl --user -u noctalia-appmenu-bridge.service --since "7 days ago" \
  | grep -c -i 'panic\|crash\|abort'
# expect: 0

# Focus-tracking regressions
# (Manual: did the menubar fail to update at any point during the week?)
# expect: no observable failures
```

Pass = SC-005 satisfied. Sign off in the v1.0.0 release notes.

---

## 8. Reporting issues

If the recipe fails at any step:

1. Capture `journalctl --user -u noctalia-appmenu-bridge.service -n 500` + `journalctl --user -u noctalia-shell.service -n 500`.
2. Capture `nix flake metadata github:yolo-labz/noctalia-appmenu` (version pin).
3. Capture environment: `tr '\0' '\n' < /proc/$(pgrep -f noctalia-appmenu-bridge)/environ | grep -E '^(QT_|GTK_|XDG_)'`.
4. Open an issue at `https://github.com/yolo-labz/noctalia-appmenu/issues/new` with the three captures + the step that failed.
