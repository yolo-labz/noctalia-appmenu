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

    /// Whether the shield should logically intercept outside clicks.
    readonly property bool _shouldShow: (popup && popup.isOpen) || (submenu && submenu.isOpen)

    visible: true              // ALWAYS — defeats `deleteOnInvisible`
    color: "transparent"

    // v1.0.11 — surface is permanently anchored to all four edges
    // below the bar strip. Geometry NEVER changes (no anchor toggle,
    // no implicit-size toggle). The previous "park off-screen" trick
    // (v1.0.10) flipped two anchors on every open/close which still
    // triggered wlr-layer-shell reconfigure cycles — niri lost the
    // mapped surface in the transition and the user's first outside
    // click landed on the underlying XDG_TOPLEVEL (Firefox).
    //
    // What toggles instead: the surface's INPUT REGION via `mask`.
    // `mask: Region {}` (empty) = surface accepts no clicks
    // (pattern borrowed from noctalia-shell's `BarExclusionZone.qml`).
    // When `_shouldShow` is true we expand the Region to the full
    // surface; when false we collapse to 0×0. The wl_surface stays
    // committed at the same geometry the whole time — Wayland just
    // updates the input region attribute in place.
    anchors.top: true
    anchors.left: true
    anchors.right: true
    anchors.bottom: true
    margins.top: barHeight

    // v1.0.11 — input region toggles via mask; surface always mapped
    // at the same geometry so clicks don't race surface reconfigure.
    mask: Region {
        width: shield._shouldShow ? shield.width : 0
        height: shield._shouldShow ? shield.height : 0
    }

    // v1.0.10 — popup sits on `WlrLayer.Overlay`; shield stays on
    // `WlrLayer.Top`. Popup is unambiguously above shield (Overlay >
    // Top in wlr) so clicks on the popup always reach the popup;
    // clicks anywhere else in the shield region hit the shield.
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
