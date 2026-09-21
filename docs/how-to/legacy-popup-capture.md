# Real legacy topbar popup capture

Measured 21/09/2026. **Legacy QML Noctalia 4.7.8-git only**, not the current
Rust host. Empty-profile Okular and Dolphin 26.08.0 are synthetic workloads;
no personal documents, accounts or daily desktop were used.

[Video](../assets/popup-capture/demo/demo.mp4) ·
[GIF](../assets/popup-capture/demo/demo.gif) ·
[Okular dropdown](../assets/popup-capture/demo/okular-popup.png) ·
[Dolphin dropdown](../assets/popup-capture/demo/dolphin-popup.png)

The original plugin at `29d099559773cd78f79133bcbca8c5ec96b2eef3` renders both
dropdowns and dispatches both About actions. No plugin/bridge runtime fix was
made. The earlier R5b invisibility was **not reproduced**; its historical cause
is still unknown. This is not evidence that the original report was fabricated
or that every host/configuration works.

## Reproduce on the measured host

Installed store paths are pinned in the source; missing closures fail rather
than silently selecting a different host. Use a fresh session for every run:

```bash
python3 -u docs/evidence/popup-capture-2026-09-21/probe-isolated.py --capture
# In another terminal, after ~20 seconds, use ONLY the freshly printed root:
python3 docs/evidence/popup-capture-2026-09-21/capture-private.py \
  /tmp/r5c-PRINTED docs/assets/popup-capture/local-replay --prepare
# Stop only this owned session; the launcher otherwise expires after 480 s:
touch /tmp/r5c-PRINTED/stop
```

The launcher uses Xvfb `-displayfd`, private HOME/config/cache/runtime, private
session and accessibility buses, `ATSPI_DBUS_IMPLEMENTATION=dbus-daemon`,
blocked system-bus/audio addresses and software rendering. Runtime roots are
fresh, mode 0700, short `/tmp/r5c-*` paths: the longer Pi scratch path exceeded
Unix `SUN_LEN` and prevented niri IPC creation. No global fallback is used.
The controller verifies root ownership, the private environment, Xvfb command
and PID start ticks. Expired roots are refused, even if a display number is reused.
The launcher terminates only its own process group and Xvfb.

Preparation resizes/focuses the **private X11 niri window**, hides Dolphin's
mount sidebar and enables its native menubar using F9/Ctrl+M **before recording**.
These are setup, not the demonstrated actions. No keyboard shortcut is sent
during either captured interaction. niri focuses each application before its
sequence and closes the resulting dialog afterwards as teardown.

The action oracle:

1. Asserts `source=atspi`, matching application PID and populated native Help.
2. Locates Help in actual screenshot OCR at y<32 (the topbar, not app menubar).
3. Clicks Help; requires a visible About row in the dropdown screenshot.
4. Clicks that row; requires a new About window with the same app PID.
5. Saves before/after windows, native menus, mouse coordinates/times, screenshots,
   real video and QML log. The helper never invokes `atspi-click`; the actual
   plugin's normal `MenuRow -> itemActivated -> fireClick` path owns dispatch.

OCR-only crops are enlarged 3× for reliable small-font recognition. Original
screenshots and the MP4 are unmodified. TSV uses quote-disabled parsing because
OCR text can contain unmatched quotes. A first small-font OCR failure and its
parsing follow-up are retained as diagnostic receipts, not runtime failures.

## Red-capable check

Start **another fresh session**, then add `--miss-topbar` to the controller
command. It deliberately clicks empty topbar space and must exit nonzero with
`visible topbar popup row missing`. This was observed in `negative-check.txt`;
`negative/okular-popup.png` has no dropdown. Neither lack of a crash nor a
populated bridge snapshot alone passes this gate.

Two independent fresh sessions passed both application sequences:
`capture-check.txt` and `replay-check.txt`. Reusing an already exercised session
for the negative check first failed the native-menu precondition; its cause was
not diagnosed (`post-dialog-precondition.txt`). The supported recipe is a fresh
session, not a claim of arbitrary repeat-session/focus self-healing.

## Assets, provenance and limitations

- Main MP4: 1280×720, H.264, 10 fps, 25.7 seconds, 186414 bytes, silent;
  wall-clock speed, no edits. GIF: 768×432, 5 fps, generated from that MP4.
- Actual Quickshell June f308426; QML Noctalia June 5311e14 (4.7.8-git);
  bridge 1.0.36; niri August feb3e43; both KDE apps 26.08.0.
- CPU x264, eight encoding threads; FFmpeg-full 9.0.1 provides x11grab.
  No audio, music, camera, private profile or external upload.
- Screenshot/action receipts accompany media under `docs/assets/popup-capture/`.
  Second-run and negative evidence live under
  `docs/evidence/popup-capture-2026-09-21/`; hashes cover assets and scripts.
- KDE/Noctalia names, icons, artwork and displayed credits remain upstream-owned;
  captured demonstration pixels do not relicense those assets.
- This host-local check does not establish Rust Noctalia compatibility,
  lazy-menu/cascade behavior, multi-output geometry or universal toolkit support.
- No deploy, release, global service change or sibling-worktree edit. Independent
  exact-head review, inherited CI disposition and merge belong to the coordinator.
