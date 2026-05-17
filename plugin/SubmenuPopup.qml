// noctalia-appmenu — nested submenu popup (v1.0.12)
//
// v1.0.12 rewrite: was a `PanelWindow` (wlr-layer-shell sibling) for
// six minor releases trying to make outside-click dismissal work via
// a separate "shield" panel; it never worked reliably on niri. This
// version is a `Quickshell.PopupWindow` (xdg_popup) anchored to its
// triggering row Item — the compositor enforces grab semantics across
// the entire popup chain (bar button → top-level popup → row in
// top-level → submenu → row in submenu → deeper submenu …). Same
// pattern as noctalia-shell `TrayMenu.qml` / `NPopupContextMenu.qml`.
//
// FR-013 (multi-screen guard) preserved: `open` refuses to fire when
// the focused output ≠ this popup's screen.
//
// Recursive nesting still loads via URL-source Loader (`v1.0.5` fix:
// inline `Component { SubmenuPopup {} }` triggered QML "instantiated
// recursively" at parse time).

import QtQuick
import Quickshell
import qs.Commons
import qs.Services.UI

PopupWindow {
    id: root

    /// Output the surface lives on. Set by the caller; matches the
    /// screen the parent popup is anchored to.
    required property ShellScreen screen

    /// The menu-tree node whose `children` populate this submenu.
    property var parentMenuItem: null

    /// The row Item that triggered this open. The submenu's
    /// `anchor.item` follows the parent xdg_popup chain.
    property Item anchorItem: null

    /// FR-013 multi-screen guard — when non-empty and ≠ `screen.name`,
    /// `open` refuses.
    property string focusedScreenName: ""

    /// Spec 009 FR-007 — depth tag (1 = first submenu under the
    /// top-level popup). Increments on recursive open via the nested
    /// Loader.
    property int depth: 1

    /// Failed-state flag (FR-009).
    property bool _failedState: false

    signal itemActivated(var item)
    // v1.0.13 — renamed from `closed` to silence
    //   qt.qml.invalidOverride: Duplicate signal name: invalid override
    //   of property change signal or superclass signal
    // PopupWindow inherits Window which already defines `closed`.
    signal closeChain()

    // xdg_popup anchor — to the right of the triggering row, top-edge
    // aligned. Quickshell + the compositor handle screen-edge clipping
    // (flips to left side when there's no room on the right).
    anchor.item: anchorItem
    anchor.rect.x: anchorItem ? anchorItem.width : 0
    anchor.rect.y: 0

    // v1.0.13 — explicit grab so the compositor extends the xdg_popup
    // grab chain across the cascade; clicking outside ANY submenu in
    // the chain dismisses the whole tree.
    grabFocus: true

    implicitWidth: Math.max(220, _calcWidth)
    implicitHeight: menuBox.implicitHeight
    visible: false
    color: "transparent"

    readonly property alias isOpen: root.visible

    function open(menuItem, anchor) {
        try {
            if (!menuItem) return;
            if (root.focusedScreenName.length > 0
                && root.screen
                && root.focusedScreenName !== root.screen.name) {
                console.log("[appmenu] cross-screen submenu open refused:",
                            "popup-screen=", root.screen.name,
                            "focused-screen=", root.focusedScreenName);
                return;
            }
            root.parentMenuItem = menuItem;
            // `anchor` is now an Item (the parent row). Same parameter
            // name as v1.0.11 for backwards compat with the MenuRow
            // signal contract.
            root.anchorItem = anchor;
            root._recalcWidth();
            root._failedState = false;
            root.visible = true;
        } catch (e) {
            console.error("[appmenu/submenu] envelope caught in open:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
            root.visible = false;
        }
    }

    function close() {
        try {
            if (nestedLoader.item) {
                nestedLoader.item.close();
            }
            nestedLoader.active = false;
            root.visible = false;
            root.closeChain();
        } catch (e) {
            console.error("[appmenu/submenu] envelope caught in close:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
            root.visible = false;
        }
    }

    // ── Width measurement (same as before, simpler API) ─────────────
    property real _calcWidth: 220
    Text {
        id: _measureText
        visible: false
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
    }
    function _recalcWidth() {
        if (!root.parentMenuItem || !root.parentMenuItem.children) {
            root._calcWidth = 220;
            return;
        }
        const fontSize = _measureText.font.pixelSize;
        const sm = Style.marginS !== undefined ? Style.marginS : 6;
        let maxW = 220;
        for (let i = 0; i < root.parentMenuItem.children.length; i++) {
            const c = root.parentMenuItem.children[i];
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
    onParentMenuItemChanged: _recalcWidth()

    // ── Menu surface — same visual vocabulary as AppmenuPopupWindow ─
    Rectangle {
        id: menuBox
        anchors.fill: parent
        color: Color.mSurface
        radius: Style.radiusL !== undefined ? Style.radiusL : 12
        border.color: Color.mOutline !== undefined ? Color.mOutline : Color.mPrimary
        border.width: 1
        clip: true

        implicitHeight: submenuCol.implicitHeight + 2 * (Style.marginS !== undefined ? Style.marginS : 6)

        Column {
            id: submenuCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Style.marginS !== undefined ? Style.marginS : 6
            spacing: 0

            Repeater {
                model: root.parentMenuItem ? (root.parentMenuItem.children || []) : []
                delegate: MenuRow {
                    onClicked: function (item) {
                        root.itemActivated(item);
                        root.close();
                    }
                    onSubmenuRequested: function (item, anchor) {
                        if (nestedLoader.item) {
                            nestedLoader.item.close();
                        }
                        root._pendingNested = {item: item, anchor: anchor};
                        if (nestedLoader.source.toString().length === 0) {
                            nestedLoader.source = Qt.resolvedUrl("SubmenuPopup.qml");
                        } else {
                            nestedLoader.active = false;
                            nestedLoader.active = true;
                        }
                        root._tryOpenNested();
                    }
                }
            }
        }
    }

    // ── Recursive nested submenu (depth ≥ 3) — v1.0.5 URL-source pattern
    property var _pendingNested: null
    function _tryOpenNested() {
        if (!nestedLoader.item || nestedLoader.status !== Loader.Ready) {
            return;
        }
        const pend = root._pendingNested;
        if (!pend) return;
        root._pendingNested = null;
        try {
            nestedLoader.item.depth = root.depth + 1;
            nestedLoader.item.screen = root.screen;
            nestedLoader.item.focusedScreenName = root.focusedScreenName;
            nestedLoader.item.open(pend.item, pend.anchor);
        } catch (e) {
            console.error("[appmenu/submenu] nested open failed:", e,
                          "stack:", (e && e.stack) || "(no stack)");
        }
    }

    Loader {
        id: nestedLoader
        active: true
        source: ""

        onStatusChanged: {
            if (status === Loader.Ready) {
                root._tryOpenNested();
            } else if (status === Loader.Error) {
                console.error("[appmenu/submenu] nestedLoader Error status");
                root._pendingNested = null;
            }
        }

        Connections {
            target: nestedLoader.item
            ignoreUnknownSignals: true
            function onItemActivated(item) {
                root.itemActivated(item);
                root.close();
            }
            function onCloseChain() {
                nestedLoader.active = false;
            }
        }
    }
}
