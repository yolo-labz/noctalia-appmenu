// noctalia-appmenu — AppMenu dropdown popup (v1.0.12).
//
// Architectural pivot from v1.0.4..v1.0.11
// =========================================
// v1.0.0..v1.0.2 used `Quickshell.PopupWindow` (xdg_popup) but Pedro
// reported the bar appeared frozen while the popup was up — actually
// the EXPECTED behaviour of `xdg_popup.grab(wl_seat)` (compositor
// routes pointer/keyboard exclusively to the popup) misdiagnosed as
// a bug. v1.0.3 switched to `PanelWindow` (wlr-layer-shell) to keep
// the bar clickable while the menu is up, then v1.0.4..v1.0.11 spent
// seven releases fighting Wayland's surface ordering / input region
// semantics to add a separate "shield" panel for outside-click
// dismissal. None of them dismissed reliably.
//
// v1.0.12 reverts to `Quickshell.PopupWindow` — same pattern noctalia-
// shell uses for `TrayMenu`, `NPopupContextMenu`, and friends — with
// the styling vocabulary borrowed from `CalendarHeaderCard` /
// `BrightnessCard` etc (rounded surfaces, Color.mSurface base,
// Color.mPrimary accent, Style.radiusL corners).
//
// Trade-off accepted from the v1.0.3 retreat:
//   - While the popup is open, clicks on the bar do NOT directly
//     fire bar buttons. The compositor closes the popup first;
//     the user then clicks the bar again to open a different menu.
//     That's the standard Linux/macOS menubar UX (KDE, GNOME do the
//     same) — the previous "bar feels frozen" framing was wrong.
//
// What we keep:
//   - The Qt-flatten + AT-SPI walk in the bridge (v1.0.8/v1.0.10)
//   - The MenuRow + SubmenuPopup widgets (now styled to match)
//   - The honest-or-hidden UX (bridge writes `menu: null` for apps
//     without a menubar; bar widget collapses).

import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.Commons
import qs.Services.UI

PopupWindow {
    id: root

    /// Screen this popup attaches to (set by BarWidget).
    required property ShellScreen screen

    /// The clicked top-level menu button — drives `anchor.item` and
    /// the surface position via `anchor.rect.y`. Set by `openAt`.
    property Item anchorItem: null

    /// The menu-tree node whose `children` populate the popup. Has
    /// shape `{ id, label, type, enabled, visible, children: [...] }`.
    /// Set by `openAt`.
    property var menuItem: null

    /// FR-013 (spec 004) — multi-screen popup-routing guard. When
    /// non-empty and ≠ `screen.name`, `openAt` refuses to open.
    property string focusedScreenName: ""

    /// Emitted when the user activates a leaf row at any depth.
    /// BarWidget connects this and delegates to fireClick.
    signal itemActivated(var item)

    // xdg_popup is anchored to a parent QtQuick Item; positioning is
    // relative to that item's window-local origin. `anchor.rect.y =
    // item.height` aligns the popup top edge with the bar button's
    // bottom edge. Quickshell handles screen-edge clipping for us.
    anchor.item: anchorItem
    anchor.rect.x: 0
    anchor.rect.y: anchorItem ? anchorItem.height : 0

    // v1.0.13 — CRITICAL FIX. Quickshell `PopupWindow.grabFocus` defaults
    // to FALSE; without it, ProxyPopupWindow sets the window flag to
    // `Qt::ToolTip` rather than `Qt::Popup` (see Quickshell source
    // `src/window/popupwindow.cpp:63`). Qt::ToolTip means NO
    // `xdg_popup.grab(wl_seat)` request is made, which means the
    // compositor never auto-dismisses the popup on outside click. v1.0.12
    // looked correct in code but silently fell back to no-grab — that's
    // why image #6 showed the menu staying open even after the architecture
    // pivot. Setting this true makes Qt issue the grab, niri honours it
    // (niri#1810 already fixed in v26.04), compositor fires `popup_done`
    // on any outside press → Quickshell flips `visible: false`.
    grabFocus: true

    implicitWidth: Math.max(220, _calcWidth)
    implicitHeight: menuBox.implicitHeight
    visible: false
    color: "transparent"

    /// Public API — open the popup anchored to `item`, populating
    /// rows from `menuTree.children`. `item` must live in the bar's
    /// window so xdg_popup's parent resolution works.
    function openAt(item, menuTree) {
        if (!item || !menuTree) return;
        if (root.focusedScreenName.length > 0
            && root.screen
            && root.focusedScreenName !== root.screen.name) {
            console.log("[appmenu] cross-screen open refused:",
                        "popup-screen=", root.screen.name,
                        "focused-screen=", root.focusedScreenName);
            return;
        }
        anchorItem = item;
        menuItem = menuTree;
        _recalcWidth();
        visible = true;
    }

    function close() {
        if (submenu && submenu.isOpen) submenu.close();
        visible = false;
    }

    readonly property alias isOpen: root.visible
    function visibleForLogic() { return root.visible; }

    // ── Width measurement ───────────────────────────────────────────
    // Same pattern as before: render labels into an invisible Text and
    // pick the widest. Avoids a circular binding through `Column`.
    property real _calcWidth: 220
    Text {
        id: _measureText
        visible: false
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
    }
    function _recalcWidth() {
        if (!root.menuItem || !root.menuItem.children) {
            root._calcWidth = 220;
            return;
        }
        const fontSize = _measureText.font.pixelSize;
        const sm = Style.marginS !== undefined ? Style.marginS : 6;
        let maxW = 220;
        for (let i = 0; i < root.menuItem.children.length; i++) {
            const c = root.menuItem.children[i];
            if (!c || !c.label) continue;
            if (c.type === "separator" || c.item_type === "separator") continue;
            const label = String(c.label).replace(/_/g, "");
            _measureText.text = label;
            const labelW = _measureText.implicitWidth;
            let extra = 4 * sm;
            if (c.icon_name) extra += fontSize + sm;
            if (c.toggle_type) extra += fontSize + sm;
            if (c.children && c.children.length > 0) extra += fontSize + sm;
            const total = labelW + extra;
            if (total > maxW) maxW = total;
        }
        root._calcWidth = maxW + 2 * Style.marginL;
    }
    onMenuItemChanged: _recalcWidth()

    // ── Menu surface ────────────────────────────────────────────────
    // Visual vocabulary follows noctalia-shell's card style
    // (Modules/Cards/CalendarHeaderCard.qml et al): Style.radiusL
    // rounding, Color.mSurface base, subtle Color.mOutline border.
    Rectangle {
        id: menuBox
        anchors.fill: parent
        color: Color.mSurface
        radius: Style.radiusL !== undefined ? Style.radiusL : 12
        border.color: Color.mOutline !== undefined ? Color.mOutline : Color.mPrimary
        border.width: 1
        clip: true

        implicitHeight: popupCol.implicitHeight + 2 * (Style.marginS !== undefined ? Style.marginS : 6)

        Column {
            id: popupCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Style.marginS !== undefined ? Style.marginS : 6
            spacing: 0

            Repeater {
                model: root.menuItem ? (root.menuItem.children || []) : []
                delegate: MenuRow {
                    onClicked: function (item) {
                        root.itemActivated(item);
                        root.close();
                    }
                    onSubmenuRequested: function (item, anchor) {
                        if (submenu.isOpen) submenu.close();
                        submenu.open(item, anchor);
                    }
                }
            }
        }
    }

    // ── Nested submenu (FR-010) ─────────────────────────────────────
    // `SubmenuPopup` is now also a `PopupWindow` (see SubmenuPopup.qml).
    // Anchoring to a row inside this popup chains the xdg_popup parent
    // relationship so the compositor's grab semantics span the cascade.
    SubmenuPopup {
        id: submenu
        screen: root.screen
        focusedScreenName: root.focusedScreenName

        onItemActivated: function (item) {
            root.itemActivated(item);
            root.close();
        }
    }
}
