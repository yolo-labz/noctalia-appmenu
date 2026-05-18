// noctalia-appmenu — outside-click shield (v1.0.14, defence in depth)
//
// History
// -------
// v1.0.9..v1.0.11 shipped a separate AppmenuShield panel, but the
// shield's input region toggling (visible-toggle race, anchor-toggle
// reconfigure, mask Region binding race) never reliably caught the
// user's first outside click on niri. v1.0.12 dropped it in favour of
// `Quickshell.PopupWindow` + `xdg_popup.grab`. v1.0.13 added the
// missing `grabFocus: true` after discovering Quickshell defaults the
// flag to FALSE — but per Pedro field report 17/05/2026 the popup
// STILL persists, suggesting Qt's `setFlag(Qt::Popup)` does not
// actually issue `xdg_popup.grab(wl_seat)` when the transient parent
// is a wlr-layer-shell surface (the bar is layer-shell, not xdg-shell).
//
// v1.0.14 strategy — belt and braces:
//   1. Keep `grabFocus: true` on the popups. When the grab DOES work
//      (other compositors, future Qt fixes), it dismisses cleanly.
//   2. Add this shield back as a SECOND dismissal path. The shield is
//      a separate full-screen wlr-layer-shell PanelWindow on
//      `WlrLayer.Top`; the popup sits on `WlrLayer.Overlay` (one
//      layer above), so popup clicks always reach the popup and
//      everything-else clicks reach the shield's MouseArea →
//      `popup.close()`.
//
// What this version DOES differently from v1.0.9
// ----------------------------------------------
// The v1.0.9 shield never fired. Three roots:
//   (a) popup was on Top alongside shield → same-layer wlr order is
//       implementation-defined on niri, shield sometimes ate popup
//       clicks. → Fixed by promoting popup to Overlay (v1.0.10+).
//   (b) visible-toggle reconfigured the surface; first click landed
//       on the underlying XDG_TOPLEVEL before configure finished.
//       → Fixed by keeping visible:true permanently and using anchor
//       toggle (v1.0.10), then mask Region (v1.0.11) — both broke for
//       different reasons (Qt::WindowTransparentForInput trap at
//       Quickshell `proxywindow.cpp:672` when the mask resolves
//       empty, and the property-binding-not-triggering-mask-update
//       gotcha).
//   (c) v1.0.14 uses the noctalia-shell `PopupMenuWindow.qml` exact
//       pattern: surface ALWAYS full-screen anchored, NO mask, plain
//       `visible: popup.isOpen` toggle. The unmap/remap cost is fine
//       — by the time the user moves to click outside the popup
//       (≥ 100 ms after open), the shield surface has had its
//       configure round-trip and is mapped + ready.
//
// Trade-off accepted: while the popup is open the shield's wl_surface
// is mapped full-screen with no input mask — it consumes all clicks
// in its visible region. Clicks in the popup's bounding rect pass
// through to the popup because the popup is on a higher layer
// (Overlay > Top). Clicks elsewhere hit the shield → close.

import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: shield

    /// The popup to dismiss when an outside click lands. Wired from
    /// BarWidget.qml.
    required property var popup
    /// Optional cascading submenu; closed alongside the popup.
    property var submenu: null

    visible: popup ? popup.isOpen : false
    color: "transparent"

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    // v1.0.15 — promoted to `WlrLayer.Overlay` (was Top). v1.0.14 put
    // the shield on Top, but noctalia-shell's `MainScreen.qml` is
    // ALREADY a full-screen `WlrLayer.Top` PanelWindow with an input
    // mask covering "everywhere except the bar" (it's the shell's
    // own click-through host for Calendar / Control Center etc).
    // Within the Top layer niri orders by creation time AND MainScreen
    // was created at shell startup — long before our plugin loaded —
    // so MainScreen sat above our shield and consumed every outside
    // click before our MouseArea could see it. Promoting the shield
    // to Overlay puts it above MainScreen unambiguously.
    //
    // The popup is ALSO on Overlay. Within Overlay, niri orders by
    // creation time again — we declare AppmenuShield in BarWidget.qml
    // BEFORE the AppmenuPopupWindow so the popup stacks above the
    // shield. Clicks on the popup reach the popup; clicks anywhere
    // else hit the shield → `popup.close()`.
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-shield-" + (screen ? screen.name : "unknown")

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        hoverEnabled: false
        onPressed: function (mouse) {
            console.log("[appmenu] shield press at",
                        Math.round(mouse.x), Math.round(mouse.y),
                        "submenu_open=", shield.submenu ? shield.submenu.isOpen : false);
            if (shield.submenu && shield.submenu.isOpen) {
                shield.submenu.close();
            }
            if (shield.popup && shield.popup.isOpen) {
                shield.popup.close();
            }
        }
    }
}
