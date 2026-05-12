# Contract: `SubmenuPopup.qml` component (Lane B)

**Status:** new at v1.0.0 (FR-010)
**File:** `plugin/SubmenuPopup.qml`
**Parent surface:** `AppmenuPopupWindow.qml` (the top-level popup that contains the parent menu item with `hasChildren = true`)
**Reference:** ADR-0008 (popup window for submenus)

## Component shape

```qml
PanelWindow {
    id: root
    required property ShellScreen screen
    required property var parentItem        // the menu item that triggered open
    required property point anchorRect      // global (screen-space) rectangle of the parent row

    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-submenu-" + screen.name

    // anchors to the right edge of anchorRect, falling back to left
    // when the right edge would clip off-screen
    visible: false
    opacity: 0

    function open() { ... }
    function close() { ... }
}
```

## Contract guarantees

1. **Sibling top-level**, not nested. The popup is a separate `PanelWindow` at the layer-shell level — it is NOT a child `Popup` of the parent `AppmenuPopupWindow`. Spec 003 FR-005..FR-007 require this to avoid `xdg_popup.grab` stealing the seat from the bar.
2. **`WlrKeyboardFocus.None`.** The popup never requests keyboard interactivity. v1 has no keyboard navigation (deferred to v2 per ADR-0010).
3. **Outside-click closes both.** A full-screen `MouseArea` inside the popup catches outside clicks and calls `close()`. Closing the submenu also signals the parent `AppmenuPopupWindow` to close (single-click outside = close everything).
4. **Anchoring fallback.** If `anchorRect.right + submenu.width > screen.right`, the popup anchors to `anchorRect.left - submenu.width` instead. Always stays on-screen.
5. **`screen` guard.** If `parentItem.screen !== focusedScreen`, refuse to open. FR-013 cross-cuts here.
6. **Leaf click activates.** On click of a leaf row, invokes the bridge's `atspi-click <service> <path>` subprocess (via `Quickshell.Io.Process`), then calls `close()` and signals the parent popup to close.
7. **Row delegate is shared.** Lane B's child spec MAY refactor row rendering into a shared `MenuRow.qml` component used by both `AppmenuPopupWindow` and `SubmenuPopup` — DRY but optional; either way the contract surface is the same.

## Fault-isolation invariants inherited from spec 003

- The submenu surface MUST NOT use `xdg_popup.grab(wl_seat)` (FR-005, FR-006 of spec 003).
- Any animation MUST occur inside the surface's reserved geometry; no animation extends surface bounds (FR-002 of spec 003).
- Exceptions inside the popup's `onClicked` / `Component.onCompleted` MUST be wrapped in `try { ... } catch (e) { console.error("[appmenu/submenu] envelope caught:", e); root._failedState = true; }` (FR-008 of spec 003).
- When `_failedState` is true, the popup MUST close and refuse to re-open until the parent applies a fresh well-formed snapshot.

## Rendering contract

- Rows render `label`, `enabled` (greyed when false), `toggle_state` indicator (FR-011), `icon_name` icon (FR-012), submenu indicator `›` when `hasChildren` is true.
- Click on a row with `hasChildren = true` opens a deeper `SubmenuPopup` instance — nesting depth is unbounded by the component but bounded in practice by AT-SPI tree depth (typically ≤ 4 levels).
- Visible row count: limited by screen height; overflow renders a scrollable area (`ListView` with `clip: true`).

## Theme tokens (FR per spec 003 §Theme integration)

- Background: `Color.mSurface`
- Foreground: `Color.mOnSurface`
- Hover: `Color.mSurfaceVariant`
- Disabled foreground: `Color.mOnSurfaceVariant`
- Borders / separators: `Color.mOutline`
- Spacing: `Style.marginM`, `Style.marginS`, `Style.marginXS`
- Font size: `Style._barBaseFontSize` (or `Style.popupBaseFontSize` if upstream noctalia exposes it)
- No raw hex / rgb values. No arbitrary Tailwind-style spacing.

## Test contract

- **QML fixture test.** Lane B ships `tests/qmltest/submenu_popup.qml` (Qt's `QtTest` framework or equivalent) — instantiates the popup with a hand-crafted JSON tree, asserts the rendered row count, click activates the correct AT-SPI path, outside-click closes.
- **Integration test (post-Lane D merge).** AT-SPI integration test from FR-022 exercises a real Qt6 app with nested menus (kate's `File → Open Recent`) and asserts the submenu opens correctly.
- **Manual smoke** on the desktop host per SC-001: open kate, click `File`, hover `Open Recent`, verify the submenu opens on the correct side, click an entry, verify it activates.

## Non-goals

- Keyboard navigation (cursor keys inside open popup). Deferred to v2 per ADR-0010.
- Mnemonic underlines (Alt-F, Alt-E). Deferred to v2.
- Cascading hover (auto-open submenu on hover after a short delay). v1 = click-to-open only.
- Animated open/close transitions. v1 = instant. Animation is a v1.x polish task.
