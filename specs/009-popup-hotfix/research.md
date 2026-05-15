# Research notes — spec 009-popup-hotfix

Spec: `specs/009-popup-hotfix/spec.md`
Plan: `specs/009-popup-hotfix/plan.md`
Date: 2026-05-15
Method: 4 parallel investigation agents (qml-architect,
dbusmenu-protocol-expert, niri-wayland-tester, Explore) +
live state probe in `/tmp/appmenu-probe/`.

No `[NEEDS CLARIFICATION]` markers in the spec — every choice is
backed by direct code reading or live runtime trace. This file
records each design decision, its rationale, and the alternatives
considered.

---

## Decision 1 — Recursive flatten lives in `fetch_menu_tree`, bottom-up

**Decision.** Move the existing wrapper-flatten logic in
`bridge/src/atspi.rs:801-811` from a one-shot post-loop check on the
*parent* item into a per-child check inside the recursive descend.
Each child, immediately after its own `fetch_menu_tree` returns, is
inspected for the `MENU_ITEM → MENU(empty) → [items]` shape and
flattened in place before being pushed to the parent's `children`.

**Rationale.** The current code only fires on the top-level
`MENU_BAR`'s direct children, so `File`/`View`/`Settings`/`Help`
are correctly stripped of their unnamed `MENU` wrapper. But Qt6
emits the same wrapper at every level — `View > Game List Mode >
[List, Grid]` arrives as `Game List Mode (MENU_ITEM, children=[""
(MENU, children=[List, Grid])])`. Without recursive flatten, QML's
`MenuRow.hasChildren` evaluates true on `Game List Mode` (the
wrapper IS in `children`), the cascade opens, but the popup renders
one empty row and the user sees a blank dropdown. This is
[Bug 1, dbusmenu-protocol-expert agent, atspi.rs:801-811].

Bottom-up placement (inside the recursion, before the child is
pushed) keeps descend count unchanged — no quadratic blowup, NFR-003
preserved.

**Alternatives.**
- *Top-down post-pass.* Walk the assembled tree once after
  `fetch_menu_tree` returns and strip wrappers. Rejected — adds an
  O(n) extra pass and complicates the test contract.
- *Strip on QML consume side.* Have `MenuRow.hasChildren` and
  `Repeater.model` skip empty-label MENU children. Rejected —
  pushes substrate-specific knowledge into QML and would have to be
  re-implemented for any future protocol substrate.

---

## Decision 2 — Constrain popup to `menuBox.height`, not full screen

**Decision.** `AppmenuPopupWindow` and `SubmenuPopup` PanelWindows
DROP `anchors.bottom: true`. `height` becomes `menuBox.height` (or
`menuBox.height + 2 * marginM` if a small surround is needed). `y` is
computed from the bar-button's screen-absolute position via
`mapToGlobal(0, 0)` (Qt 5.7+, portable across windows). `x` similarly.
The full-screen outside-click MouseArea is REMOVED; outside-click
dismissal is achieved by making the popup window itself only as
large as the menu, plus a separate `WlrLayer.Background` /
`HyprlandFocusGrab`-style listener if needed (research item below).

**Rationale.** Both popup windows currently anchor to the full
screen (`anchors.top/left/right/bottom: true`). A child
`MouseArea { anchors.fill: parent; onClicked: root.close() }` covers
the entire layer-shell surface. When the user moves the cursor back
toward the bar to click another menu, the popup PanelWindow's
MouseArea is the topmost surface at that point (created later than
the bar surface, on the same layer `WlrLayer.Top`, niri uses
creation order for overlapping surfaces of the same layer). The
click "outside" the menu rectangle but inside the popup PanelWindow
fires the close-on-outside handler — bar click is dropped, menu
closes, perceived as a freeze.
[Bug 2, niri-wayland-tester agent, AppmenuPopupWindow.qml:86-89,133-138]

Shrinking the popup window to `menuBox`'s actual extent removes the
full-screen MouseArea entirely. Cursor-over-bar input lands on the
bar surface, which sees the click and opens the next menu.
Outside-click dismissal then needs a different mechanism — see
sub-decision 2a.

**Sub-decision 2a — Outside-click dismissal without a full-screen
surface.** Use Quickshell's `HyprlandFocusGrab`-style API if
available, else listen to `niri-IPC` `WindowFocusChanged` events
and close the popup on any focus shift. The bridge already
forwards focus events via `IpcHandler.update`; a parallel
`appmenu.dismiss` IPC channel is straightforward. Pedro's
existing reload-test path covers this. If neither hook is
trustworthy in v0.3.0 Quickshell, fall back to a 200×bar-height
"sentinel" MouseArea band IMMEDIATELY ABOVE the menu (only — not
covering the bar) to catch click-outside on the empty desktop area
beneath / next to the menu. To be finalised in research item R3
during Lane Q implementation.

**Alternatives.**
- *Keep full-screen surface + transparent passthrough above the bar.*
  Quickshell does not expose a per-region passthrough flag; a
  `MouseArea { enabled: false }` doesn't actually yield input —
  it just doesn't accept clicks itself, but the surface is still
  the topmost hit-test target. Rejected.
- *Switch to `Quickshell.PopupWindow` with `grabFocus: true`.* This
  is what PR #52 explicitly fixed — `xdg_popup.grab(wl_seat)` makes
  the bar entirely unresponsive while the popup is open. Worse than
  today. Rejected (NFR-002).
- *Composite popup onto bar's PanelWindow as a child item.*
  `noctalia-shell`'s bar uses a single shared full-screen
  PanelWindow per output (`MainScreen.qml`). Hosting our popup in
  that surface would invalidate the shared surface on every focus
  change → AMD flicker bug (PR #51). Rejected.

---

## Decision 3 — `popupCol` width via `childrenRect.width`, not anchors

**Decision.** Drop `anchors.left/right` from `popupCol` and
`submenuCol`. Bind `menuBox.width` to
`Math.max(180, popupCol.childrenRect.width + Style.marginM * 2)`.

**Rationale.** Today `popupCol` anchors to fill `menuBox`, but
`menuBox.width` references `popupCol.implicitWidth`. An anchored-to-
parent Column has `implicitWidth = 0` (its width is dictated by the
anchor, so children don't contribute to implicit size). So
`menuBox.width = max(180, 0 + margins) = 180px` regardless of
content. Long labels like "Show Labels Under Icons" are clipped or
ellipsised. [Bug 3, qml-architect agent, AppmenuPopupWindow.qml:151,189-193]

`childrenRect.width` is QML's idiomatic "size to actual rendered
children" and avoids the circular binding entirely.

**Alternatives.**
- *Set `popupCol.implicitWidth: childrenRect.width` explicitly.*
  Works but is more verbose and requires touching the Column itself.
  `childrenRect` on the menuBox-side binding is one line.
- *Make rows have explicit `width` and sum manually.* Brittle;
  any future row variant requires recomputing the sum.

---

## Decision 4 — Loader async-safe via `Loader.status === Ready` check

**Decision.** Replace the immediate
`nestedLoader.item.open(...)` call after `sourceComponent =
nestedComponent` with a status-check pattern: if
`nestedLoader.status === Loader.Ready` immediately (synchronous
loader case, common for in-line `Component {}`), call `open()` now;
else attach a one-shot `Loader.statusChanged` handler that fires
`open()` on the next `Ready` transition.

**Rationale.** `Loader` instantiation is asynchronous by default
even when the source `Component` is in-process. Reading
`nestedLoader.item` immediately after `sourceComponent =` returns
null in many engine versions; the call is silently dropped, and
the depth-≥3 cascade never opens.
[Bug 5, qml-architect agent, SubmenuPopup.qml:210-215]

Belt-and-braces (synchronous check + Connections listener) handles
both cases without a race.

**Alternatives.**
- *Set `Loader.asynchronous: false`.* Property does not always
  guarantee instant `item` availability; engine still defers to the
  next event-loop tick in some versions.
- *Pre-instantiate the nested SubmenuPopup at parse time.* Breaks
  the recursive design — would require declaring N nested
  SubmenuPopup instances at parse time and the recursion depth is
  unbounded.

---

## Decision 5 — `_sameTopLevel` honours children-shape changes

**Decision.** Extend `_sameTopLevel` to compare `children.length`
on each top-level item AND the first-level child labels. If any
top-level item's first-level children list differs (length OR any
label), the dedup returns false and `topLevel` is reassigned. Deeper
nesting changes are caught by this proxy (a deep child change almost
always rolls up to a first-level structural change because Qt
re-emits the full tree on `accessible-children-changed`).

**Rationale.** Today's check is identity-only (id/label/enabled at
the top level). After a `MenuError::Stale → RefreshActive` re-walk,
identical top-level shapes cause `topLevel = newTopLevel` to be
SKIPPED. The Repeater never re-renders, the in-memory `modelData`
references on each delegate still point at the OLD subtree, and the
next popup open shows the stale children.
[Bug 4, qml-architect + dbusmenu-protocol-expert agents,
BarWidget.qml:247-257]

This widening trades a few extra delegate rebuilds for
correctness — but only on actual menu changes. The PR #51 flicker
risk is bounded because the comparison only fires on real
structural changes.

**Alternatives.**
- *Drop dedup entirely.* Reintroduces PR #51 flicker on every focus
  event for the same app.
- *Compare full subtree structurally (deep equal).* O(n²) on
  large menus, unnecessary — first-level change is a sufficient
  proxy.
- *Bridge sends a generation counter; QML compares it.* Cleaner,
  but requires bridge schema change and is overkill for the actual
  failure mode.

---

## Decision 6 — `focusedScreenName` falls back to `active.json`

**Decision.** Bridge emits an OPTIONAL `focused_output` field in
`active.json` derived from `niri-IPC`'s focused window's `output`
property. QML's `focusedScreenName` derivation tries
`Quickshell.Wayland.ToplevelManager.activeToplevel.screens[0].name`
first; if empty, falls back to the most recent `active.json`
snapshot's `focused_output` field; if still empty, defaults to "" (
permissive guard, current behaviour).

**Rationale.** On niri,
`zwlr_foreign_toplevel_handle_v1.output_enter` fires only AFTER the
toplevel's first `enter` event on a surface. For a freshly-created
window the `screens` list is empty for one or two frames. During
that window, `AppmenuPopupWindow.openAt`'s cross-screen guard at
line 102-112 refuses to open ("cross-screen open refused" log line)
because `focusedScreenName == ""` makes the guard permissive but
`screens[0].name == ""` causes it to look like a mismatch, depending
on how the comparison branches. [Bug 7, niri-wayland-tester agent,
BarWidget.qml:95-108]

Bridge's niri-IPC subscription has the focused output name reliably
(it's part of every focus event payload), so threading it through
`active.json` is one extra field with zero protocol risk.

**Alternatives.**
- *Use `Screen.name` of the bar widget itself.* Wrong on
  multi-monitor — the bar instance lives on screen X but the
  focused window may be on screen Y, exactly the case the guard is
  designed to detect.
- *Wait for `screens[0]` to populate via `Connections`.* Adds
  latency to first-popup-open and complicates the QML state flow.

---

## Decision 7 — Recursive submenu uses depth-suffixed namespace

**Decision.** `SubmenuPopup.qml` accepts an optional `depth: int`
property. When the recursive `nestedComponent` instantiates, it
sets `depth: parentSubmenu.depth + 1` (the top-level
`AppmenuPopupWindow → SubmenuPopup` chain starts at depth 1 for the
first SubmenuPopup). The `WlrLayershell.namespace` becomes
`"noctalia-appmenu-submenu-d" + depth + "-" + screen.name`.

**Rationale.** Today every `SubmenuPopup` (the parent and its
recursively-loaded child via `nestedComponent`) uses the SAME
namespace string. Wayland's `wlr-layer-shell-unstable-v1` does not
forbid duplicate namespaces, but compositors are free to use it for
debugging / styling / window-rule matching, and a niri user with a
window-rule pinned to that namespace would see two surfaces collide
unpredictably. Belt-and-braces fix.
[Bug suspected, niri-wayland-tester agent, SubmenuPopup.qml:84,229-231]

**Alternatives.**
- *Use a UUID per instance.* Breaks user window-rules entirely.
- *Leave as-is.* Latent risk; cheap to fix while we're touching
  the file.

---

## Decision 8 — `_failedState` self-clear extracted into a named function

**Decision.** Extract the latch-clear from inline in
`_applyPending` into a tiny named helper
`_clearFailedStateIfRecovered()`. It is invoked at the end of
every successful apply, BEFORE the `_pendingSnapshot = undefined`
drain. The helper is also invoked once on `_applyPending` entry if
`_pendingSnapshot` is undefined and `_failedState` is true and the
last apply was > 1 tick ago — cheap "self-heal" pass.

**Rationale.** Today the clear lives at line 207 inside
`_applyPending`'s try-block. If the bridge stops re-pushing (the
focused app stays unchanged), the latch is permanent and the widget
stays hidden until a new `applySnapshot` arrives. For long-running
sessions on a single app this is a real outage.
[Bug 8, qml-architect agent SUSPECTED, BarWidget.qml:206-208]

**Alternatives.**
- *Move clear to a `Timer` that polls every N ms.* Wasteful.
- *Drop the `_failedState` machinery entirely.* Throws would
  poison the IPC dispatcher again; ADR (FR-008 spec 003)
  forbids.

---

## Live runtime inputs

`/tmp/appmenu-probe/` snapshot taken at 2026-05-15 17:31:

- `active.json`: `{"app_id":"com.mitchellh.ghostty","focus_pid":6733,"menu":null,"menu_path":"","menu_service":"","title":"HOME | ⠐ nixos","v":1}` — bridge correctly emits `menu: null` for ghostty (no AT-SPI menu).
- `bus.txt`: bridge holds `org.noctalia.AppMenu` per ADR-0008.
- `procs.txt`: bridge running, niri running, no zombie.
- `shell.log`: 30-min noctalia-shell journal — no errors related to appmenu.

The probe confirms the widget infrastructure is healthy when no app
exposes a menu. The bugs are interaction-level, not lifecycle.
