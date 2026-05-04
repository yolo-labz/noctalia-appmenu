// noctalia-appmenu — AppMenu bar widget
//
// Renders the focused application's menubar in noctalia's topbar.
// Subscribes to org.noctalia.AppMenu /org/noctalia/AppMenu/Active,
// which is published by noctalia-appmenu-bridge (sidecar).
//
// See ADR-0007 for why we route through the bridge's fixed proxy
// instead of binding directly to each app's DBusMenu.

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.DBusMenu
import Quickshell.Wayland
import Quickshell.Io

import "components" as Components

Item {
    id: root

    // Per-instance settings — read from Settings.data.bar.widgets entry
    property string fallbackText: ""
    property int maxLabelWidth: 200
    property bool showOnlyWhenFocused: true
    property int popupOffsetY: 4

    // The bridge advertises (busName, objectPath, appId, title) as
    // properties on org.noctalia.AppMenu /org/noctalia/AppMenu/Active.
    // The QML side never has to talk to the registrar directly.
    DBusObject {
        id: activeProxy
        bus: DBus.SessionBus
        service: "org.noctalia.AppMenu"
        objectPath: "/org/noctalia/AppMenu/Active"
        interfaceName: "org.noctalia.AppMenu.Active"

        property string busName
        property string objectPath_
        property string appId
        property string title
    }

    // The bridge re-exports the active app's menu under a fixed path.
    // We bind a DBusMenuHandle — the same primitive Quickshell uses
    // for tray menus — to that fixed (service, path) pair so QML can
    // reuse the existing rendering pipeline.
    DBusMenuHandle {
        id: handle
        // Wired up implicitly by Quickshell when SystemTrayItem-style
        // service/path properties are available; for this fixed proxy
        // we read them off activeProxy.
        // (If Quickshell's DBusMenuHandle ever exposes a public
        // create(service, path) factory — see ADR-0007 — replace
        // this binding with the direct factory call.)
    }

    visible: handle.menu !== null && handle.menu.children.length > 0

    implicitHeight: parent ? parent.height : 28
    implicitWidth: row.implicitWidth

    RowLayout {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Repeater {
            model: handle.menu ? handle.menu.children : []
            delegate: Components.MenuButton {
                required property var modelData
                item: modelData
                maxWidth: root.maxLabelWidth
                popupOffsetY: root.popupOffsetY
            }
        }
    }

    // Fallback: when there's an active toplevel but no menu, render the
    // app name from the desktop entry. This keeps the bar from "going
    // empty" when focus moves to Firefox / Electron apps. See ADR-0006.
    Text {
        id: fallback
        visible: !root.visible && activeProxy.appId !== ""
        anchors.verticalCenter: parent.verticalCenter
        text: activeProxy.appId
        color: "#cdd6f4"        // ctp-text — should be replaced with theme token
        font.family: "Inter"
        font.pixelSize: 13
        elide: Text.ElideRight
        Layout.maximumWidth: root.maxLabelWidth
    }
}
