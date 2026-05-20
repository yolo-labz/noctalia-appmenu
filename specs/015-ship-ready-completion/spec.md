# Specification: noctalia-appmenu ship-ready completion

**ID:** 015-ship-ready-completion
**Created:** 2026-05-19
**Author:** @phsb5321
**Constitution version:** 1.0.0
**Status:** draft

## Why

Twenty-one minor releases (v1.0.0 → v1.0.21) shipped against the
noctalia-appmenu plugin. Pedro's most recent field reports
(images #5 + #6, 19/05/2026, after v1.0.21 deploy) show **two**
defects that persist across multiple patch cycles — a visual gap
versus the noctalia Calendar/Clock reference and a routing bug
where clicking *New Tab* on Firefox window A opens the tab on
Firefox window B. Both have been "fixed" before; both came back.

The plugin is "mostly working" — bar strip renders, popup opens,
outside-click dismiss is solid, self-heal recovers from AT-SPI
walk races. But "mostly working" is the failure mode that powered
the v1.0.5..v1.0.12 drift loop documented in `specs/013-sota-overhaul`.
Each unverified-but-shipped patch costs a re-iteration. The
v1.0.20 niri-pre-focus patch shipped without a measurable test
gate — Pedro alone is the regression detector.

This spec is the close-out of the noctalia-appmenu MVP. It
re-states the remaining behaviour gaps as **finite, mechanically
verifiable** requirements, demands the verification harness ship
**before** the next patch, and reinforces the CLAUDE.md drift
doctrine with one new trigger covering exactly the class of
regression that v1.0.20 evaded.

After this spec ships:

- Multi-Firefox / multi-window-same-app menu actions route to the
  captured window with zero misroutes in a 10-trial automated run.
- The popup is visually indistinguishable from the noctalia Calendar
  popup at the same monitor scale (Pedro-judged < 3 sub-pixel diffs).
- A release-checklist script blocks the `plugin-tag` stage of
  `scripts/release.sh` when any user-visible gate fails.
- CLAUDE.md grows drift trigger **I** (user-visible failure mode
  persists across 2 deploys → redesign spec required).
- The next plugin release ships with zero open user-visible defects.

## User scenarios

### Scenario 1: Multi-Firefox-instance routing correctness

**Given** Pedro has three Firefox windows open on the same PID
(canonical multi-instance case — windows 327, 328, 329 on the
desktop host as of 19/05/2026 21:30 BRT)

**When** Pedro focuses Firefox window 327 via niri-IPC or via
clicking its surface, sees the bar render Firefox's menubar, then
clicks `File → New Tab`

**Then** the new tab opens on Firefox window 327 — verifiable
because window 327's titlebar reflects the new tab's URL within
500 ms of the click, while windows 328 and 329 keep their existing
titles unchanged. The same flow repeats with windows 328 and 329
without any cross-window misroute.

### Scenario 2: Visual parity with noctalia Calendar

**Given** Pedro has the noctalia bar visible at the configured
monitor scale (typically `Settings.data.bar.fontScale = 1.2` on
the desktop host) with the Catppuccin-Mocha-derived Color
singleton active

**When** Pedro screenshots the appmenu popup (any Firefox top-level
menu fully realised) and the Calendar popup at the same DPI

**Then** the two screenshots agree on: corner radius (radiusL on
the body, asymmetric radiusL only on the bottom corners for the
appmenu's bar-attach behaviour), border treatment (none — edge
defined by radius + drop shadow only), surface colour (Color.mSurface),
row hover treatment (Color.mHover with ColorAnimation of duration
Style.animationFast), inner padding (Style.marginM), separator
rendering (NDivider, not a bare Rectangle), disabled-row opacity
(0.5), and typography (NText at Style.fontSizeS, Color.mOnSurface
default, Color.mOnHover when row hovered). Pedro counts < 3
sub-pixel differences between the two screenshots.

### Scenario 3: Self-heal absent in steady state

**Given** Pedro has held focus on Firefox window 327 for ≥ 3
seconds (long enough for the bridge's AT-SPI walker to settle on
the focused-window menu)

**When** Pedro clicks any top-level entry (File, Edit, View, …)

**Then** the popup opens immediately with the entry's children
populated. The plugin's `RefreshActive` retry path is NOT
triggered (verifiable by grepping `journalctl --user -u
noctalia-shell` for the `[appmenu] empty top-level — triggering
RefreshActive retry:` log line — zero hits in the 5 seconds after
the click).

### Scenario 4: Release-gate executable verification

**Given** a future plugin release `v1.0.NN` is about to be
tagged via `scripts/release.sh`

**When** the release script reaches the `plugin-tag` stage

**Then** before tagging, an additional `verify-checklist` stage
runs `scripts/verify-release.sh` which executes every release-gate
check (visual parity smoke, routing smoke on a synthetic
multi-window AT-SPI fixture, self-heal absence in steady-state,
deploy idempotence). The tag is created only if every check exits
zero. A failing check aborts the release with the exact gate name
and remediation pointer.

### Scenario 5: Drift trigger I catches multi-deploy regressions

**Given** a future Claude session ships `v1.0.NN+1` and then
`v1.0.NN+2` against the same user-reported symptom (Pedro
re-files the same noun phrase from the original report twice
across two deploys)

**When** that agent attempts to commit `v1.0.NN+3` against the
same symptom

**Then** the pre-commit hook (extension of the existing drift
guard) refuses the commit and points the agent at the redesign
spec template. The commit message would have been the *third*
patch in the same drift mode — exactly the failure pattern the
v1.0.5..v1.0.12 case study covers, escalated through "user
acknowledges symptom persistence" rather than the existing
"agent cites prior version SHA" trigger.

## Functional requirements

### FR-001 — Configurable niri focus-settle window

The bridge's `atspi-click` subcommand SHALL accept a
`--focus-settle-ms <integer>` argument (default 150 ms,
overridable per-call by the plugin and per-install by the
bridge config) that controls the delay between
`niri_ipc::Action::FocusWindow` and the subsequent
`org.a11y.atspi.Action.DoAction(0)`. The default doubles the
v1.0.20 hard-coded 30 ms and aligns with empirical Firefox
internal-active-browser sync time. Verifiable by spawning
`noctalia-appmenu-bridge atspi-click <s> <p> --winid <w>
--focus-settle-ms 200` and observing the 200 ms delta between
the niri-IPC reply and the AT-SPI call (visible in
`strace -e trace=connect,write -tt`).

### FR-002 — Click diagnostics promoted to INFO

The bridge's `atspi-click` subcommand SHALL log every click
attempt to stderr at INFO level with the (service, path,
winid, focus_settle_ms, niri_focus_result, do_action_result)
tuple. The plugin's `clickProcess` SHALL capture the
subprocess stderr and forward it to `console.log` so the
journal carries one structured line per click. Verifiable by
clicking *File → New Tab* and grepping
`journalctl --user -u noctalia-shell | grep '\[appmenu\] atspi-click'`
for a single line containing all six fields.

### FR-003 — Accelerator-key fallback for high-confidence routing — DEFERRED

**Status: DEFERRED.** See [ADR-0028](../../docs/adr/ADR-0028-fr-003-accelerator-deferred.md).

niri-ipc 26.4.0 (the bridge's pinned IPC surface) exposes
**no** keyboard-input-synthesis variant on its `Action` enum
— verified against the 141-variant enum in
`/nix/store/mk4r9mi3k0qwwzlk135hz65in175ix98-cargo-package-niri-ipc-26.4.0/src/lib.rs`.
Alternatives (`wtype` requires the Wayland virtual-keyboard
protocol which niri does not implement; `ydotool` requires
root + `/dev/uinput`) both violate the "single sidecar talks
niri-IPC + D-Bus, nothing else" architecture invariant from
ADR-0001. Deferring closes the drift-trigger-I exit
(would-be 4th patch on the wrong-window-routing symptom
axis after v1.0.20..v1.0.22). The 150 ms focus-settle floor
shipped in v1.0.22 is the mitigation of record; self-heal
(FR-005) + cascade self-heal (FR-006) cover residual races.

Reopening criteria + un-defer flow live in ADR-0028
§"Reopening criteria". Tasks T1.1, T1.2, T1.3, T2.5 carry
the same DEFERRED status in `tasks.md`.

Original (now-shelved) proposal: *For menu leaves whose
`keybinding` field (parsed from AT-SPI's
`Action.GetKeyBinding(0)` and serialised into `active.json`
per a v1.0.22 schema bump) names a standard-X11-style
accelerator (Ctrl+T, Ctrl+W, Ctrl+N, Ctrl+Shift+P, …), the
bridge would prefer dispatching the accelerator via niri-IPC
keyboard-input rather than AT-SPI DoAction. Accelerator-
bearing leaves route to the compositor's focused window by
Wayland-spec design — eliminating the Firefox-internal-
active-browser race for the common case.*

### FR-004 — Visual-spec pixel audit

A `specs/015-ship-ready-completion/visual-audit.md` artefact
SHALL list every visual token used by the popup (radius,
margin, border-width, color reference, animation duration,
typography size + weight) and the corresponding shell-widget
source-of-truth (file + line). Each row carries a pass/fail
column. The plugin SHALL match every row before the spec
ships. Verifiable by grepping the audit table for any FAIL
row.

### FR-005 — Self-heal hard ceiling + telemetry

The plugin's `RefreshActive` retry path SHALL fire AT MOST
ONCE per popup-open session (single-shot already implemented
in v1.0.21 — formalise as an FR and add a unit-testable
counter on `_pendingRetryButton` clearing). The plugin SHALL
log one summary line per popup-close at INFO level with the
(label, retried_count) tuple. Verifiable by clicking the
same top-level entry twice in succession after focus has
been held for ≥ 3 seconds (steady state): the journal SHALL
show `retried_count=0` for both clicks.

### FR-006 — Self-heal coverage for submenu cascades

When a submenu popup (`SubmenuPopup.qml`) opens against a
parent menu item whose `children` array is empty, the same
`RefreshActive` + retry-open flow SHALL fire that
`BarWidget.qml`'s top-level click handler runs in v1.0.21.
Heuristic: any submenu node whose label is non-empty AND
whose `type === "submenu"` triggers the retry.
Verifiable by hovering over a parent menu item with empty
children and confirming the cascade self-heals within the
same 250 ms window as the top-level path.

### FR-007 — Release-checklist gate in `scripts/release.sh`

`scripts/release.sh` SHALL grow a new stage `verify-checklist`
(between `plugin-tag` and `plugin-release`) that runs
`scripts/verify-release.sh`. The verify script SHALL execute
the four release-gate categories (visual smoke, routing
smoke, self-heal absence in steady-state, deploy idempotence)
and return non-zero on any failure. A non-zero exit SHALL
abort the release with the gate name on stderr. Verifiable
by intentionally regressing the popup radius to `radiusM`
and running the script — the visual smoke gate SHALL fail.

### FR-008 — Drift trigger I in CLAUDE.md

`CLAUDE.md` SHALL grow trigger **I** in the drift-detection
table: *"User-reported failure mode persists across ≥ 2
deploys against the same symptom"*. Mechanical detection:
`gh issue list --state open --search '<symptom phrase>'` AND
`gh search prs --state merged --search '<symptom phrase> in:body'`
both return ≥ 2 results. Required action: open a redesign
spec under `specs/NNN-<bug>/spec.md` and halt further patches
on the same axis. Verifiable by running the drift-trigger
self-test (a CI workflow added under spec 015's plan phase).

### FR-009 — Plugin Cargo.toml bump SHALL be the last semantic commit

Per CLAUDE.md trigger E + the existing
`scripts/verify-tag-subject.sh` pre-push hook, any `v1.0.NN`
tag SHALL be cut against a commit whose subject contains
`v1.0.NN`. This FR re-states the existing guard for
completeness — every spec 015 patch SHALL respect it. No
new code; verifiable by retrying the v1.0.14 incident
synthetically and confirming the hook refuses the push.

### FR-010 — Visual treatment SHALL ONLY reference shell singletons

Plugin QML SHALL NOT contain any raw hex literal, `font.pixelSize`
literal, `radius: <integer>`, `border.width: <integer>`,
`anchors.margins: <integer>`, or `duration: <integer>` for any
visual property. Every visual value SHALL bind to
`Color.m*` or `Style.<token>`. Verifiable by a `grep -nE
'#[0-9a-fA-F]{6}|pixelSize: [0-9]+|radius: [0-9]+|border\.width: [0-9]+'`
pass over `plugin/*.qml` returning zero hits inside visual
blocks (header comments excluded).

## Non-functional requirements

### NFR-001 Performance

First-focus-to-popup-visible latency SHALL remain ≤ 50 ms on
the desktop host (7950X3D, niri 26.4) for applications whose
menu is already in the bridge cache. Cold-start (focus event
to first popup) ≤ 250 ms for applications under 50 top-level
menu entries. Verifiable by a `time.now()`-bracketed QML
log line on popup `openAt` versus the `[appmenu] click on
top-level:` line.

### NFR-002 Reliability

Routing smoke (FR-003) SHALL pass in 10 / 10 trials on the
multi-Firefox-instance scenario. Visual-audit table
(FR-004) SHALL have zero FAIL rows. Self-heal counter
(FR-005) SHALL show 0 retries across 50 popup-opens in
steady state.

### NFR-003 Reproducibility

`scripts/verify-release.sh` SHALL be runnable from a clean
`nix develop` shell without additional install steps. Its
required tools (busctl, gdbus, niri, jq) are already in the
devshell.

### NFR-004 Observability

Every release-gate check SHALL emit exactly one structured
line per outcome (PASS / FAIL / SKIP) to stderr with a
machine-parseable prefix (`[gate name=routing-smoke] PASS`).
A summary JSON SHALL land at
`/tmp/noctalia-appmenu-release-gate-<TAG>.json` for the
release script to attach to the GH release as a verification
artefact.

## Out of scope

- Multi-compositor support beyond niri (KWin, Hyprland, Sway).
  Stays niri-first until SC-001..SC-005 are met on niri.
- Resurrection of the `com.canonical.dbusmenu` substrate.
  ADR-0024 remains authoritative — AT-SPI is the v1 substrate.
- Themes beyond the user's active Color/Style singletons. No
  hardcoded palettes; no Catppuccin-Mocha-specific code paths.
- Replacing `Quickshell.PanelWindow` for the popup carrier.
  Option G (v1.0.16) settled this; future popup-surface
  changes require a separate spec.
- Cross-PID multi-window apps (Chromium with multiple profile
  processes). Out of scope until single-PID multi-window
  (Firefox) is fully closed.

## Constraints / dependencies

- noctalia-shell continues to expose its `Style` + `Color`
  singletons (`qs.Commons`). Verified against the
  2026-05-17 shell snapshot mounted at
  `/nix/store/k7ylwjx92y0lbch5gydbx17mjiy6vblz-noctalia-share-patched`.
- `Quickshell.PanelWindow` remains the canonical wlr-layer-shell
  carrier.
- niri-IPC continues to publish `WindowFocusChanged` events
  and accept `Action::FocusWindow { id }` (niri 26.4 verified).
- The `niri-ipc` crate (Cargo dep) tracks niri's stable IPC
  surface. Crate version 26.4.0 used by v1.0.20.
- `Settings.data.bar.fontScale`, `Settings.data.general.enableShadows`,
  `Settings.data.general.radiusRatio` are read by the popup but
  not written — the spec adds no new settings.

## Assumptions

- The bridge's `do_action` path will continue to use AT-SPI
  for items without accelerators (FR-003 is additive).
- The release skill (`scripts/release.sh`) is the canonical
  release path — bypassing it for "hotfix" workflows is not
  supported and the new verify-checklist stage assumes this.
- A "user-reported failure mode" is identifiable by the noun
  phrase in Pedro's `gh issue create` body or chat message
  (FR-008). Approximate string match is acceptable.
- Pedro's monitor scale + theme remain stable for the visual
  parity comparison (Catppuccin-Mocha-derived Horizon-Terminal-Dark
  on desktop). A theme change after this spec ships would
  re-baseline the visual audit but not invalidate the spec.

## Success criteria

- **SC-001** — Routing-smoke harness runs 10 alternating
  multi-Firefox-instance trials with three distinct windows.
  Every trial opens *New Tab* on the captured window
  (verified by titlebar change). Zero misroutes. The harness
  output JSON contains `"trials_passed": 10, "trials_failed": 0`.
- **SC-002** — Pedro side-by-sides the appmenu popup and the
  noctalia Calendar popup at the same monitor scale and
  reports `< 3` sub-pixel visual differences. The visual-audit
  table (FR-004) has zero FAIL rows.
- **SC-003** — In 50 steady-state popup-opens (focus held
  ≥ 3 s before any click), the plugin's `[appmenu]
  RefreshActive retry succeeded` and `[appmenu] RefreshActive
  retry STILL empty` log lines have a combined count of `0`.
- **SC-004** — `scripts/release.sh 1.0.NN` aborts before tag
  push when `scripts/verify-release.sh` is intentionally
  regressed (radius regressed to `radiusM`). The release
  script exit code is non-zero and stderr names the failing
  gate.
- **SC-005** — `CLAUDE.md` contains trigger I in the
  drift-detection table AND the pre-commit drift hook
  (extension of `scripts/verify-tag-subject.sh`) refuses a
  third patch against the same symptom phrase across two
  prior merged PRs. Demonstrated by a synthetic dry-run
  recorded in `specs/015-ship-ready-completion/case-study.md`.

When all five SCs pass, this spec is "shipped" and the slug
moves from `specs/015-ship-ready-completion/` to a Git tag
`v1.0.22` (or higher, depending on how many patches land).

## References

- `specs/013-sota-overhaul/spec.md` — predecessor drift-doctrine
  spec; this spec extends it with trigger I.
- `specs/013-sota-overhaul/visual-spec.md` — visual idiom
  canonical reference; FR-004 audits against it.
- `specs/013-sota-overhaul/agent-governance.md` — drift case
  study v1.0.5..v1.0.12.
- `CLAUDE.md` — Releases section + Drift detection section.
- `scripts/release.sh`, `scripts/verify-tag-subject.sh` —
  release skill and tag-subject pre-push guard.
- Memory entries:
  `feedback_nh_switch_no_shell_restart.md`,
  `feedback_qml_qmllint_not_load_test.md`,
  `feedback_codex_review_before_iter_3.md`.
- Issue #109 — multi-Firefox routing root cause + workaround
  options. This spec's FR-001 + FR-003 are the formalised
  fixes.
- PR history that informs this close-out: #82..#113.
