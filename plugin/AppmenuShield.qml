// noctalia-appmenu — outside-click shield for popups (v1.0.9).
//
// Why this exists
// ---------------
// v1.0.4 dropped the full-screen `MouseArea` outside-click catcher
// inside `AppmenuPopupWindow.qml` because anchoring that surface to
// all four screen edges produced a 1920×1080 `wlr-layer-shell` surface
// that triggered whole-output damage every frame on AMD/niri (cited
// upstream: noctalia-shell#2216). The comment at AppmenuPopupWindow.qml
// L105-109 claimed "outside dismissal is now BarWidget's responsibility"
// but no such hook was ever written → Pedro field report 16/05/2026
// 17:10 BRT post-v1.0.8: the popup never dismisses on outside click.
//
// Design
// ------
// A separate transparent layer-shell `PanelWindow` anchored to all
// four edges (full-screen) that captures every pointer press in the
// "outside" region and calls `popup.close()` / `submenu.close()`.
//
// Z-order trick (no `mask` Region needed):
//   - Shield + popup are both on `WlrLayer.Top`.
//   - Within a single wlr layer, surfaces stack in creation order;
//     later surfaces float above earlier ones.
//   - The shield is declared BEFORE the popup in `BarWidget.qml`, so
//     the popup surface always paints (and accepts input) above the
//     shield. Pointer presses on the popup never reach the shield.
//   - The bar itself is hosted by noctalia-shell's own bar
//     `PanelWindow`, which is declared before this plugin loads → bar
//     is BELOW the shield in z-order. We carve a strip out of the
//     shield's input region for the bar's height by anchoring the
//     shield with `margins.top: barHeight` so the bar stays clickable
//     while the popup is up (clicking another menu button replaces
//     the popup contents via `BarWidget`'s existing handler).
//
// Why not `WlrLayer.Overlay` for the popup
// ----------------------------------------
// noctalia-shell's other widgets (notifications, control center, …)
// can land on `WlrLayer.Overlay`; promoting the popup there would
// stack it above unrelated UI surfaces. Keeping everything on `Top`
// preserves the existing z-order with the rest of the shell and only
// adds the shield underneath the popup.
//
// Cost
// ----
// One additional `wl_surface` per screen, mapped only while a popup is
// logically open. Transparent, no painting beyond the empty buffer
// commit. Negligible compared to the always-mapped popup surface
// itself (v1.0.4 keep-mapped pattern).

import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: shield

    /// The popup that the shield should dismiss when an outside click
    /// is observed. Wired up from `BarWidget.qml`.
    required property var popup
    /// Optional cascading submenu — closed alongside the popup so
    /// deep open menus collapse atomically.
    property var submenu: null
    /// Pixel height of the noctalia-shell bar strip we sit beneath.
    /// The shield does NOT cover the bar (so bar buttons stay
    /// clickable). Default matches the upstream noctalia-shell bar.
    property int barHeight: 32

    /// Whether the shield should logically intercept outside clicks
    /// (popup is open). The wl_surface stays MAPPED — only the input
    /// region's geometry changes — to avoid the configure/commit race
    /// every visibility toggle would trigger under Quickshell's
    /// `deleteOnInvisible: true` default (see AppmenuPopupWindow.qml
    /// L108-127 for the v1.0.4 keep-mapped pattern this mirrors).
    readonly property bool _shouldShow: (popup && popup.isOpen) || (submenu && submenu.isOpen)

    /// Off-screen parking position used when `_shouldShow` is false —
    /// the 1×1 surface stays mapped beyond the screen edge so it
    /// neither paints nor catches input but never has to reconfigure
    /// on subsequent shows. Same trick AppmenuPopupWindow uses.
    readonly property int _parkOffset: -10000

    visible: true              // ALWAYS — defeats `deleteOnInvisible`
    color: "transparent"

    // Anchors: full-screen below the bar when open, 1×1 off-screen
    // when closed. The surface configures with its final geometry in
    // one round-trip so we never pay the wl_surface recreation cost
    // mid-interaction (which is what made v1.0.9's `visible: _shouldShow`
    // race with the user's first outside-click).
    anchors.top: true
    anchors.left: true
    anchors.right: _shouldShow
    anchors.bottom: _shouldShow
    margins.top: _shouldShow ? barHeight : _parkOffset
    margins.left: _shouldShow ? 0 : _parkOffset
    implicitWidth: _shouldShow ? 0 : 1
    implicitHeight: _shouldShow ? 0 : 1

    // v1.0.10 — popup now sits on `WlrLayer.Overlay`; the shield stays
    // on `WlrLayer.Top` so the popup is unambiguously above it in the
    // wlr layer stack. We pay the price of stacking the popup above
    // notifications/control-center while a menu is open — small UX
    // hit, large correctness win (same-layer ordering is
    // implementation-defined on niri).
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-shield-" + (screen ? screen.name : "unknown")

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        enabled: shield._shouldShow
        onPressed: function (mouse) {
            console.log("[appmenu] shield press at",
                        Math.round(mouse.x), Math.round(mouse.y),
                        "submenu_open=", shield.submenu ? shield.submenu.isOpen : false,
                        "popup_open=", shield.popup ? shield.popup.isOpen : false);
            if (shield.submenu && shield.submenu.isOpen) {
                shield.submenu.close();
            }
            if (shield.popup && shield.popup.isOpen) {
                shield.popup.close();
            }
        }
    }
}
