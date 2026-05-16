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

    /// Whether the shield should be mapped right now. Recomputed
    /// whenever the popup or submenu open/close state changes.
    readonly property bool _shouldShow: (popup && popup.isOpen) || (submenu && submenu.isOpen)

    /// Mapped only while a popup is logically open. When `_shouldShow`
    /// is false the surface is unmapped (cheap) — we accept the
    /// `deleteOnInvisible` recreation cost here because the shield
    /// has no rendering load and rebuilds in <1 ms.
    visible: _shouldShow
    color: "transparent"

    // Anchors: full-screen below the bar. Using top+bottom+left+right
    // forces the surface to screen size minus the bar strip, which is
    // exactly the "outside-popup" region we want to listen on.
    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    margins.top: barHeight

    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-shield-" + (screen ? screen.name : "unknown")

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onPressed: {
            if (shield.submenu && shield.submenu.isOpen) {
                shield.submenu.close();
            }
            if (shield.popup && shield.popup.isOpen) {
                shield.popup.close();
            }
        }
    }
}
