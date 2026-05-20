# ADR-0028 — FR-003 accelerator dispatch deferred (niri-ipc 26.4.0 gap)

- **Status:** accepted
- **Date:** 2026-05-20
- **Deciders:** Pedro H S Balbino
- **Supersedes:** none
- **Amends:** spec 015 §FR-003 (status: open → deferred)
- **Tracking PR / branch:** `121-fr-003-deferred`

## Context

Spec 015 FR-003 proposed an "accelerator-key fallback" routing path:
when an AT-SPI menu leaf advertises a keybinding (e.g. *File →
New Tab* → `Ctrl+T`), the bridge would synthesise the accelerator
via niri-IPC keyboard injection and let the compositor deliver it
to the focused window — instead of round-tripping the click through
AT-SPI `DoAction`.

The Wayland-spec property the FR was leaning on: keyboard events
route to the compositor's focused surface, by design. An
accelerator delivered via the compositor's input pipe therefore
bypasses the Firefox-internal "active browser instance" race the
AT-SPI clicking path occasionally loses (the v1.0.20..v1.0.22
wrong-window-routing symptom).

The path requires a niri-IPC primitive for keyboard-input
synthesis. The FR assumed `niri_ipc::Action::SendKeyboardInput`
or equivalent.

## Decision

**Defer FR-003. Mark status: deferred. Do not implement.**

The accelerator path is shelved until niri (the upstream
compositor) ships a keyboard-input synthesis primitive in
`niri-ipc`. The bridge continues to use the AT-SPI `DoAction`
clicking path (FR-001 + FR-002), with the 150 ms focus-settle
floor introduced in v1.0.22 as the routing-race mitigation.

## Why — niri-ipc 26.4.0 surface audit

Pin in `bridge/Cargo.toml`:

```toml
niri-ipc = "26.4.0"
```

The crate's `Action` enum (141 variants in 26.4.0) exposes:

- **Window management:** `FocusWindow`, `CloseWindow`,
  `FocusMonitor`, `FocusWorkspace`, `MoveWindowToWorkspace`, …
- **Keyboard layout / inhibit:** `KeyboardLayouts` (query),
  `SwitchLayout`, `ToggleKeyboardShortcutsInhibit`,
  `ShowHotkeyOverlay`.
- **Pointer-style:** nothing — niri does not expose pointer
  motion / button synthesis via IPC either.

The crate exposes **no** keyboard-input-synthesis variant:
no `SendKeyboardInput`, `InjectKey`, `KeyPress`, `KeyboardInput`,
`SimulateKey`, or `EmulateKey`. Verified against
`/nix/store/mk4r9mi3k0qwwzlk135hz65in175ix98-cargo-package-niri-ipc-26.4.0/src/lib.rs`.

## Alternatives considered

### A. Subprocess `wtype` / `ydotool`

`wtype` (Wayland-native, virtual-keyboard protocol) or `ydotool`
(uinput-daemon) could synthesise the accelerator without niri's
IPC surface.

**Rejected** for the bridge to take on:

1. `wtype` requires the Wayland virtual-keyboard protocol —
   niri does **not** implement `zwp_virtual_keyboard_v1` (open
   upstream issue niri/niri#1132). Pipewire-friendly compositors
   ship it; niri does not.
2. `ydotool` requires a uinput daemon running as root with
   `/dev/uinput` permissions — large attack-surface escalation
   for a per-user bridge.
3. Either dependency means the bridge gains a runtime that is
   not part of the Wayland-IPC contract — that breaks the
   "single sidecar talks niri-IPC + D-Bus, nothing else"
   architecture invariant from ADR-0001.

### B. Upstream `SendKeyboardInput` to niri

The right long-term answer. niri's IPC model is opt-in per-IPC-call
and the maintainer (YaLTeR) has accepted similar additions in the
past. A separate spec issue tracks this proposal — out of scope
for spec 015's ship-ready cycle.

### C. Use AT-SPI's `Action.DoAction(KeyBinding)` directly

`Action.GetKeyBinding(0)` returns the advertised accelerator
string but **AT-SPI has no `DoAction(KeyBinding)` surface**;
`DoAction(index)` only re-fires the leaf's default action —
which is exactly the click we already do. AT-SPI does not
expose "synthesise this accelerator on the focused app" as a
primitive; that is the compositor's job, which loops us back
to the niri-IPC gap.

## Consequences

### Positive

- No code change. The drift-trigger-I scenario (4th patch on
  the same routing symptom) is averted by **not patching** —
  the FR is closed-as-deferred with a clear upstream blocker,
  not iterated on.
- The 150 ms focus-settle floor (v1.0.22) is the mitigation
  of record for the FR-003 symptom class. Self-heal (FR-005)
  + cascade self-heal (FR-006) handle the residual races.
- The spec-015 release gates (FR-007) can ship without FR-003
  — they verify routing via the AT-SPI clicking path, which
  IS the production path.

### Negative

- The "single-trial-deterministic" property FR-003 was reaching
  for stays unachieved on accelerator-bearing leaves. The
  routing smoke continues to depend on focus-settle + self-heal
  rather than a Wayland-spec guarantee.
- Spec 015's release-ready definition no longer includes
  FR-003. Operators reading the spec MUST follow the cross-
  reference to this ADR to understand why.

### Neutral

- The `keybinding: Option<String>` field on `bridge/src/atspi.rs::MenuItem`
  is **not** added. `active.json` schema is unchanged. When
  FR-003 is un-deferred (after niri-IPC gains the primitive),
  the schema bump and plugin `fireClick` integration land
  together — single coherent change, not a half-shipped surface.

## Reopening criteria

Mechanically un-defer this ADR when ALL of:

1. `niri-ipc` releases a version with an `Action` variant for
   keyboard-input synthesis. Grep:
   `cargo search niri-ipc` ≥ next minor.
2. Pedro confirms upgrading the bridge's pin is safe (semver
   review).
3. A successor ADR cites this one as `Supersedes:` and walks
   through the new niri-IPC surface used.

## Cross-references

- Spec: `specs/015-ship-ready-completion/spec.md` §FR-003
- Tasks: `specs/015-ship-ready-completion/tasks.md` T1.1, T1.2,
  T1.3, T2.5 (all → deferred)
- Related: ADR-0001 (architecture: single Rust sidecar, niri-IPC
  + D-Bus only)
- Upstream tracker: niri repo (no specific issue at time of
  writing — file one before re-opening this ADR)
- Drift triggers: this defer-instead-of-iterate decision is the
  CLAUDE.md trigger-I exit (would-be 4th patch on the same
  symptom, blocked at the architecture-redesign layer).
