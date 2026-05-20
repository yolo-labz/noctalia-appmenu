# Self-heal-absence checklist — spec 015 SC-003 gate

**Owner:** `scripts/verify-release.sh` gate `self-heal-absence`
**Hypothesis:** the AT-SPI walker is reliable in steady state.
Self-heal (`gdbus RefreshActive` retry) is the safety net for
*walker race* conditions — its firing in steady state means the
walker has regressed.

Each row is a finite pass/fail check.

## Prerequisites

- [ ] **SH-001** Bridge is running (`systemctl --user is-active
      noctalia-appmenu-bridge.service` returns `active`).
- [ ] **SH-002** A long-lived multi-window app (Firefox) is
      focused and has been so for ≥ 5 seconds (longer than the
      spec's 3 s steady-state minimum). Verifiable by reading
      niri focus timestamp or by sleeping the harness 5 s after
      `focus-window`.

## Top-level click trials

Click each of the 8 standard Firefox top-level entries (File,
Edit, View, History, Bookmarks, Profiles, Tools, Help) in turn,
each preceded by 3 seconds of unmoved focus:

- [ ] **SH-010** All 8 clicks open the popup with
      `children.length > 0` on the first attempt. No
      `[appmenu] empty top-level — triggering RefreshActive
      retry:` log line appears.
- [ ] **SH-011** No `[appmenu] RefreshActive retry succeeded:`
      log line appears either (since SH-010 means no retry
      fires).
- [ ] **SH-012** Self-heal retry counter (FR-005 telemetry)
      reports `retried_count == 0` for each click.

## Submenu cascade trials

For each top-level entry whose children include a submenu
(e.g. `New Container Tab → <container list>`):

- [ ] **SH-020** Hovering or clicking the parent menu item
      opens the submenu cascade with children populated. No
      submenu self-heal triggers (per FR-006).
- [ ] **SH-021** No `[appmenu] cascade self-heal:` log line.

## 50-iteration steady-state regression smoke

- [ ] **SH-030** Repeat the SH-010 protocol 50 times in a row
      (across multiple top-level entries; not all 50 on
      `File`). Total `[appmenu] RefreshActive retry succeeded`
      and `[appmenu] RefreshActive retry STILL empty` line
      counts (combined) == `0`.

## Negative-case (self-heal works when needed)

- [ ] **SH-040** Restart the bridge service. Within 200 ms
      (before the cold AT-SPI walk completes), click File.
      Self-heal SHALL fire (`retry succeeded` log line),
      proving the safety net still works.
- [ ] **SH-041** Following SH-040, click File again. Steady
      state has now been reached. No self-heal fires.

## Result roll-up

Gate emits:
`[gate name=self-heal-absence result=PASS|FAIL steady_retries=N cascade_retries=M]`
PASS condition: `steady_retries == 0 AND cascade_retries == 0 AND
SH-040 confirmed the safety net is wired`.
