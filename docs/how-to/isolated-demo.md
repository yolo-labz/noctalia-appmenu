# Isolated legacy demonstration — 21/09/2026

This is **partial evidence**, not current Rust-host support or a completed
mouse-driven demo. Actual native Okular/Dolphin topbar menus and a real
`About Okular` action through the bridge CLI were observed. The topbar Help
button logged its click but no dropdown was visible; that remains unfulfilled.

## Root cause and repair

The original private probe reproduces `NameHasNoOwner: unit failed` for
`org.a11y.atspi.Registry`. Installed at-spi2-core 2.60.6's launcher chooses a
broker/systemd activation path by default. Setting only
`ATSPI_DBUS_IMPLEMENTATION=dbus-daemon` makes the same probe activate Registry
successfully. The immutable accessibility.conf names its store-local
accessibility-services directory; Registry's service Exec points at
`libexec/at-spi2-registryd --use-gnome-session`. The session service for
`org.a11y.Bus` includes `SystemdService=at-spi-dbus-bus.service`, but our
`dbus-run-session` is not a systemd user bus. No live service/config was changed.
Do not infer that the failed broker successfully modified the live user bus.

## Recreate

From this worktree, on the measured host with the pinned installed closures:

```bash
python3 -u docs/evidence/swarm-2026-09-21/probe-isolated.py --capture
# In another terminal, use ONLY the printed /tmp/r5-* owned runtime, after
# the private shell loads (~20 seconds):
python3 docs/evidence/swarm-2026-09-21/capture-private.py /tmp/r5-PRINTED \
  docs/assets/isolated-legacy-demo --prepare --app okular --demo
```

The first helper allocates Xvfb with `-displayfd`, launches private niri/session
bus/HOME/config/cache, enables accessibility and disables audio access. It
stops its process group and Xvfb after 90 seconds (25 without `--capture`).
The second checks the owned Xvfb PID and display receipt, uses **R5_X_DISPLAY**,
not niri's rewritten DISPLAY (its nested Xwayland), and asserts `source=atspi`,
PID match and nonempty children before recording. It resizes/focuses only the
private X11 niri window, hides Dolphin Places (host mount names must not be
filmed), enables Dolphin's menubar, and uses empty profiles/no documents.
The cache seeds the legacy changelog acknowledgement; telemetry remains false.

The recorded run predates this acknowledgement seed: its private first-run
notice was dismissed with the checkbox left OFF, then Dolphin F9/Ctrl+M and
Okular focus were applied. The final replay check is separately logged; do not
confuse the observed recording with a claim that all launcher paths are tested.

`--demo` records 16 seconds, calls the same bridge `atspi-click` used by QML
on the observed About Okular accessible, asserts that its real dialog appears,
then switches to Dolphin and reasserts its native menu. It does **not** claim
that a popup row was clicked. Dialog focus briefly produces fallback labels.
The general system FFmpeg lacks x11grab; the helper pins installed FFmpeg-full
9.0.1. Software Xvfb frames use CPU x264, eight threads, no audio or speed edit.

```bash
ffmpeg -i docs/assets/isolated-legacy-demo/demo.mp4 -filter_complex \
 '[0:v]fps=5,scale=768:-1,split[a][b];[a]palettegen[p];[b][p]paletteuse' \
 -filter_complex_threads 4 docs/assets/isolated-legacy-demo/demo.gif
```

## Evidence and cleanup

[Assets](../assets/isolated-legacy-demo/): MP4 1280×720, 10 fps, 16 s;
GIF 768×432, 5 fps; poster 1280×720; no audio. `*-active.json`, `action.json`,
window receipts and SHA256SUMS accompany actual pixels. Empty-profile frames
were visually inspected; sampled OCR is in `../evidence/swarm-2026-09-21/r5b-ocr.txt`.
This is not an exhaustive per-frame OCR certification.

Versions: actual repository plugin at base ba16b777; bridge 1.0.36;
QML shell v4.7.8-git (June closure), Quickshell June f308426; Okular 26.08.0;
niri August feb3e43. Installed Rust Noctalia v5.0.1 is NOT the demonstrated host.
No mocks, account profiles, documents, microphone, music or desktop screenshot.
KDE/Noctalia artwork remains upstream-owned; captured pixels are demonstration
material, not a relicensing of upstream assets.

Helpers stop only owned processes. Runtime roots contain disposable profiles;
remove only the exact printed root after process exit if desired. No global
cache cleanup, service restart, deploy or merge. Remaining work: diagnose the
legacy popup mapping failure under this private stack, then record a real
popup-row action. No runtime plugin fix was attempted in this bounded pass.
