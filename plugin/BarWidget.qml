// noctalia-appmenu — AppMenu bar widget (v0.1: fallback-only)
//
// Subscribes to org.noctalia.AppMenu /org/noctalia/AppMenu/Active
// (published by noctalia-appmenu-bridge) and renders the focused
// app's name in the topbar.
//
// **v0.1 LIMITATION**: this widget renders only the focused app's
// name (from the registrar's published `appId`). Full menu-tree
// rendering is blocked on Quickshell's DBusMenuHandle being
// QML_UNCREATABLE — see ADR-0015 for the v0.2 mirror plan.
//
// Once spec 002 lands the bridge will also implement
// `com.canonical.dbusmenu` server-side at a fixed path and a
// public DBusMenuHandle factory will let us bind it from QML.
// At that point we replace the Text fallback below with a Repeater
// over `handle.menu.children`.

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Item {
    id: root

    // Per-instance settings — read from Settings.data.bar.widgets entry
    property string fallbackText: ""
    property int maxLabelWidth: 200
    property bool showOnlyWhenFocused: true

    // The bridge advertises (busName, objectPath, appId, title) as
    // properties on org.noctalia.AppMenu /org/noctalia/AppMenu/Active.
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

    visible: activeProxy.appId !== "" || root.fallbackText !== ""

    implicitHeight: parent ? parent.height : 28
    implicitWidth: label.implicitWidth + 16

    Text {
        id: label
        anchors.verticalCenter: parent.verticalCenter
        anchors.horizontalCenter: parent.horizontalCenter
        text: activeProxy.appId !== "" ? activeProxy.appId : root.fallbackText
        color: "#cdd6f4"        // ctp-text — to migrate to noctalia theme tokens once available
        font.family: "Inter"
        font.pixelSize: 13
        elide: Text.ElideRight
        Layout.maximumWidth: root.maxLabelWidth
    }
}
