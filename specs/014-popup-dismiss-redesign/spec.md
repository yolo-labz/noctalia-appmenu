---
spec: 014-popup-dismiss-redesign
title: AppMenu popup outside-click dismiss — architecture redesign
status: blocked-on-decision
created: 2026-05-18
owner: pedro
case_study: v1.0.5..v1.0.15 popup-dismiss loop (15-18/05/2026)
references: specs/013-sota-overhaul/spec.md, specs/013-sota-overhaul/agent-governance.md, CLAUDE.md "Drift detection"
---

# 014 — AppMenu popup outside-click dismiss

## 0. STOP — context

This spec exists because **eleven consecutive plugin releases (v1.0.5
through v1.0.15) failed to solve the same one-line bug** ("popup does
not dismiss on outside click"). The drift triggers added to CLAUDE.md
in PR #94 fired **A, B, C, D, E** all simultaneously; the decision
tree dictates *halt point fixes, open a redesign spec*. **THIS IS THAT
SPEC.** No more `vX.Y.Z+1` until Pedro picks an architecture.

## 1. Problem statement

The noctalia-appmenu plugin renders a focused app's menubar as a
dropdown anchored to a top-bar button. Clicking outside the dropdown
(on the Firefox content area, on another app, on the bar's empty
strip) must dismiss the dropdown. As of v1.0.15 deployed on Pedro's
desktop (niri v26.04, Qt 6.10.2, Quickshell `2026-04-28`), this still
does not work.

## 2. The eleven attempts (what was tried, what broke)

| Tag | Approach | Failure mode |
|---|---|---|
| v1.0.5 | recursive Component drop (made plugin actually load) | unrelated; bug not addressed |
| v1.0.6 | skip-list + 30s cache | scope shift, masks dismiss bug |
| v1.0.7 | restore Firefox/Chromium to walker | scope shift |
| v1.0.8 | parallel children walk + 2.5s budget | scope shift |
| v1.0.9 | First Shield (PanelWindow Top, `visible: _shouldShow`) | wl_surface unmap/map race lost first click |
| v1.0.10 | Shield with anchor toggle | anchor count change reconfigures surface |
| v1.0.11 | Shield with `mask: Region { width: 0/full }` | `Qt::WindowTransparentForInput` trap (Quickshell `proxywindow.cpp:672`); binding-driven mask updates don't re-set `wl_surface.set_input_region` |
| v1.0.12 | Drop Shield, `Quickshell.PopupWindow` (xdg_popup) | `grabFocus` defaults to FALSE → Qt::ToolTip → no `xdg_popup.grab` |
| v1.0.13 | `grabFocus: true` explicit | Qt 6.10.2 issues `xdg_popup.grab` BEFORE attaching layer-shell parent via `zwlr_layer_surface_v1.get_popup` — niri/Smithay see parent=`nil`, silently drop grab (niri#1810 / Smithay#1761 client-side regression) |
| v1.0.14 | Re-add Shield + keep grabFocus belt-and-braces (Shield on Top, popup on Overlay) | noctalia-shell's `MainScreen.qml` is itself full-screen `WlrLayer.Top` with input mask covering "everywhere except bar" — created before plugin loaded → above our shield in z-order → consumed every outside click |
| v1.0.15 | Shield → `WlrLayer.Overlay` (above MainScreen) | **Unverified by Pedro at spec-write time.** If xdg_popups are NOT promoted above Overlay, popup will be BELOW shield → shield captures popup clicks too → menu items unclickable. If popups ARE above Overlay, the dismiss works |

## 3. Constraints the architecture must respect

1. **No full-screen wl_surface that paints visible content** on AMD/niri.
   v1.0.3 / noctalia-shell#2216 demonstrated whole-output damage every
   frame freezes the compositor.
2. **The bar (parent surface) is `wlr-layer-shell` not `xdg-shell`.**
   This is not negotiable — the bar is owned by noctalia-shell.
3. **Qt 6.10.2 has a client-side bug** (niri#1810 / Smithay#1761):
   `setFlag(Qt::Popup)` does NOT issue `xdg_popup.grab` properly when
   the transient parent is a layer-shell surface. `grabFocus: true` on
   `Quickshell.PopupWindow` is therefore a no-op for our case. A
   compositor-side `popup_done` event will never reach us via the
   xdg-shell grab path until Qt is fixed.
4. **niri layer ordering** within a single layer is implementation-
   defined; we must not assume creation-order or namespace-alphabetical
   stacking.
5. **noctalia-shell already occupies `WlrLayer.Top` with an
   input-grabbing full-screen `MainScreen.qml`**. Any shield on Top
   competes with it; any shield on Overlay is above all noctalia
   panels but also above xdg_popups (if niri renders popups within
   their parent's stratum, which it does on current source reading).
6. **Plugin code cannot modify noctalia-shell internals** (no
   `PopupMenuWindow.open()` API exposed to plugins).
7. **Pedro is on Catppuccin Mocha + niri-on-AMD**. Cross-compositor
   work is out of scope until niri works.

## 4. Why this is hard

The problem is the intersection of FIVE independent constraints, each
of which has working solutions in other contexts:

- xdg_popup grab works **when** parent is xdg-shell toplevel (Firefox,
  GIMP, etc) — but our parent is layer-shell.
- Full-screen layer-shell shield works **when** no higher input-priority
  surface sits above it — but noctalia-shell's MainScreen does.
- Overlay-layer shield avoids MainScreen — but may be above xdg_popups
  too, making the popup unusable.
- Quickshell ships `HyprlandFocusGrab` for exactly this problem on
  Hyprland — but no niri equivalent exists yet.
- Per-app menu mirroring already works (the popup CONTENTS render
  correctly with full AT-SPI data) — only the input-routing layer
  fails.

## 5. Architecture options

### Option A — Wait for Qt fix, ship "broken-but-honest" v1.0.16

Drop the shield entirely. Drop the v1.0.4 keep-mapped pattern.
Render popup as a regular `Quickshell.PopupWindow` with `grabFocus: true`.
Document that dismiss does not work on niri+Qt 6.10.2 until QTBUG-???
lands. Provide an explicit "close" button on the popup chrome + Escape
key handling.

- **Pros:** Smallest code change. Honest about the constraint. Solo
  Pedro can pick a different bar that uses xdg-shell instead of
  layer-shell to bypass entirely.
- **Cons:** Pedro has explicitly said outside-click dismiss is
  non-negotiable. Escape + close button do not match macOS-style
  menubar UX.
- **Timeline:** v1.0.16 plugin patch, < 1 day.

### Option B — Patch Qt upstream + run a fork until upstream lands

File the QTBUG, write the patch (re-order get_popup + grab in
`qwaylandwindow.cpp`), submit to qt5/qt6/qtwayland upstream, run a
nixpkgs overlay with the patched Qt until merged.

- **Pros:** Fixes the root cause. Benefits every layer-shell+Qt
  user on niri.
- **Cons:** Maintaining a Qt patch overlay is non-trivial (Qt
  rebuilds ~2h on 7950X3D; nixpkgs binary cache misses). Upstream
  review cycle is months. Pedro is solo.
- **Timeline:** patch ready in days, upstream merge in months, fork
  maintenance forever.

### Option C — Move appmenu to noctalia-shell core, not a plugin

If the plugin became part of noctalia-shell upstream it could
piggy-back on `PopupMenuWindow` (the shell's working dismiss
infrastructure). v1.0.13 research confirmed `PopupMenuWindow` works
on niri because `MainScreen` cooperates with it via shared
`BarService.popupOpen` state.

- **Pros:** Reuses a proven-working dismiss path. Visual idiom snaps
  to "calendar-pretty" automatically.
- **Cons:** Upstream donation negotiation with noctalia-shell
  maintainer. Loses plugin status (won't show in plugin marketplace
  if/when that exists). Cross-compositor support harder.
- **Timeline:** depends entirely on upstream maintainer + Pedro's
  appetite for the conversation.

### Option D — Custom Wayland client code in the bridge

Bypass Qt entirely. The Rust bridge already speaks Wayland (via niri-
IPC). Extend it to also be the popup surface owner: bridge creates an
xdg_popup directly via `smithay-client-toolkit`, registers its own
`wl_pointer` listener, dispatches clicks to a QML-side IPC channel.

- **Pros:** Full control over Wayland protocol. No Qt bug
  interference. No Quickshell limitations.
- **Cons:** Massive rewrite. Bridge gains rendering responsibility
  it currently lacks. We'd be writing a Wayland UI toolkit. Months.
- **Timeline:** 3-6 months solo.

### Option E — Niri compositor extension request

File against niri: "expose `niri ipc subscribe pointer_press_outside_layer`
or similar event so plugins can self-dismiss". Pure protocol/IPC
solution, no Qt involvement.

- **Pros:** Solves the class of "layer-shell popup needs dismiss"
  problems for everyone on niri.
- **Cons:** Requires upstream niri buy-in. May be rejected as
  out-of-scope. niri maintainer YaLTeR has historically been receptive
  to plugin-friendly IPC extensions.
- **Timeline:** weeks for the upstream conversation, then niri
  release cycle.

### Option F — Quickshell upstream `NiriFocusGrab` request

`Quickshell.Hyprland.HyprlandFocusGrab` exists for exactly this problem
on Hyprland. Ask outfoxxed (Quickshell maintainer) to implement
`Quickshell.Niri.NiriFocusGrab` — using niri's IPC event stream as the
"focus lost" signal under the hood.

- **Pros:** API-symmetric with the Hyprland version Pedro likely
  already trusts. Quickshell becomes Pedro's single integration point
  for compositor-specific focus grabs.
- **Cons:** Requires Quickshell upstream conversation. Probably
  depends on Option E (niri IPC extension) anyway.
- **Timeline:** weeks-to-months.

### Option G — Accept v1.0.4 reality: stay on PanelWindow, restore the v1.0.0..v1.0.2 in-popup MouseArea

Go back to the v1.0.0 design: popup is a full-screen `PanelWindow` (NOT
a constrained one), all four edges anchored, content positioned via
internal margins, full-screen `MouseArea` inside the surface catches
outside clicks. Yes this is the v1.0.3 "compositor freeze" surface,
**but with a transparent background and content positioned at the menu
rect** — the freeze was content-rendering damage, not the surface
size. Test on Pedro's hardware whether a *transparent* full-screen
layer-shell surface still freezes.

- **Pros:** Conceptually simplest. Reuses the v1.0.0 codepath that
  Pedro tested and reported "bar froze" — but the bar-froze report
  may have conflated xdg_popup grab semantics (normal menubar UX) with
  an actual compositor freeze.
- **Cons:** Risks the same AMD/niri damage cascade.
- **Timeline:** 1 day spike + Pedro test.

## 6. Decision points for Pedro

Pedro must answer the following before any further commit lands:

1. **Acceptable to live with no dismiss in v1.0.16 (Option A) while
   chasing a longer fix?** Yes / No.
2. **Willing to maintain a patched-Qt nixpkgs overlay (Option B)?** Yes
   / No.
3. **Willing to negotiate upstream donation to noctalia-shell (Option
   C)?** Yes / No.
4. **Willing to invest 3-6 months of Rust+Wayland work in the bridge
   (Option D)?** Yes / No.
5. **Want me to file the niri IPC + Quickshell upstream requests
   (Options E + F) regardless of which short-term path we pick?** Yes
   / No.
6. **Want me to spike Option G (transparent full-screen PanelWindow)
   first to rule it out cheaply?** Yes / No.

Default if Pedro stays silent: spike **Option G** as the cheapest test
(~1 day, no upstream coordination), file Options E + F as upstream
issues in parallel. If Option G freezes the compositor, fall back to
Option A and live with the constraint until Qt / niri / Quickshell
upstream lands.

## 7. What I am NOT doing until Pedro decides

- Cutting v1.0.16, v1.0.17, etc.
- Tweaking shield layers, masks, anchors.
- Adding more grab-style hacks.
- Filing more parallel research swarms.

CLAUDE.md decision tree row B/C/E demands a HARD STOP at this point.
This document is the stop.

## 8. Validation criteria once we pick a path

Whichever option we choose, the resulting v1.x.x release must satisfy:

- **SC-014-1:** Clicking inside Firefox content while popup is open
  → popup dismisses within one frame.
- **SC-014-2:** Clicking inside the popup (on a menu row) → row
  activates (existing AT-SPI click flow fires) without popup
  dismissing first.
- **SC-014-3:** Pressing Escape while popup is open → popup dismisses.
- **SC-014-4:** Alt-tabbing to another window → popup dismisses
  (existing `onAppIdChanged` handler must still fire).
- **SC-014-5:** No compositor freeze under any popup open/close
  pattern, no whole-output damage cascades.
- **SC-014-6:** Smoke test added to CI: an offscreen QML harness that
  loads `AppmenuPopupWindow` + simulates the dismiss flow and asserts
  surface state transitions. (CI gap that allowed v1.0.5..v1.0.15
  silent failures; CLAUDE.md trigger F demands this.)

## 9. Open questions

- Is the v1.0.15 Overlay shield actually working (verified at spec-
  write time)? Pedro must report.
- Does niri actually render xdg_popups within their parent layer's
  stratum, or globally above all layers? Empirical test needed.
- Has Qt 6.11 / 6.12 fixed the get_popup ordering bug? Recent qtwayland
  commits not yet read.
