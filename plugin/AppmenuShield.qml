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
// v1.0.15 strategy — belt and braces:
//   1. Keep `grabFocus: true` on the popups. When the grab DOES work
//      (other compositors, future Qt fixes), it dismisses cleanly.
//   2. Add this shield back as a SECOND dismissal path. The shield is
//      a separate full-screen wlr-layer-shell PanelWindow on
//      `WlrLayer.Overlay` with an explicit input mask. The mask excludes
//      the bar strip and the popup rectangle, so popup clicks and menubar
//      swaps pass through while everything else reaches the shield's
//      MouseArea → `popup.close()`.
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
//   (c) v1.0.15 keeps the shield on Overlay and gives it an explicit
//       non-empty mask: full output minus the bar strip and popup rect.
//       This removes same-layer ordering ambiguity while preserving
//       submenu clicks and top-level menubar swaps.
//
// Trade-off accepted: while the popup is open the shield consumes
// non-popup, non-bar clicks on the output. The bar and popup regions
// remain pass-through.

import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons

PanelWindow {
    id: shield

    /// The popup to dismiss when an outside click lands. Wired from
    /// BarWidget.qml.
    required property var popup
    /// Optional cascading submenu; closed alongside the popup.
    property var submenu: null

    readonly property Item popupAnchor: popup ? popup.anchorItem : null
    readonly property point popupTopLeftGlobal: popupAnchor
        ? popupAnchor.mapToGlobal(0, popupAnchor.height)
        : Qt.point(0, 0)
    readonly property point shieldTopLeftGlobal: contentItem
        ? contentItem.mapToGlobal(0, 0)
        : Qt.point(0, 0)
    readonly property int popupMargin: 8
    readonly property int popupX: Math.round(popupTopLeftGlobal.x - shieldTopLeftGlobal.x)
    readonly property int popupY: Math.round(popupTopLeftGlobal.y - shieldTopLeftGlobal.y)
    readonly property int popupW: popup ? Math.ceil(popup.implicitWidth) : 0
    readonly property int popupH: popup ? Math.ceil(popup.implicitHeight) : 0
    readonly property int barPassThroughHeight: Math.max(1, Style.barHeight || 1)

    visible: popup ? popup.isOpen : false
    color: "transparent"

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    WlrLayershell.exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "noctalia-appmenu-shield-" + (screen ? screen.name : "unknown")

    mask: Region {
        intersection: Intersection.Subtract

        Region {
            x: 0
            y: 0
            width: shield.width
            height: shield.barPassThroughHeight
        }

        Region {
            x: shield.popupX - shield.popupMargin
            y: shield.popupY - shield.popupMargin
            width: shield.popupW + 2 * shield.popupMargin
            height: shield.popupH + 2 * shield.popupMargin
        }
    }

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
