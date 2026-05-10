// noctalia-appmenu — top-level dropdown surface
//
// Swarm B (research note 02-popupwindow-input.md) identified the bar
// click-dead-zone Pedro hit when a top-level menu was open as a Wayland
// PROTOCOL property of `Quickshell.PopupWindow`, not a Quickshell bug:
//
//   • PopupWindow with `grabFocus: true` → Qt sets `Qt::Popup` →
//     QPA-Wayland issues `xdg_popup.grab(wl_seat, serial)`. xdg-shell
//     spec REQUIRES the compositor route ALL pointer/keyboard input
//     to the popup until the grab is released (`popup_done`). The
//     bar surface receives zero events for the duration. There is
//     no compositor-side knob; this is the protocol's defined
//     behaviour for popups that demand "outside-click dismisses me".
//
//   • PopupWindow with `grabFocus: false` → Qt sets `Qt::ToolTip` →
//     no xdg_popup.grab is issued, but Qt-Quick's scene-graph capture
//     on the popup root keeps pointer events away from the parent
//     surface for as long as the popup is up. `propagateComposedEvents`
//     only bubbles within ONE QML scene; it cannot cross `wl_surface`
//     boundaries. So the bar still feels frozen.
//
// Either knob is wrong — the bar must remain clickable while a
// top-level menu is open (the user is about to click another menu, or
// click a bar widget). The fix every mature Quickshell config
// converges on is to STOP using `PopupWindow` for bar dropdowns and
// open a SECOND, sibling `PanelWindow` on `WlrLayer.Top` with:
//
//   • `WlrLayershell.keyboardFocus: WlrKeyboardFocus.None` →
//     `wlr_layer_surface.set_keyboard_interactivity(none)`. The popup
//     window does not request keyboard focus and never participates
//     in any seat grab.
//   • `WlrLayershell.exclusionMode: ExclusionMode.Ignore` → no
//     `set_exclusive_zone`; does not push the desktop around.
//   • Full-screen click-catcher `MouseArea` — closes the popup on
//     outside click, replicating the xdg_popup.grab UX without paying
//     the input-lockout cost.
//
// Sibling layout means Wayland routes input surface-by-surface based
// on cursor position — the bar surface gets clicks over the bar, the
// popup surface gets clicks over its menu rectangle. Verified against
// noctalia's own `Modules/MainScreen/PopupMenuWindow.qml` (the
// canonical noctalia pattern after they refactored away from
// PopupWindow for the same reason).
//
// **Submenus**: still legitimately want input lockout to the parent
// menu while open (so keyboard nav works without leaking to the bar).
// Submenus stay as `PopupWindow` parented to the menu rectangle, NOT
// to the bar. This file does not implement them yet (alpha.19 ships
// only the top-level fix; nested-submenu work tracked separately).

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Commons

PanelWindow {
    id: root

    /// Screen this popup attaches to. One AppmenuPopupWindow instance
    /// is created per BarWidget instance, and BarSection mounts one
    /// BarWidget per screen — so each screen gets its own popup.
    required property ShellScreen screen

    /// The clicked top-level menu button — used to compute popup x/y.
    /// Set by `openAt`; null while popup is closed.
    property Item anchorItem: null

    /// The menu-tree node whose `children` populate the popup. Has
    /// shape { id, label, type, enabled, visible, children: [...] }.
    /// Set by `openAt`; null while popup is closed.
    property var menuItem: null

    /// Emitted when the user activates a leaf row (one without
    /// children). BarWidget connects this and delegates to fireClick.
    signal itemActivated(var item)

    anchors.top: true
    anchors.left: true
    anchors.right: true
    anchors.bottom: true
    visible: false
    color: "transparent"

    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-popup-" + (screen ? screen.name : "unknown")

    /// Open the popup, anchored below the clicked bar button, with
    /// `menuTree.children` populating the rows.
    function openAt(item, menuTree) {
        if (!item || !menuTree) return;
        anchorItem = item;
        menuItem = menuTree;
        visible = true;
    }

    function close() {
        visible = false;
        // Don't null anchorItem/menuItem — bindings can briefly read
        // them during the visible transition; let the next openAt
        // overwrite. The popup is invisible regardless.
    }

    // ── Outside-click dismisser ─────────────────────────────────────
    // Full-screen MouseArea swallows clicks anywhere on this layer-shell
    // surface and closes the popup. The menu rectangle below has its
    // own MouseArea that swallows its events first, so this only fires
    // for clicks OUTSIDE the menu — the desired UX.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onClicked: root.close()
    }

    // ── Menu rectangle ──────────────────────────────────────────────
    // Positioned below `anchorItem`, sized to its content. Uses
    // `mapToItem(null, ...)` to translate the button's position from
    // its (BarWidget) parent coordinates into the popup window's
    // coordinate space. The bar lives on the same screen as this
    // popup window, so window-local coords match.
    Rectangle {
        id: menuBox
        visible: root.visible && !!root.menuItem

        // Width = max(180, content). Height tracks popupCol.
        width: Math.max(180, popupCol.implicitWidth + Style.marginM * 2)
        height: popupCol.implicitHeight + Style.marginM * 2

        // Anchor below the clicked button. mapToItem with null target
        // returns scene-graph (window) coordinates of the anchorItem's
        // (0,0). y = bottom of anchor; clamp x to keep the box on
        // screen.
        x: {
            if (!root.anchorItem) return 0;
            const p = root.anchorItem.mapToItem(null, 0, 0);
            const maxX = root.width - menuBox.width;
            return Math.max(0, Math.min(maxX, p.x));
        }
        y: {
            if (!root.anchorItem) return 0;
            const p = root.anchorItem.mapToItem(null, 0, 0);
            return p.y + root.anchorItem.height;
        }

        color: Color.mSurface
        border.color: Color.mOutline
        border.width: 1
        radius: Style.marginS

        // Block click-through into the outside-click dismisser. Without
        // this, clicks on the menu's own background between rows would
        // close the popup. The Repeater rows have their own MouseAreas.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: false
            onClicked: {
                // Swallow — close-on-outside is handled by the parent
                // MouseArea covering the rest of the surface.
            }
        }

        Column {
            id: popupCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Style.marginXS !== undefined ? Style.marginXS : 4
            spacing: 0

            Repeater {
                model: root.menuItem ? (root.menuItem.children || []) : []
                delegate: Item {
                    id: row
                    required property var modelData
                    readonly property bool isSeparator: modelData && modelData.type === "separator"
                    readonly property bool isVisible: !modelData || modelData.visible !== false
                    visible: isVisible
                    width: parent ? parent.width : 0
                    height: isSeparator ? Style.marginXS * 2 : (Style.barHeight - Style.marginS)

                    // Separator
                    Rectangle {
                        visible: row.isSeparator
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Style.marginS
                        anchors.rightMargin: Style.marginS
                        height: 1
                        color: Color.mOutline
                        opacity: 0.4
                    }

                    // Action / submenu row
                    Rectangle {
                        visible: !row.isSeparator
                        anchors.fill: parent
                        color: rowHover.containsMouse
                            ? Color.mSurfaceVariant
                            : "transparent"
                        radius: Style.marginXS !== undefined ? Style.marginXS : 4

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Style.marginS
                            anchors.rightMargin: Style.marginS
                            spacing: Style.marginS

                            Text {
                                Layout.fillWidth: true
                                text: (row.modelData ? row.modelData.label : "").replace(/_/g, "")
                                color: row.modelData && row.modelData.enabled === false
                                    ? Color.mOnSurfaceVariant
                                    : Color.mOnSurface
                                font.family: Settings.data.ui.fontDefault || "Inter"
                                font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                            }

                            // Submenu indicator
                            Text {
                                visible: row.modelData && row.modelData.children && row.modelData.children.length > 0
                                text: "›"
                                color: Color.mOnSurfaceVariant
                                font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                            }
                        }

                        MouseArea {
                            id: rowHover
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton
                            enabled: row.modelData && row.modelData.enabled !== false
                            onClicked: {
                                if (!row.modelData) return;
                                const hasChildren = row.modelData.children && row.modelData.children.length > 0;
                                if (!hasChildren) {
                                    root.itemActivated(row.modelData);
                                    root.close();
                                }
                                // Nested submenus: TODO alpha.19+. For
                                // now, leaf-only activation.
                            }
                        }
                    }
                }
            }
        }
    }
}
