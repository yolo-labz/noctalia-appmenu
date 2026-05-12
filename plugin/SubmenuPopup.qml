// noctalia-appmenu — nested submenu popup (FR-010, spec 004)
//
// Sibling top-level layer-shell PanelWindow per ADR-0008 + spec 003
// FR-005..FR-007. Hosts the children of a menu item whose
// `hasChildren === true`. NOT a `Popup` nested inside the parent
// `AppmenuPopupWindow` — that pattern would re-introduce the
// `xdg_popup.grab(wl_seat)` seat-steal Pedro fought through PRs
// #50..#52 (research note 02-popupwindow-input.md).
//
// Wayland routes input surface-by-surface: with each level of submenu
// living on its own `PanelWindow`, the bar, the parent popup, and the
// submenu all stay clickable. Outside-click is caught by a full-screen
// `MouseArea` inside this surface (spec 003 FR-006); the parent popup
// observes the close cascade via `closed` signals.
//
// Recursive nesting: this file declares a local `Component {
// SubmenuPopup { } }` factory. QML parses the inner type lazily — at
// instantiation time, not at parse time — so the recursion terminates
// naturally on the finite-depth AT-SPI menu tree (typically ≤ 4 levels
// per spec 004 contracts/submenu-popup-component.md §Test contract).
//
// FR-013 (multi-screen guard): `open` refuses to fire when this
// surface's `screen.name !== focusedScreenName` (and focusedScreenName
// is non-empty). On a multi-monitor host, a submenu cannot appear on
// the wrong output.
//
// Spec 003 FR-008/FR-009 — every public entry (`open`, `close`) and
// the row delegate `onClicked` handlers are wrapped in try/catch
// envelopes that flip `_failedState` on throw. While failed, the
// popup closes and refuses to re-open until the next `open` call (the
// caller decides whether the snapshot has recovered).

import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

PanelWindow {
    id: root

    /// Output the surface lives on. Set by the caller; matches the
    /// screen the parent `AppmenuPopupWindow` is anchored to.
    required property ShellScreen screen

    /// The menu-tree node whose `children` populate this submenu.
    /// Null while closed.
    property var parentMenuItem: null

    /// Window-local geometry of the parent row that triggered open.
    /// Used to anchor the menu box's x/y. Defaults zero-rect.
    property rect anchorRect: Qt.rect(0, 0, 0, 0)

    /// FR-013 (multi-screen guard) — when non-empty and ≠ `screen.name`
    /// the surface refuses to open. Threaded from `BarWidget`
    /// through `AppmenuPopupWindow`.
    property string focusedScreenName: ""

    /// Spec 003 FR-009 — failed-state flag. Set when an envelope catches
    /// a throw; cleared on the next successful `open`.
    property bool _failedState: false

    /// Emitted when the user activates a leaf row (no `children`). The
    /// parent popup (and ultimately `BarWidget`) consumes this and
    /// dispatches the AT-SPI click subprocess.
    signal itemActivated(var item)

    /// Emitted when this popup closes (outside-click, leaf-activation,
    /// or programmatic). The parent popup uses this to collapse the
    /// chain.
    signal closed()

    anchors.top: true
    anchors.left: true
    anchors.right: true
    anchors.bottom: true
    visible: false
    color: "transparent"

    // Same layer-shell envelope as AppmenuPopupWindow (spec 003
    // FR-005..FR-007).
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-submenu-"
                              + (screen ? screen.name : "unknown")

    /// Open this submenu, populated by `menuItem.children`, anchored to
    /// the right edge of `anchor` (falling back to the left edge when
    /// the right would clip off-screen).
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
            // Tear down any deeper nested submenu first so the cascade
            // closes from leaf to root.
            if (nestedLoader.item) {
                nestedLoader.item.close();
            }
            nestedLoader.sourceComponent = null;
            root.visible = false;
            root.closed();
        } catch (e) {
            console.error("[appmenu/submenu] envelope caught in close:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
            root.visible = false;
        }
    }

    // ── Outside-click dismisser ─────────────────────────────────────
    // Full-screen MouseArea swallows clicks anywhere on this surface
    // (the menuBox below covers its own clicks first).
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onClicked: root.close()
    }

    // ── Menu rectangle ──────────────────────────────────────────────
    // Sized to its content; anchored to the right edge of `anchorRect`
    // by default. Falls back to anchoring on the left when the right
    // would push the box off-screen.
    Rectangle {
        id: menuBox
        visible: root.visible && !!root.parentMenuItem

        width: Math.max(180, submenuCol.implicitWidth + Style.marginM * 2)
        height: submenuCol.implicitHeight + Style.marginM * 2

        x: {
            if (!root.parentMenuItem) return 0;
            const preferRight = root.anchorRect.x + root.anchorRect.width;
            const screenRight = root.width;
            if (preferRight + menuBox.width <= screenRight) {
                return preferRight;
            }
            // Fall back: anchor right edge of box to left edge of row.
            const fallback = root.anchorRect.x - menuBox.width;
            return Math.max(0, fallback);
        }
        y: {
            if (!root.parentMenuItem) return 0;
            const maxY = root.height - menuBox.height;
            return Math.max(0, Math.min(maxY, root.anchorRect.y));
        }

        color: Color.mSurface
        border.color: Color.mOutline
        border.width: 1
        radius: Style.marginS

        // Swallow clicks on the menu background so the outside-click
        // dismisser only fires for the real outside region.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: false
            onClicked: {
                // Swallow.
            }
        }

        Column {
            id: submenuCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Style.marginXS !== undefined ? Style.marginXS : 4
            spacing: 0

            Repeater {
                model: root.parentMenuItem
                       ? (root.parentMenuItem.children || [])
                       : []
                delegate: MenuRow {
                    onClicked: function (item) {
                        // Leaf activation — bubble up; close cascade.
                        root.itemActivated(item);
                        root.close();
                    }
                    onSubmenuRequested: function (item, anchor) {
                        // Open a deeper submenu via the recursive
                        // factory. Tear down any prior nested popup
                        // first so only one branch is open at a time.
                        if (nestedLoader.item) {
                            nestedLoader.item.close();
                        }
                        nestedLoader.sourceComponent = nestedComponent;
                        if (nestedLoader.item) {
                            nestedLoader.item.screen = root.screen;
                            nestedLoader.item.focusedScreenName =
                                root.focusedScreenName;
                            nestedLoader.item.open(item, anchor);
                        }
                    }
                }
            }
        }
    }

    // ── Recursive nested submenu (depth ≥ 3) ─────────────────────────
    // Deferred via Loader so the QML graph doesn't infinite-recurse at
    // build time. The inner SubmenuPopup is instantiated only when
    // `nestedLoader.sourceComponent = nestedComponent` fires from a
    // row's `submenuRequested` handler.
    Component {
        id: nestedComponent
        SubmenuPopup { }
    }

    Loader {
        id: nestedLoader
        active: true
        sourceComponent: null

        Connections {
            target: nestedLoader.item
            ignoreUnknownSignals: true
            function onItemActivated(item) {
                root.itemActivated(item);
                root.close();
            }
            function onClosed() {
                // Deeper level closed by outside-click — clear the
                // loader so the next `submenuRequested` can re-trigger.
                nestedLoader.sourceComponent = null;
            }
        }
    }
}
