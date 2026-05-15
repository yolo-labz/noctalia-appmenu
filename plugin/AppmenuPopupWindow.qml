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
// **Submenus (FR-010 spec 004, this revision)**: nested submenus open
// on a *separate* sibling top-level `PanelWindow` (`SubmenuPopup`),
// declared below as a child of this surface's QML tree but mounted as
// its own Wayland surface — Quickshell `PanelWindow` is always a
// top-level layer-shell surface, never a `Popup` of its declarative
// parent. Cascading depth ≥ 3 is supported recursively from inside
// `SubmenuPopup.qml`.

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

    /// FR-013 (spec 004) — multi-screen popup-routing guard. When
    /// non-empty and ≠ `screen.name`, `openAt` refuses to open. Set
    /// from `BarWidget` based on the focused window's output.
    property string focusedScreenName: ""

    /// Emitted when the user activates a leaf row (one without
    /// children, at any nesting depth). BarWidget connects this and
    /// delegates to fireClick.
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
        if (root.focusedScreenName.length > 0
            && root.screen
            && root.focusedScreenName !== root.screen.name) {
            // FR-013 — refuse to open on the wrong screen. Visible only
            // in `journalctl --user -u noctalia-shell`; no user-facing
            // failure.
            console.log("[appmenu] cross-screen open refused:",
                        "popup-screen=", root.screen.name,
                        "focused-screen=", root.focusedScreenName);
            return;
        }
        anchorItem = item;
        menuItem = menuTree;
        visible = true;
    }

    function close() {
        // Tear down any open submenu chain first so we close
        // leaf-to-root and don't leave orphan surfaces.
        if (submenu.visible) submenu.close();
        visible = false;
        // Don't null anchorItem/menuItem — bindings can briefly read
        // them during the visible transition; let the next openAt
        // overwrite. The popup is invisible regardless.
    }

    // ── Spec 009 FR-003 — JS-computed menu width ──────────────────────
    // Re-computed when menuItem changes. Avoids the popupCol-anchored
    // implicitWidth=0 circular-binding trap (Column anchored left+right
    // to menuBox has implicitWidth = 0; menuBox.width clamped at 180px).
    //
    // Uses a hidden Text element as a metrics oracle — Text.implicitWidth
    // is the painted width of the assigned text in the bound font. Avoids
    // pulling in `FontMetrics` (qmllint 6.11 in CI doesn't always resolve
    // it under our import chain).
    property real _calcWidth: 180
    Text {
        id: _measureText
        visible: false
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
    }
    function _recalcWidth() {
        if (!root.menuItem || !root.menuItem.children) {
            root._calcWidth = 180;
            return;
        }
        const fontSize = _measureText.font.pixelSize;
        const sm = Style.marginS !== undefined ? Style.marginS : 6;
        let maxW = 180;
        for (let i = 0; i < root.menuItem.children.length; i++) {
            const c = root.menuItem.children[i];
            if (!c || !c.label) continue;
            if (c.type === "separator" || c.item_type === "separator") continue;
            const label = String(c.label).replace(/_/g, "");
            _measureText.text = label;
            const labelW = _measureText.implicitWidth;
            // Slot extras: icon + toggle + chevron + spacing (4 × marginS).
            let extra = 4 * sm;
            if (c.icon_name) extra += fontSize + sm;
            if (c.toggle_type) extra += fontSize + sm;
            if (c.children && c.children.length > 0) extra += fontSize + sm;
            const total = labelW + extra;
            if (total > maxW) maxW = total;
        }
        // Add menuBox horizontal margins (Style.marginM × 2).
        root._calcWidth = maxW + 2 * Style.marginM;
    }
    onMenuItemChanged: _recalcWidth()

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

        // Spec 009 FR-003 — width derives from a JS calculation over
        // the menu's labels, NOT from popupCol.implicitWidth (which is
        // 0 because popupCol is anchored left+right to menuBox — an
        // anchored Column has implicitWidth = 0). Without this, the
        // width clamps at 180px and long labels like "Show Labels
        // Under Icons" are clipped. Recomputed via root._recalcWidth()
        // each time menuItem changes.
        width: Math.max(180, root._calcWidth)
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
                delegate: MenuRow {
                    onClicked: function (item) {
                        // Leaf at this depth — bubble to BarWidget +
                        // collapse the whole popup chain.
                        root.itemActivated(item);
                        root.close();
                    }
                    onSubmenuRequested: function (item, anchor) {
                        // FR-010 — open the nested SubmenuPopup as a
                        // sibling top-level surface. anchorRect carries
                        // the row's geometry in window-local coords.
                        if (submenu.visible) submenu.close();
                        submenu.open(item, anchor);
                    }
                }
            }
        }
    }

    // ── Nested submenu (FR-010) ─────────────────────────────────────
    // SubmenuPopup is its own top-level Wayland surface — declaring it
    // inside this QML object does NOT make it a child Wayland surface;
    // Quickshell `PanelWindow` always promotes to a layer-shell
    // top-level. Each level of submenu therefore stays clickable
    // independently of the bar and of this popup.
    SubmenuPopup {
        id: submenu
        screen: root.screen
        focusedScreenName: root.focusedScreenName

        onItemActivated: function (item) {
            // Leaf activation at any deeper depth bubbles through the
            // chain. AppmenuPopupWindow signals BarWidget; submenu has
            // already closed itself.
            root.itemActivated(item);
            root.close();
        }
    }
}
