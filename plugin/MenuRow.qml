// noctalia-appmenu — shared menu row delegate
//
// Used by both AppmenuPopupWindow (top-level popup) and SubmenuPopup
// (nested popups). Renders one menu entry as either:
//
//   • a separator (thin horizontal rule), or
//   • an actionable row:
//       [ icon? ] [ toggle_state slot? ] [ label ]  …  [ › chevron? ]
//
// FR-011 (spec 004) — toggle_state. When `modelData.toggle_type` is
// `"checkmark"` and `toggle_state` is truthy, the indicator slot shows
// `✓`; when `toggle_state` is false, the slot stays reserved but blank
// so neighbouring rows align consistently. Rows with
// `toggle_type == null` reserve no slot.
//
// FR-012 (spec 004) — icon_name. When `modelData.icon_name` is a
// non-empty string, an Image is shown to the left of the label,
// resolved via Quickshell's icon-theme lookup (`Quickshell.iconPath`).
// When empty, the Image takes no horizontal space.
//
// Spec 003 FR-008/FR-010 — the `onClicked` handler is wrapped in a
// try/catch envelope; a single broken delegate cannot poison sibling
// rows or the parent popup's IPC dispatcher.

import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.Commons

Item {
    id: row

    /// The MenuItem object from active.json — shape per spec 004
    /// contracts/active-json-schema.md §MenuItem.
    required property var modelData

    /// Emitted when the user clicks a leaf row (no `children`).
    signal clicked(var item)

    /// Emitted when the user clicks a row with `children`. The parent
    /// popup consumes this and opens a SubmenuPopup anchored to
    /// `anchorItem` — the row Item itself. The compositor walks the
    /// xdg_popup parent chain (bar button → top-level popup → this
    /// row → submenu) so a grab held at the root extends across the
    /// entire cascade.
    signal submenuRequested(var item, Item anchorItem)

    readonly property bool isSeparator: modelData && modelData.item_type === "separator"
                                        || (modelData && modelData.type === "separator")
    readonly property bool isVisible: !modelData || modelData.visible !== false
    readonly property bool isEnabled: modelData && modelData.enabled !== false
    readonly property bool hasChildren: modelData && modelData.children
                                        && modelData.children.length > 0
    readonly property string iconName: modelData && modelData.icon_name
                                       ? modelData.icon_name : ""
    readonly property string toggleType: modelData && modelData.toggle_type
                                         ? modelData.toggle_type : ""
    readonly property bool toggleOn: modelData && modelData.toggle_state === true

    visible: isVisible
    width: parent ? parent.width : 0
    height: isSeparator
            ? Style.marginXS * 2
            // v1.0.12 — give rows more breathing room. The previous
            // height (Style.barHeight - Style.marginS) cramped the
            // label to the bar's tight strip; menus are reading
            // surfaces and benefit from extra vertical padding.
            : Math.max(28, Style.barHeight - Style.marginXS)

    // Theme-token spacing fallback for older noctalia (matches
    // AppmenuPopupWindow.qml:205 defensive pattern).
    readonly property int _xs: Style.marginXS !== undefined ? Style.marginXS : 4

    // ── Separator ─────────────────────────────────────────────────────
    Rectangle {
        visible: row.isSeparator
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Style.marginS
        anchors.rightMargin: Style.marginS
        height: 1
        color: Color.mOutline
        opacity: 0.4
    }

    // ── Action / submenu row ─────────────────────────────────────────
    Rectangle {
        id: rowBg
        visible: !row.isSeparator
        anchors.fill: parent
        // v1.0.12 — hover uses the primary-tinted surface (matches
        // CalendarHeaderCard's hover treatment) instead of the plain
        // surface variant. More legible against the menuBox bg.
        color: !row.isEnabled
               ? "transparent"
               : (rowHover.containsMouse ? Color.mSurfaceVariant : "transparent")
        radius: Style.radiusS !== undefined ? Style.radiusS : row._xs

        // Subtle vertical accent stripe on the left when hovered —
        // mirrors the focused-row treatment in noctalia's Launcher
        // entries.
        Rectangle {
            visible: rowHover.containsMouse && row.isEnabled
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.topMargin: row._xs
            anchors.bottomMargin: row._xs
            width: 2
            radius: 1
            color: Color.mPrimary
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Style.marginS
            anchors.rightMargin: Style.marginS
            spacing: Style.marginS

            // FR-011 — toggle_state indicator slot. Reserved when
            // toggle_type is non-empty; blank when state is false.
            Text {
                id: toggleIndicator
                visible: row.toggleType.length > 0
                Layout.preferredWidth: visible
                                       ? Math.max(1, Style._barBaseFontSize
                                                  * (Settings.data.bar.fontScale || 1.0))
                                       : 0
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                text: row.toggleType === "checkmark"
                      ? (row.toggleOn ? "\u2713" : "")
                      : (row.toggleType === "radio"
                         ? (row.toggleOn ? "\u2022" : "")
                         : "")
                color: row.isEnabled ? Color.mOnSurface : Color.mOnSurfaceVariant
                font.family: Settings.data.ui.fontDefault || "Inter"
                font.pixelSize: Math.max(1, Style._barBaseFontSize
                                         * (Settings.data.bar.fontScale || 1.0))
            }

            // FR-012 — icon_name. Quickshell.iconPath returns either a
            // `image://icon/<name>` URL or the fallback empty string;
            // we wrap the lookup defensively so an icon-theme miss
            // doesn't trip a binding error.
            Image {
                id: iconImage
                visible: row.iconName.length > 0 && source.toString().length > 0
                Layout.preferredWidth: visible
                                       ? Math.max(1, Style._barBaseFontSize
                                                  * (Settings.data.bar.fontScale || 1.0))
                                       : 0
                Layout.preferredHeight: Layout.preferredWidth
                fillMode: Image.PreserveAspectFit
                smooth: true
                source: {
                    if (row.iconName.length === 0) return "";
                    try {
                        if (typeof Quickshell !== "undefined"
                            && Quickshell.iconPath) {
                            return Quickshell.iconPath(row.iconName, "");
                        }
                    } catch (_) {
                        // fall through to the URL-scheme fallback
                    }
                    return "image://icon/" + row.iconName;
                }
            }

            Text {
                Layout.fillWidth: true
                text: (row.modelData ? row.modelData.label : "").replace(/_/g, "")
                color: row.isEnabled ? Color.mOnSurface : Color.mOnSurfaceVariant
                font.family: Settings.data.ui.fontDefault || "Inter"
                font.pixelSize: Math.max(1, Style._barBaseFontSize
                                         * (Settings.data.bar.fontScale || 1.0))
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }

            // Submenu chevron
            Text {
                visible: row.hasChildren
                text: "\u203A"
                color: Color.mOnSurfaceVariant
                font.family: Settings.data.ui.fontDefault || "Inter"
                font.pixelSize: Math.max(1, Style._barBaseFontSize
                                         * (Settings.data.bar.fontScale || 1.0))
            }
        }

        MouseArea {
            id: rowHover
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton
            enabled: row.isEnabled
            onClicked: {
                // Spec 003 FR-010 — per-delegate envelope. A throw in
                // one row's click handler cannot block siblings from
                // rendering or activating.
                try {
                    if (!row.modelData) return;
                    if (row.hasChildren) {
                        // v1.0.12 — submenu is now a PopupWindow
                        // anchored to this row Item (xdg_popup parent
                        // chain). No screen-absolute math needed; the
                        // compositor resolves position relative to the
                        // anchor item's window.
                        row.submenuRequested(row.modelData, row);
                    } else {
                        row.clicked(row.modelData);
                    }
                } catch (e) {
                    console.error("[appmenu/row] envelope caught:", e,
                                  "stack:", (e && e.stack) || "(no stack)");
                }
            }
        }
    }
}
