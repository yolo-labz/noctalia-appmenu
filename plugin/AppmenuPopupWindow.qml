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

    // v1.0.3 FR-002 — constrained surface (spec 010).
    //
    // CRITICAL fix for the whole-compositor freeze Pedro reported on
    // 15/05/2026 against shadPS4QtLauncher + Firefox: the previous
    // implementation anchored the PanelWindow to all four screen
    // edges, producing a 1920×1080 wlr-layer-shell surface on
    // `WlrLayer.Top`. On AMD GPUs under niri this triggers
    // whole-output damage every frame the popup is up (cited:
    // noctalia-shell #2216). Combined with a full-screen MouseArea
    // outside-click catcher, Qt's Wayland QPA also held implicit
    // pointer capture on the surface — every event went to the
    // popup, the bar froze, the screen "froze."
    //
    // Fix: anchor only `top + left` (Quickshell PanelWindow contract:
    // two adjacent anchors = positionable surface; two opposite
    // anchors = forced screen-size). Set `implicitWidth/Height` to
    // the menuBox extent. Position via `margins.top/left` from the
    // anchor item's screen-absolute coords.
    //
    // The full-screen outside-click MouseArea is dropped — outside
    // dismissal is now BarWidget's responsibility (closes the popup
    // when any bar press lands), plus leaf-click and bar-button-
    // re-click as today.
    // v1.0.4 — keep wl_surface MAPPED across open/close cycles.
    //
    // Quickshell `wlr_layershell.cpp::deleteOnInvisible() == true` —
    // every `visible: true` allocates a fresh wl_surface and waits
    // for compositor configure+commit; every `visible: false`
    // destroys it. This is the heavy operation that froze Pedro's
    // compositor on every menu open.
    //
    // Strategy:
    //   1. `visible: true` PERMANENTLY at construction (defeats
    //      the destroy/recreate cycle).
    //   2. Off-screen "parking" via `margins.top: -10000` when
    //      the menu is logically closed → user sees nothing, the
    //      surface is mapped but invisible, no compositor work
    //      beyond a tiny 1×1 transparent buffer.
    //   3. `_isOpen` boolean drives positioning + size; when
    //      true, surface jumps to its computed position with the
    //      computed size.
    //   4. `_recalcWidth` runs SYNCHRONOUSLY before `_isOpen`
    //      transitions to true so the surface configures with its
    //      final size in one round-trip — avoids the resize re-
    //      commit codex flagged.
    anchors.top: true
    anchors.left: true
    visible: true              // ALWAYS — defeats deleteOnInvisible
    color: "transparent"

    /// Logical open/close — controls geometry; surface stays mapped.
    property bool _isOpen: false
    /// Surface position in screen coords (when _isOpen).
    property real _surfaceX: 0
    property real _surfaceY: 0
    /// Off-screen parking position (when !_isOpen). Negative
    /// margins place the 1×1 surface beyond the screen edge so it
    /// neither paints nor catches input.
    readonly property int _parkOffset: -10000

    implicitWidth: _isOpen ? menuBox.width : 1
    implicitHeight: _isOpen ? menuBox.height : 1
    margins.top: _isOpen ? _surfaceY : _parkOffset
    margins.left: _isOpen ? _surfaceX : _parkOffset

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
            console.log("[appmenu] cross-screen open refused:",
                        "popup-screen=", root.screen.name,
                        "focused-screen=", root.focusedScreenName);
            return;
        }
        anchorItem = item;
        menuItem = menuTree;
        // SYNC width recalc BEFORE opening — the surface configures
        // with its final size in one round-trip (codex review fix).
        root._recalcWidth();
        try {
            const p = item.mapToGlobal(0, 0);
            root._surfaceX = Math.max(0, p.x);
            root._surfaceY = Math.max(0, p.y + item.height);
        } catch (e) {
            root._surfaceX = 0;
            root._surfaceY = 0;
        }
        root._isOpen = true;
    }

    function close() {
        if (submenu.isOpen) submenu.close();
        root._isOpen = false;
        // Surface stays mapped — geometry parks off-screen.
    }

    /// External "is the menu currently open" query — used by
    /// `BarWidget`'s close-on-press hook to dismiss before
    /// processing a new bar click.
    readonly property alias isOpen: root._isOpen
    /// Compatibility — BarWidget previously checked `popup.visible`.
    /// Now visible is always true; expose a derived alias so
    /// existing callers keep working without breaking the API.
    function visibleForLogic() { return root._isOpen; }

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
    // SYNCHRONOUS recalc — width must be final BEFORE _isOpen flips
    // true so the layer-shell surface configures with its final
    // dimensions in one round-trip (codex review v1.0.4 fix). The
    // 13× layout pass is on the click hot-path but bounded ≤ 50ms
    // for a typical menu, well under the deleteOnInvisible+configure
    // round-trip cost it replaces.
    onMenuItemChanged: _recalcWidth()

    // ── Menu rectangle ──────────────────────────────────────────────
    // The PanelWindow itself is now sized to menuBox, so menuBox
    // simply fills the surface starting at (0,0). Outside-click is
    // BarWidget's job (it closes the popup before processing its
    // own press).
    Rectangle {
        id: menuBox
        visible: root._isOpen && !!root.menuItem
        x: 0
        y: 0
        width: Math.max(180, root._calcWidth)
        height: popupCol.implicitHeight + Style.marginM * 2

        // v1.0.2 visual contrast bump preserved.
        color: Color.mSurfaceVariant !== undefined
               ? Color.mSurfaceVariant
               : Color.mSurface
        border.color: Color.mPrimary !== undefined
                      ? Color.mPrimary
                      : Color.mOutline
        border.width: 2
        radius: Style.marginS

        // Swallow clicks on the menu background so the bar's
        // close-on-press hook (BarWidget) doesn't fire when the user
        // clicks an empty pixel between rows of the open popup.
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            hoverEnabled: false
            onClicked: {
                // Swallow — outside-click handled by BarWidget hook.
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
                        if (submenu.isOpen) submenu.close();
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
