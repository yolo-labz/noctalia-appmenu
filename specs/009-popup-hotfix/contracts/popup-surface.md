# Contract: popup PanelWindow surface geometry

Spec: `specs/009-popup-hotfix/spec.md` FR-002, FR-003, FR-007

## Producer

`plugin/AppmenuPopupWindow.qml` and `plugin/SubmenuPopup.qml` (the
recursive child instances inherit the same contract).

## Consumer

The compositor (niri 25.x) and the user (interaction).

## Behavioural contract

### Surface size

Each popup PanelWindow MUST be sized to the actual menu rectangle
plus a small surround if needed for shadow / border:

- `width` = `menuBox.width` (which is sized to content per FR-003)
- `height` = `menuBox.height`
- NO `anchors.bottom: true`. NO `anchors.right: true` (only `anchors.top`
  and `anchors.left` may remain to anchor the window to the screen
  origin if Quickshell requires SOMETHING).

### Surface position

- `x` = bar-button screen-absolute X (via `mapToGlobal(0, 0).x`)
  clamped to `[0, screenWidth - menuBox.width]`.
- `y` = bar-button screen-absolute Y + `bar-button.height` (i.e. the
  bottom edge of the bar button).
- For nested `SubmenuPopup`, `x` = parent row's right edge in screen
  coords (fall back to left edge - menuBox.width if right would clip).
- For nested `SubmenuPopup`, `y` = parent row's top edge in screen
  coords, clamped to keep the box on-screen.

### Hit testing

- Cursor input over the bar's y-band MUST land on the bar
  PanelWindow, NOT on a popup PanelWindow. FR-002 verification
  asserts this.
- Cursor input over the menu rectangle lands on the menu (existing
  behaviour, preserved).
- Cursor input AROUND the menu (on the desktop / wallpaper area) is
  the outside-click case. Dismissal mechanism: see Decision 2a in
  research.md — preferred path is a Quickshell focus-grab API or a
  bridge-side IPC dismiss signal on focus shift; fallback is no
  immediate dismissal (popup closes when bar clicks switch
  top-level menu, when ESC is pressed, when leaf is activated, or
  when focus moves to another window).

### Layer-shell namespace (FR-007)

| Surface | Namespace |
|---|---|
| `AppmenuPopupWindow` | `noctalia-appmenu-popup-<screen.name>` |
| Depth-1 `SubmenuPopup` | `noctalia-appmenu-submenu-d1-<screen.name>` |
| Depth-N `SubmenuPopup` (recursive) | `noctalia-appmenu-submenu-dN-<screen.name>` |

The depth counter starts at 1 for the first SubmenuPopup directly
declared inside `AppmenuPopupWindow`, and increments by 1 for each
recursive descent inside `SubmenuPopup`'s `nestedComponent`.

### `WlrLayershell` invariants (preserved from PR #52)

- `layer: WlrLayer.Top` — unchanged.
- `keyboardFocus: WlrKeyboardFocus.None` — unchanged.
- `exclusionMode: ExclusionMode.Ignore` — unchanged.
- NO switch to `Quickshell.PopupWindow` — would re-introduce
  `xdg_popup.grab(wl_seat)` and re-break the bar (NFR-002).

## Test contracts

- **`plugin/tests/qmltest/popup_geometry.qml`** (new) — instantiates
  an `AppmenuPopupWindow` with a mock anchor item at scene coords
  `(100, 32)` (bar-button bottom-left); asserts:
  - `popup.x ~= 100` (within 1px tolerance for fractional scaling)
  - `popup.y ~= 32 + bar-button.height`
  - `popup.width === menuBox.width` and < screen width
  - `popup.height === menuBox.height` and < screen height
- **`plugin/tests/qmltest/submenu_cascade.qml`** (new) — opens
  depth-3 cascade synthetically; asserts every `SubmenuPopup` at
  every depth has `visible == true` and its `WlrLayershell.namespace`
  matches the depth-suffixed pattern.
- **Manual smoke (SC-003).** With a popup open, click the next
  top-level button on the bar; assert the popup closes AND the new
  popup opens (not a no-op).
- **Manual smoke (SC-001).** With a long-label menu open
  (`Show Labels Under Icons`), screenshot; assert the menu is
  visible as a distinct rectangle with border + radius, label fully
  rendered.

## Backwards compatibility

This is a behavioural change with no API surface; downstream
consumers (none exist for the PanelWindow geometry) are unaffected.
