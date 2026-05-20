// noctalia-appmenu — nested submenu popup (v1.0.16 — Option G)
//
// Same architecture as AppmenuPopupWindow.qml v1.0.16:
//   full-screen transparent PanelWindow on Overlay
//   + outside-click MouseArea
//   + inner menu Rectangle positioned at computed coords
//
// The submenu's menu Rectangle is positioned to the right of the
// triggering row (anchorRect.x + anchorRect.width, anchorRect.y).
// `anchorRect` is the parent MenuRow's screen-absolute rect computed
// via mapToGlobal in MenuRow.qml.
//
// Recursive nesting via URL-source Loader stays as before — that
// pattern works.

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import qs.Services.UI
import qs.Widgets

PanelWindow {
    id: root

    /// Output the surface lives on. Set by the caller.
    required property ShellScreen screen

    /// The menu-tree node whose `children` populate this submenu.
    property var parentMenuItem: null

    /// Screen-absolute rect of the parent row that triggered open.
    /// Submenu Rectangle is positioned to the right of this rect.
    property rect anchorRect: Qt.rect(0, 0, 0, 0)

    /// FR-013 multi-screen guard.
    property string focusedScreenName: ""

    /// Spec 009 FR-007 — depth tag for namespace uniqueness.
    property int depth: 1

    /// Failed-state flag (FR-009).
    property bool _failedState: false

    signal itemActivated(var item)
    signal closeChain()

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
    WlrLayershell.namespace: "noctalia-appmenu-submenu-d"
                              + depth + "-"
                              + (screen ? screen.name : "unknown")

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
            root.anchorRect = anchor || Qt.rect(0, 0, 0, 0);
            root._recalcWidth();
            const preferRight = root.anchorRect.x + root.anchorRect.width;
            root._menuX = Math.max(0, preferRight);
            root._menuY = Math.max(0, root.anchorRect.y);
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

    // ── OUTSIDE-CLICK CATCHER ───────────────────────────────────────
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onPressed: function (mouse) {
            console.log("[appmenu/submenu d" + root.depth
                        + "] outside-click dismiss at",
                        Math.round(mouse.x), Math.round(mouse.y));
            root.close();
        }
    }

    // ── Drop shadow (sibling pattern — mirror of AppmenuPopupWindow) ─
    NDropShadow {
        anchors.fill: menuBox
        source: menuBox
        visible: menuBox.visible
    }

    // ── Menu Rectangle (visible content) ───────────────────────────
    Rectangle {
        id: menuBox
        // v1.0.18 — mirror of AppmenuPopupWindow.qml visual iter 2.
        visible: root.visible && !!root.parentMenuItem
        x: root._menuX
        y: root._menuY
        width: Math.max(220, root._calcWidth)
        height: submenuCol.implicitHeight + 2 * Style.marginM
        color: Color.mSurface
        radius: Style.radiusL
        border.width: 0
        clip: true

        opacity: root.visible ? 1.0 : 0.0
        Behavior on opacity {
            NumberAnimation {
                duration: Style.animationNormal
                easing.type: Easing.OutQuad
            }
        }

        // Swallow inside-menu clicks.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: false
            onPressed: function (mouse) {
                mouse.accepted = true;
            }
        }

        Column {
            id: submenuCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Style.marginM
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
                        // Spec 015 FR-006 cascade self-heal. When the
                        // bridge's AT-SPI walker raced lazy realisation
                        // and serialised this submenu's children empty,
                        // kick RefreshActive and re-try the open once.
                        // Single-shot ceiling per cascade-open session
                        // (FR-005) — _cascadePendingRetry resets in the
                        // retry timer before the inspect, so a
                        // repeatedly-empty walk produces one fall-through
                        // to native click rather than a refresh storm.
                        if ((!item.children || item.children.length === 0)
                            && root._cascadePendingRetry === null) {
                            console.log("[appmenu/submenu] empty cascade — triggering RefreshActive retry:",
                                        item.label);
                            root._cascadePendingRetry = {item: item, anchor: anchor};
                            root._cascadeRetryCount++;
                            refreshActiveProcess.running = true;
                            cascadeRetryTimer.restart();
                            return;
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

    // ── Recursive nested submenu (depth ≥ 3) — v1.0.5 URL pattern ──
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

    // ── Cascade self-heal (spec 015 FR-005 + FR-006) ────────────────
    /// Captured submenu-anchor for the retry-open path. Single-shot —
    /// reset before retry to bound the loop at one fall-through.
    property var _cascadePendingRetry: null
    /// Telemetry counter; bumped each time the cascade self-heal fires
    /// in this popup's lifetime. Emitted on close so steady-state runs
    /// can grep `retried=0` in journal.
    property int _cascadeRetryCount: 0

    Process {
        id: refreshActiveProcess
        command: [
            "gdbus", "call", "--session",
            "--dest", "org.noctalia.AppMenu",
            "--object-path", "/org/noctalia/AppMenu/Active",
            "--method", "org.noctalia.AppMenu.Active.RefreshActive"
        ]
    }

    Timer {
        id: cascadeRetryTimer
        interval: 250
        repeat: false
        onTriggered: {
            const pend = root._cascadePendingRetry;
            root._cascadePendingRetry = null;
            if (!pend) return;
            const item = pend.item;
            if (item && item.children && item.children.length > 0) {
                console.log("[appmenu/submenu] cascade self-heal succeeded:",
                            item.label, "children:", item.children.length);
                root._pendingNested = {item: item, anchor: pend.anchor};
                if (nestedLoader.source.toString().length === 0) {
                    nestedLoader.source = Qt.resolvedUrl("SubmenuPopup.qml");
                } else {
                    nestedLoader.active = false;
                    nestedLoader.active = true;
                }
                root._tryOpenNested();
            } else {
                console.log("[appmenu/submenu] cascade self-heal STILL empty for:",
                            item ? item.label : "(unknown)",
                            "— falling through to native click");
                root.itemActivated(pend.item);
                root.close();
            }
        }
    }

    onVisibleChanged: {
        if (!visible) {
            console.log("[appmenu/submenu] popup-close depth=", root.depth,
                        "retried=", root._cascadeRetryCount);
            root._cascadeRetryCount = 0;
            root._cascadePendingRetry = null;
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
