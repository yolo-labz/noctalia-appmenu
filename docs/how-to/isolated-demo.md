# Capture a real two-app demo without touching the daily desktop

## Current gate — 21/09/2026

**No demo asset is published by this pass.** A real private Xvfb → niri →
Quickshell/QML noctalia-shell session loaded this repository's AppMenu widget;
the bridge acquired `org.noctalia.AppMenu` and followed the private niri socket.
Okular and Dolphin were launched with fresh HOME/config/cache directories.
However, the private accessibility bus could not activate
`org.a11y.atspi.Registry` (`NameHasNoOwner: unit failed`), so native menu/action
behavior was not verified. A fallback-only recording would not satisfy the demo.

Evidence: [isolated-plugin.txt](../evidence/swarm-2026-09-21/isolated-plugin.txt).
Noctalia's installed `noctalia v5.0.1` binary is a different shell from the QML
host this plugin requires. The probe instead used already-present, immutable
June 2026 Nix-store QML/Quickshell/bridge closures. This is **not** certification
of the Rust shell or a fresh verification of toolkit support.

## Reproduce the bounded runtime probe

From the repository root on the measured host:

```bash
python3 docs/evidence/swarm-2026-09-21/probe-isolated.py
```

This is a host-local forensic reproducer, not a portable launcher: exact store
paths in the script are part of its provenance. It requires those closures,
`niri`, `dbus-run-session`, Okular and Dolphin. It creates a short `/tmp/r5-*`
root, a private X server allocated via `-displayfd`, a new session bus and
HOME/XDG directories, and stops its own process group after 25 seconds. It
captures logs, **not pixels**. Store paths must be substituted with verified
matching packages on another machine. Never substitute the daily display/bus.

Two setup failures were distinguished and fixed in the probe:

- niri's X11 backend needed `libXcursor`, `libXi`, `libXrandr` on its private
  `LD_LIBRARY_PATH`;
- a long scratch path exceeded Unix socket `SUN_LEN`; a short private runtime
  directory allowed niri IPC to start. Do not reuse the daily runtime directory.

## Finish the capture (not yet executed)

1. Provision a **disposable VM/test login** with the QML noctalia-shell host,
   matching Quickshell, niri, this plugin/bridge, `at-spi2-core`, Qt6 Okular and
   Kate (or Dolphin), FFmpeg and a private-display recorder. No account sign-in,
   synced folders, notification sources, personal files or shared session bus.
   A VM's ordinary isolated user session can supply the AT-SPI activation that
   the nested `dbus-run-session` probe lacks. Alternatively provision the registry
   daemon explicitly on the private accessibility bus; do not use the host's.
2. Enable `QT_ACCESSIBILITY=1`; install the plugin in that disposable user's
   `~/.config/noctalia/plugins/noctalia-appmenu`. Set
   `plugins.json` → `states.noctalia-appmenu.enabled=true` and
   `settings.json` → `bar.widgets.left=[{"id":"plugin:noctalia-appmenu"}]`.
   Start the bridge and QML shell **inside that session only**.
3. Record tool versions, plugin revision, and real bus introspection before
   filming. In the isolated session:

   ```bash
   niri msg version
   qs --version
   noctalia-appmenu-bridge --version
   okular --version
   dolphin --version  # or kate --version
   busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress
   # Use the returned address, never the host's address:
   busctl --address="$PRIVATE_A11Y_ADDRESS" list
   busctl --user introspect org.noctalia.AppMenu /org/noctalia/AppMenu/Active
   ```

   Require a live `org.a11y.atspi.Registry`, then focus each app and inspect
   `$XDG_CACHE_HOME/noctalia-appmenu/active.json`: `source=atspi`, the correct
   app PID and populated menu children. `desktop-fallback` is not a pass.
4. Open only a synthetic one-page PDF and an empty demo directory/text file.
   Film app A focus → topbar File/View popup → harmless visible action (e.g.
   zoom); then app B focus → its changed menu → harmless action (e.g. toggle a
   panel). Keep the actual topbar and action result visible; do not render a
   mock bar or use the app's in-window menu as a substitute.
5. For an Xvfb-backed session, record the **allocated private X display only**:

   ```bash
   # PRIVATE_DISPLAY must come from the owned Xvfb -displayfd result.
   ffmpeg -f x11grab -video_size 1280x720 -framerate 30 \
     -i "$PRIVATE_DISPLAY" -t 25 -an -c:v libx264 -preset veryfast \
     -threads 8 -pix_fmt yuv420p docs/assets/demo-isolated.mp4
   ffprobe -v error -show_streams -show_format docs/assets/demo-isolated.mp4
   sha256sum docs/assets/demo-isolated.mp4
   ```

   This software-rendered Xvfb source has no DMA-BUF device; CPU encoding is
   intentional. Use a supported hardware encoder in a GPU-backed VM. No music
   or microphone/system audio. Label synthetic documents and any speed edits.
6. Review every frame for private content and verify two actual native menus
   and actions before adding a README link. Save source commands, versions,
   dimensions, duration, checksum and bus evidence next to the asset. Stop only
   owned processes; never restart desktop services or globally clean caches.

The recipe is a continuation procedure, **not a claim it passed**. The nested
probe proves plugin loading, not a completed two-app visual/action test.
