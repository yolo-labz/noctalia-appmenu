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

    /// Spec 009 FR-007 — recursive submenu namespace uniqueness.
    /// Depth 1 is the first SubmenuPopup directly declared inside
    /// `AppmenuPopupWindow`; recursive instances loaded via
    /// `nestedComponent` increment this. The depth suffix lets niri
    /// (and any future window-rule) discriminate sibling layer-shell
    /// surfaces that would otherwise collide on the same namespace
    /// string.
    property int depth: 1

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

    // v1.0.4 — same keep-mapped strategy as AppmenuPopupWindow.
    // visible:true permanently; geometry parks off-screen when
    // !_isOpen so the wl_surface stays mapped across cascade
    // open/close cycles (defeats Quickshell deleteOnInvisible).
    anchors.top: true
    anchors.left: true
    visible: true
    color: "transparent"

    /// Logical open/close state — controls geometry; surface stays mapped.
    property bool _isOpen: false
    property real _surfaceX: 0
    property real _surfaceY: 0
    readonly property int _parkOffset: -10000

    implicitWidth: _isOpen ? menuBox.width : 1
    implicitHeight: _isOpen ? menuBox.height : 1
    margins.top: _isOpen ? _surfaceY : _parkOffset
    margins.left: _isOpen ? _surfaceX : _parkOffset

    // v1.0.10 — promoted to `WlrLayer.Overlay` to match the parent
    // popup (see AppmenuPopupWindow.qml comment). Submenus must stay
    // strictly above the shield AND above their parent popup so a
    // pointer chase across nested submenus never goes through the
    // shield.
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-submenu-d"
                              + depth + "-"
                              + (screen ? screen.name : "unknown")

    /// External "is the submenu currently open" query.
    readonly property alias isOpen: root._isOpen

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
            // SYNC width recalc BEFORE _isOpen flips (codex v1.0.4 fix).
            root._recalcWidth();
            const preferRight = root.anchorRect.x + root.anchorRect.width;
            root._surfaceX = Math.max(0, preferRight);
            root._surfaceY = Math.max(0, root.anchorRect.y);
            root._failedState = false;
            root._isOpen = true;
        } catch (e) {
            console.error("[appmenu/submenu] envelope caught in open:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
            root._isOpen = false;
        }
    }

    function close() {
        try {
            if (nestedLoader.item) {
                nestedLoader.item.close();
            }
            nestedLoader.active = false;  // v1.0.5: tear down via active toggle
            root._isOpen = false;
            root.closed();
        } catch (e) {
            console.error("[appmenu/submenu] envelope caught in close:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
            root._isOpen = false;
        }
    }

    // ── Spec 009 FR-003 — JS-computed menu width ──────────────────────
    // Hidden Text element as metrics oracle (see AppmenuPopupWindow for
    // rationale). Avoids FontMetrics, which qmllint 6.11 in CI doesn't
    // always resolve under our import chain.
    property real _calcWidth: 180
    Text {
        id: _measureText
        visible: false
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
    }
    function _recalcWidth() {
        if (!root.parentMenuItem || !root.parentMenuItem.children) {
            root._calcWidth = 180;
            return;
        }
        const fontSize = _measureText.font.pixelSize;
        const sm = Style.marginS !== undefined ? Style.marginS : 6;
        let maxW = 180;
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
        root._calcWidth = maxW + 2 * Style.marginM;
    }
    onParentMenuItemChanged: _recalcWidth()

    // ── Menu rectangle ──────────────────────────────────────────────
    // PanelWindow itself is sized to menuBox (FR-002), so menuBox
    // simply fills the surface starting at (0,0).
    Rectangle {
        id: menuBox
        visible: root._isOpen && !!root.parentMenuItem
        x: 0
        y: 0
        width: Math.max(180, root._calcWidth)
        height: submenuCol.implicitHeight + Style.marginM * 2

        // v1.0.2 visual contrast bump — match AppmenuPopupWindow.
        color: Color.mSurfaceVariant !== undefined
               ? Color.mSurfaceVariant
               : Color.mSurface
        border.color: Color.mPrimary !== undefined
                      ? Color.mPrimary
                      : Color.mOutline
        border.width: 2
        radius: Style.marginS

        // Swallow clicks on the menu background so the bar's
        // close-on-press hook doesn't fire when the user clicks
        // empty pixels between rows of an open submenu.
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
                        // Spec 009 FR-004 — async-safe nested open.
                        // Loader is asynchronous by default; reading
                        // `nestedLoader.item` immediately after setting
                        // `sourceComponent` returns null in many engine
                        // versions, so the previous immediate
                        // `.open()` call was silently dropped (depth-3+
                        // submenus never appeared).
                        //
                        // Belt-and-braces: store the pending open
                        // request; `_tryOpenNested` fires it when
                        // status reaches Ready. If status is already
                        // Ready synchronously (some engines), we fire
                        // immediately.
                        if (nestedLoader.item) {
                            nestedLoader.item.close();
                        }
                        root._pendingNested = {item: item, anchor: anchor};
                        // v1.0.5 — load via URL string. Inline
                        // `Component { SubmenuPopup {} }` triggers
                        // QML engine "instantiated recursively" error
                        // at parse time (the file references itself
                        // inline). URL-source defers resolution to
                        // load time, breaking the parse-time cycle.
                        if (nestedLoader.source.toString().length === 0) {
                            nestedLoader.source = Qt.resolvedUrl("SubmenuPopup.qml");
                        } else {
                            // Already loaded once — re-trigger by
                            // toggling active.
                            nestedLoader.active = false;
                            nestedLoader.active = true;
                        }
                        root._tryOpenNested();
                    }
                }
            }
        }
    }

    // ── Recursive nested submenu (depth ≥ 3) ─────────────────────────
    // v1.0.5 — recursion via URL-source Loader (not inline Component).
    //
    // The inline `Component { SubmenuPopup {} }` pattern that v1.0.0
    // shipped triggered the QML engine error "SubmenuPopup is
    // instantiated recursively" at parse time of THIS file (a file
    // cannot reference itself inline). The plugin failed to load
    // entirely from v1.0.0 onwards — Pedro's screenshots that DID
    // show menus rendered the pre-v1.0.0 plugin (whatever the shell
    // last successfully loaded).
    //
    // Loader.source as a URL string defers QML type resolution to
    // load time, breaking the parse-time recursion.

    // Spec 009 FR-004 — pending-open record consumed by
    // `_tryOpenNested` when the Loader transitions to Ready.
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
        // v1.0.5 — URL source instead of sourceComponent. Empty
        // string defers actual file load until we set a real URL
        // in the submenuRequested handler.
        source: ""

        // Spec 009 FR-004 — fire pending open as soon as the Loader
        // finishes async instantiation.
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
            function onClosed() {
                // Deeper level closed by outside-click — clear the
                // loader so the next `submenuRequested` can re-trigger.
                nestedLoader.active = false;  // v1.0.5: tear down via active toggle
            }
        }
    }
}
