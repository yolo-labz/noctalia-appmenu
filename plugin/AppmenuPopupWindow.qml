// noctalia-appmenu — AppMenu dropdown popup (v1.0.16 — Option G)
//
// Architectural pivot per spec 014 — Option G (transparent full-screen
// PanelWindow + in-popup MouseArea). Pedro decision 18/05/2026.
//
// What the previous 11 releases (v1.0.5..v1.0.15) got wrong
// =========================================================
// - v1.0.0..v1.0.2: `Quickshell.PopupWindow` (xdg_popup) — Qt 6.10.2
//   does not issue `xdg_popup.grab` when transient parent is wlr-layer-
//   shell (niri#1810/Smithay#1761 client-side regression). No dismiss.
// - v1.0.3..v1.0.4: tried to fix the v1.0.2 freeze by constraining the
//   PanelWindow surface to a small rect — that broke outside-click
//   dismissal entirely.
// - v1.0.9..v1.0.15: six separate "shield" attempts (visible toggle,
//   anchor toggle, mask Region toggle, Top layer, Overlay layer,
//   xdg_popup retry) — every one broke for a different niri /
//   Quickshell / Qt edge case.
//
// What v1.0.16 does
// =================
// Returns to the simplest possible design:
//
//   PanelWindow {                       // wlr-layer-shell
//     anchors.top/bottom/left/right     // FULL SCREEN
//     color: "transparent"              // paints nothing
//     WlrLayershell.layer: Overlay      // above noctalia MainScreen
//
//     MouseArea { anchors.fill: parent  // OUTSIDE-CLICK CATCHER
//                 onPressed: root.close() }
//
//     Rectangle {                       // ACTUAL MENU
//       x/y: <computed from anchor btn>
//       width/height: <measured>
//       MouseArea { anchors.fill: parent
//                   onClicked: /* swallow — inside the menu */ }
//       ColumnLayout { /* MenuRow children */ }
//     }
//   }
//
// Why the v1.0.3 freeze does not apply here
// =========================================
// The v1.0.3 freeze (noctalia-shell#2216) was caused by whole-output
// damage every frame the popup was up — i.e. the surface was repainting
// content. v1.0.16's outer PanelWindow paints NOTHING (color is
// transparent + no children at the root level). Only the inner menu
// Rectangle paints, and only its small bounding box. Wayland damage is
// per-rect; the rest of the screen sees no damage events.
//
// Visible side effect: the popup grabs ALL pointer input while open
// (the wlr-layer-shell surface covers the screen). Clicks on Firefox,
// the desktop, etc. all flow through this surface's MouseArea — which
// closes the popup. Clicks on the menu Rectangle are caught by its own
// MouseArea (inside the popup MenuRow delegate) and processed normally.
// Clicks on the BAR are NOT delivered to the bar while popup is open,
// because the popup surface is on Overlay (above the bar's Top layer).
// That matches standard macOS/Linux menubar UX — first click closes
// the popup, second click opens the new bar button.

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Commons
import qs.Services.UI

PanelWindow {
    id: root

    /// Screen the popup attaches to (set by BarWidget).
    required property ShellScreen screen

    /// The clicked top-level menu button — used to compute the menu
    /// Rectangle's screen-relative position. Set by `openAt`.
    property Item anchorItem: null

    /// The menu-tree node whose `children` populate the popup rows.
    property var menuItem: null

    /// FR-013 (spec 004) — multi-screen popup-routing guard.
    property string focusedScreenName: ""

    /// Emitted when the user activates a leaf row.
    signal itemActivated(var item)

    // ── Full-screen transparent surface ─────────────────────────────
    visible: false
    color: "transparent"
    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-popup-" + (screen ? screen.name : "unknown")

    // ── Public API ──────────────────────────────────────────────────
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
        try {
            const p = item.mapToGlobal(0, 0);
            root._menuX = Math.max(0, p.x);
            root._menuY = Math.max(0, p.y + item.height);
        } catch (e) {
            root._menuX = 0;
            root._menuY = 0;
        }
        visible = true;
    }

    function close() {
        if (submenu && submenu.isOpen) submenu.close();
        visible = false;
    }

    readonly property alias isOpen: root.visible
    function visibleForLogic() { return root.visible; }

    property real _menuX: 0
    property real _menuY: 0

    // ── Width measurement ───────────────────────────────────────────
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

    // ── OUTSIDE-CLICK CATCHER ───────────────────────────────────────
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onPressed: function (mouse) {
            console.log("[appmenu] outside-click dismiss at",
                        Math.round(mouse.x), Math.round(mouse.y));
            root.close();
        }
    }

    // ── Menu surface (visible content, only this rect paints) ──────
    Rectangle {
        id: menuBox
        // v1.0.17 — visual polish per specs/013-sota-overhaul/visual-spec.md.
        // Canonical noctalia card vocabulary (matches NPopupContextMenu):
        //   surface=mSurface, border=mOutline at Style.borderS,
        //   radius=Style.radiusM (16 px at default ratio), open/close
        //   opacity fade Style.animationNormal/OutQuad.
        visible: root.visible && !!root.menuItem
        x: root._menuX
        y: root._menuY
        width: Math.max(220, root._calcWidth)
        height: popupCol.implicitHeight + 2 * (Style.marginS !== undefined ? Style.marginS : 6)
        color: Color.mSurface
        radius: Style.radiusM !== undefined ? Style.radiusM : 16
        border.color: Color.mOutline !== undefined ? Color.mOutline : Color.mPrimary
        border.width: Style.borderS !== undefined ? Style.borderS : 1
        clip: true

        // Open/close fade — shell idiom from NPopupContextMenu.qml:214-222.
        opacity: root.visible ? 1.0 : 0.0
        Behavior on opacity {
            NumberAnimation {
                duration: Style.animationNormal !== undefined ? Style.animationNormal : 300
                easing.type: Easing.OutQuad
            }
        }

        // SWALLOW clicks on the menu background so they do NOT bubble
        // to the outer outside-click catcher.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: false
            onPressed: function (mouse) {
                mouse.accepted = true;
            }
        }

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

    // ── Nested submenu (FR-010) — same Option G pattern ─────────────
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
