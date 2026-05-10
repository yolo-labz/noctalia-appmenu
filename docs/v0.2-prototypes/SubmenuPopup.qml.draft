// SubmenuPopup.qml — recursive popup that renders a DBusMenuItem's
// children list and recurses for nested submenus.
//
// Per ADR-0008: uses Quickshell.PopupWindow with wlr-layer-shell so
// z-order is predictable and multi-monitor handling is the layer-shell
// library's problem, not ours.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

PopupWindow {
    id: root

    property var item                  // parent DBusMenuItem
    property var anchorItem             // QQuickItem to anchor below
    property int offsetY: 4
    property int submenuOffsetX: 2

    visible: false
    color: "transparent"

    // Layer-shell positioning — Quickshell's PopupWindow handles it.
    anchor.window: anchorItem ? anchorItem.parent.QQQ_window : null
    anchor.rect: anchorItem
                  ? Qt.rect(anchorItem.x, anchorItem.y + anchorItem.height + offsetY,
                            anchorItem.width, 0)
                  : Qt.rect(0, 0, 0, 0)

    function open() {
        if (!item || !item.children || item.children.length === 0) return;
        visible = true;
    }
    function close() { visible = false; }

    Rectangle {
        id: panel
        color: "#1e1e2e"               // ctp-base
        border.color: "#45475a"        // ctp-surface1
        border.width: 1
        radius: 6
        implicitWidth: Math.max(160, layout.implicitWidth + 16)
        implicitHeight: layout.implicitHeight + 12

        ColumnLayout {
            id: layout
            anchors.fill: parent
            anchors.margins: 6
            spacing: 0

            Repeater {
                model: root.item ? root.item.children : []
                delegate: Item {
                    id: row
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: rowText.implicitHeight + 8

                    // DBusMenuItem may be a separator
                    visible: !modelData.isSeparator

                    Rectangle {
                        anchors.fill: parent
                        color: rowMouse.containsMouse ? "#313244" : "transparent" // ctp-surface0
                        radius: 4
                    }

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 6

                        Text {
                            id: rowText
                            text: row.modelData.label
                            color: row.modelData.enabled ? "#cdd6f4" : "#7f849c" // ctp-text / ctp-overlay1
                            font.family: "Inter"
                            font.pixelSize: 12
                            verticalAlignment: Text.AlignVCenter
                            anchors.verticalCenter: parent.verticalCenter
                            Layout.fillWidth: true
                        }

                        Text {
                            visible: row.modelData.shortcut !== ""
                            text: row.modelData.shortcut
                            color: "#7f849c"    // ctp-overlay1
                            font.family: "JetBrains Mono"
                            font.pixelSize: 11
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Text {
                            visible: row.modelData.children.length > 0
                            text: "›"
                            color: "#cdd6f4"
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            if (row.modelData.children.length > 0) {
                                // Open nested submenu — recurse
                                nested.item = row.modelData;
                                nested.anchorItem = row;
                                nested.offsetX = root.submenuOffsetX;
                                nested.open();
                            } else {
                                row.modelData.activate();
                                root.close();
                            }
                        }
                    }
                }
            }
        }
    }

    SubmenuPopup {
        id: nested
        property int offsetX: 2
        // Anchored to the right of the parent row, not below it
    }

    // Dismiss on outside click
    MouseArea {
        anchors.fill: parent
        z: -1
        onClicked: root.close()
    }
}
