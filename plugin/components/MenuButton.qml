// MenuButton.qml — top-level menu entry (e.g. "File", "Edit")
// Click opens a SubmenuPopup anchored below this button.

import QtQuick
import QtQuick.Controls
import Quickshell

Item {
    id: root

    // The top-level DBusMenuItem we represent
    property var item
    property int maxWidth: 200
    property int popupOffsetY: 4

    implicitHeight: 28
    implicitWidth: Math.min(maxWidth, label.implicitWidth + 16)

    Rectangle {
        id: bg
        anchors.fill: parent
        color: button.hovered || popup.visible ? "#313244" : "transparent"   // ctp-surface0
        radius: 4
    }

    AbstractButton {
        id: button
        anchors.fill: parent
        hoverEnabled: true
        text: root.item ? root.item.label : ""

        contentItem: Text {
            id: label
            text: root.item ? root.item.label : ""
            color: "#cdd6f4"      // ctp-text
            font.family: "Inter"
            font.pixelSize: 13
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
        }

        onClicked: popup.open()
    }

    SubmenuPopup {
        id: popup
        item: root.item
        anchorItem: root
        offsetY: root.popupOffsetY
    }
}
