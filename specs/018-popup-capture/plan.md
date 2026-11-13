# Plan

1. Reuse the R5b private Xvfb → niri → Quickshell launcher with fresh empty
   profiles, private session/a11y buses and dbus-daemon Registry activation.
2. Keep short owned runtime paths (Unix socket path limit); pin measured store
   binaries and check PID identity before any capture/control.
3. Drive topbar Help and About rows by actual screenshot OCR coordinates.
   Assert native menu source/PID, visible popup row and a new same-PID About
   window. Record mouse events and unedited video; no direct action dispatch.
4. Replay in another fresh session and retain a deliberate wrong-coordinate
   failure to demonstrate the visibility gate is red-capable.
5. Preserve report, hashes and limitations. Coordinator supplies independent
   review and owns merge. No agents spawned by this worker.

## Shared callers traced

BarWidget.qml ordinary click and lazy-expansion completion call openAt;
AppmenuPopupWindow emits itemActivated to BarWidget.fireClick. MenuRow is shared
by top-level and SubmenuPopup; nested items use SubmenuPopup.open and its loader.
No caller or popup code changed: the original rendering/dispatch path works.

## Constitution check

I PASS: niri only. II PASS: existing bridge retained. III PASS: assigned
worktree only. IV PASS: Conventional Commit/DCO when committed. V PASS: this
spec/plan/tasks precedes capture implementation. VI N/A: no release/workflow
changes. VII PASS: no runtime/fallback changes. No dependencies added; installed
FFmpeg and Tesseract are host-local evidence tools, not package dependencies.

## Bounds

Legacy QML 4.7.8-git only; fixed 1280×720 private output, English empty-profile
apps. This is a measured host-local integration check, not a portable CI claim.
CPU x264 is appropriate for software Xvfb pixels; eight encoding threads avoid
competing whole-machine builds during the coordinated swarm.
