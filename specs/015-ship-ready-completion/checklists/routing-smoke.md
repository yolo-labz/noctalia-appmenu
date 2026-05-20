# Routing-smoke checklist — spec 015 SC-001 gate

**Owner:** `scripts/verify-release.sh` gate `routing-smoke`
**Scenario class:** Multi-window-same-PID app (Firefox is the
canonical case; any single-PID-multi-window app counts)

Each row is a finite pass/fail check. The release gate stops
on any FAIL.

## Prerequisites

- [ ] **RS-001** At least 3 windows of the same single-PID app
      are open (Firefox preferred; if not running, the gate
      attempts to spawn 3 windows via
      `firefox --no-remote -P default --new-window` and
      retries).
- [ ] **RS-002** niri reports each window with a distinct `id`
      and identical `pid`. Verified via `niri msg --json windows`.
- [ ] **RS-003** The bridge's AT-SPI walker has populated the
      menu cache for the focused PID (confirmed by
      `cat ~/.cache/noctalia-appmenu/active.json | jq '.menu.children | length > 0'`).

## Trial protocol

For each window `W` in the multi-window set, in alternation:

1. Issue `niri msg action focus-window --id W`.
2. Sleep 250 ms (longer than spec 015 FR-001's 150 ms settle).
3. Read `~/.cache/noctalia-appmenu/active.json` and confirm
   `focus_winid == W`.
4. Extract the `New Tab` leaf's `(service, path)` from the
   menu tree.
5. Invoke `noctalia-appmenu-bridge atspi-click <service>
   <path> --winid W --focus-settle-ms 150`.
6. Sleep 500 ms.
7. Re-query `niri msg --json windows`; locate window `W` by
   `id`; read its `title`.
8. Assert that `W`'s tab count incremented — verifiable by
   the title changing or by Firefox's about:about page count
   (whichever the gate harness uses).

10 trials. Gate fails on any trial whose `W` did NOT receive
the new tab.

## Counters

- [ ] **RS-010** `trials_passed == 10`.
- [ ] **RS-011** `trials_failed == 0`.
- [ ] **RS-012** Cross-window misroute rate == `0`.
- [ ] **RS-013** Mean click-to-tab-visible latency reported
      (informational, not pass/fail).

## Accelerator-bearing leaves (FR-003)

For leaves whose `keybinding` field names a standard accelerator:

- [ ] **RS-020** Bridge dispatches via niri-IPC keyboard input
      rather than AT-SPI DoAction. Verifiable in the bridge's
      INFO log: `[appmenu] atspi-click ... mode=accelerator
      key=Ctrl+T`.
- [ ] **RS-021** Routing smoke trials with accelerator-bearing
      leaves SHALL be 10 / 10 across the multi-window set.
      Wayland-spec routing-to-focused-window guarantee covers
      this case.

## Edge cases

- [ ] **RS-030** When the captured `winid == 0` (older plugin
      or synthetic item), the bridge SHALL skip the pre-focus
      step and fall back to DoAction-without-pre-focus
      behaviour. Gate passes on this configuration as a
      sanity check.
- [ ] **RS-031** When niri rejects `FocusWindow(id=W)` (window
      closed between snapshot and click), the bridge logs a
      WARN line and proceeds with DoAction. Gate verifies
      the WARN line appears in journal.
- [ ] **RS-032** When the focused window changes between
      popup-open and click (Pedro hovers over a sibling
      window), the captured `_capturedWinid` (not the live
      `focusWinid`) drives routing. Gate spawns this race
      synthetically: focus A, capture, focus B, click.
      Expected: tab opens on A.

## Result roll-up

Gate outputs a single line:
`[gate name=routing-smoke result=PASS|FAIL trials_passed=N trials_failed=M]`
to stderr and a JSON summary to
`/tmp/noctalia-appmenu-release-gate-vX.Y.Z.json` for the
release-attestation pipeline.
