// noctalia-appmenu — AppMenu bar widget (v0.2.0-alpha: menu strip render)
//
// Reads `~/.cache/noctalia-appmenu/active.json` (written by
// noctalia-appmenu-bridge on every focus change). v0.2 adds a `menu`
// field carrying the focused app's full DBusMenu tree:
//
//   {
//     app_id, title, menu_service, menu_path,
//     menu: {
//       id, label, type, enabled, visible, children: [
//         { id, label, "submenu", children: [...] },  // File
//         { id, label, "submenu", children: [...] },  // Edit
//         …
//       ]
//     }
//   }
//
// This widget renders the top-level `menu.children` as a horizontal
// strip of clickable menu-button items. Clicking a top-level item
// opens a Quickshell PopupWindow with that item's children as
// vertical action rows. Clicking a leaf action spawns
// `noctalia-appmenu-bridge click <busName> <menuPath> <itemId>`,
// which calls `com.canonical.dbusmenu::Event(itemId, "clicked", "",
// timestamp)` on the registered app — same effect as if the user had
// clicked the menu in-window.
//
// Falls back to the v0.1 placeholder ("·") when no app has registered
// a menu (most apps don't yet — Phase E ships QT_QPA_PLATFORMTHEME
// and GTK_MODULES env so Qt/GTK apps export DBusMenu automatically).
//
// **Bar-widget API contract** (ADR-0018), **always-visible / fixed-width
// slot** (ADR-0019/ADR-0020), **FileView text() call** (ADR-0021) all
// preserved.

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Services.UI

Item {
    id: root

    // ── Bar-widget API contract (injected by BarSection.qml) ────────
    property ShellScreen screen
    property string widgetId: ""
    property string section: ""
    property int sectionWidgetIndex: -1
    property int sectionWidgetsCount: 0
    property var pluginApi: null

    readonly property string screenName: screen ? screen.name : ""
    property var widgetSettings: {
        if (section && sectionWidgetIndex >= 0 && screenName) {
            const widgets = Settings.getBarWidgetsForScreen(screenName)[section];
            if (widgets && sectionWidgetIndex < widgets.length) {
                return widgets[sectionWidgetIndex];
            }
        }
        return {};
    }

    // Per-instance settings.
    readonly property string fallbackText: widgetSettings.fallbackText !== undefined ? widgetSettings.fallbackText : ""
    readonly property int maxLabelWidth: widgetSettings.maxLabelWidth !== undefined ? widgetSettings.maxLabelWidth : 200
    readonly property int maxStripWidth: widgetSettings.maxStripWidth !== undefined ? widgetSettings.maxStripWidth : 600

    // ── Bridge state ─────────────────────────────────────────────────
    property string appId: ""
    property string title: ""
    property string menuService: ""
    property string menuPath: ""
    /// Top-level menu items: array of {id, label, type, enabled,
    /// visible, icon_name, children: [...]}. Empty when no app
    /// registered or no menu data yet.
    property var topLevel: []
    /// Path to the bridge binary — needed to spawn click subcommand
    /// from Process. Resolved once at startup; falls back to
    /// `noctalia-appmenu-bridge` (PATH lookup) if env var not set.
    readonly property string bridgeBin: {
        const fromEnv = Quickshell.env("NOCTALIA_APPMENU_BRIDGE");
        return (fromEnv && fromEnv.length > 0) ? fromEnv : "noctalia-appmenu-bridge";
    }

    /// Display string for the v0.1 fallback path (shown only when
    /// `topLevel.length === 0`). Always non-empty so the widget
    /// claims layout (ADR-0019).
    readonly property string fallbackDisplayText: {
        if (appId !== "")
            return appId;
        if (fallbackText !== "")
            return fallbackText;
        return "·";
    }

    /// Apply a parsed bridge snapshot object (the JSON written to
    /// `active.json` and pushed via IPC) to the widget's exposed
    /// state. Pure — both the FileView (cold-start) and IpcHandler
    /// (steady-state push, PR #44) call into here so the two paths
    /// stay byte-identical.
    function applySnapshot(j) {
        if (!j) {
            root.appId = "";
            root.title = "";
            root.menuService = "";
            root.menuPath = "";
            root.topLevel = [];
            return;
        }
        root.appId = j.app_id || "";
        root.title = j.title || "";
        root.menuService = j.menu_service || "";
        root.menuPath = j.menu_path || "";
        // Walk into menu.children. Defaults to empty when bridge
        // wrote `menu: null` (e.g. focused app has no menu and no
        // synthetic fallback applied).
        root.topLevel = (j.menu && j.menu.children) ? j.menu.children : [];
    }

    /// Push channel (PR #44 — replaces FileView as the steady-state
    /// path). Bridge invokes `qs ipc call appmenu update <json>` on
    /// every focus change; the IpcHandler unwraps the JSON string and
    /// delegates to `applySnapshot`. This eliminates the inotify
    /// debounce + atomic-rename race that caused Pedro to repeatedly
    /// screenshot "nothing here" — the widget now wakes up the
    /// instant the bridge has data, no filesystem watch in between.
    ///
    /// FileView (below) is retained for cold-start: when quickshell
    /// starts before the bridge has fired its first push, the widget
    /// reads `active.json` and renders whatever the previous bridge
    /// run left there.
    IpcHandler {
        target: "appmenu"
        // Quickshell's IpcHandler requires explicitly-typed
        // parameters: untyped `function update(json)` is rejected
        // at registration time with "Type of argument 1 (json:
        // QVariant) cannot be used across IPC" because the IPC
        // bridge cannot transit QVariant. Typing the parameter as
        // `string` matches what `qs ipc call appmenu update <body>`
        // sends (the bridge passes the JSON-encoded body verbatim).
        function update(json: string) {
            try {
                const j = JSON.parse(json);
                root.applySnapshot(j);
            } catch (e) {
                // Bridge sent a malformed payload — drop the update
                // rather than corrupt widget state. Steady-state
                // bridge writes are well-formed; this is a defensive
                // guard for future protocol drift.
            }
        }
    }

    FileView {
        id: activeFile
        path: {
            const xdg = Quickshell.env("XDG_CACHE_HOME");
            const home = Quickshell.env("HOME");
            const base = xdg && xdg.length > 0
                ? xdg
                : (home + "/.cache");
            return base + "/noctalia-appmenu/active.json";
        }
        watchChanges: true
        blockLoading: false
        printErrors: false

        onFileChanged: reload()
        onLoaded: {
            // FileView's `text` is a FUNCTION call (ADR-0021).
            const content = text();
            if (!content || content.length === 0) {
                root.applySnapshot(null);
                return;
            }
            try {
                const j = JSON.parse(content);
                root.applySnapshot(j);
            } catch (e) {
                // Partial-write; ignore until next change.
            }
        }
    }

    // ── Layout ───────────────────────────────────────────────────────
    // Slot reserves up to `maxStripWidth` (default 600). Strip grows
    // up to that limit then elides items past it (clipped by the
    // RowLayout). Fallback path uses the smaller `maxLabelWidth`
    // budget per ADR-0020.
    implicitHeight: Style.barHeight
    implicitWidth: topLevel.length > 0
        ? Math.min(maxStripWidth, strip.implicitWidth + Style.marginM * 2)
        : maxLabelWidth + Style.marginM * 2

    opacity: (topLevel.length > 0 || appId !== "" || fallbackText !== "") ? 1.0 : 0.45

    // ── v0.1 fallback: app-id text label ────────────────────────────
    // Shown when no app has registered a DBusMenu (most apps until
    // Phase E ships the env vars). Identical to v0.1.9 behavior.
    Text {
        id: fallbackLabel
        visible: root.topLevel.length === 0
        anchors.fill: parent
        anchors.leftMargin: Style.marginM
        anchors.rightMargin: Style.marginM
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignLeft
        text: root.fallbackDisplayText
        color: Color.mOnSurface
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
        elide: Text.ElideRight
    }

    // ── v0.2 menu strip: horizontal row of top-level menu buttons ──
    Row {
        id: strip
        visible: root.topLevel.length > 0
        anchors.left: parent.left
        anchors.leftMargin: Style.marginS
        anchors.verticalCenter: parent.verticalCenter
        spacing: Style.marginS

        Repeater {
            model: root.topLevel
            delegate: Rectangle {
                id: btn
                required property var modelData

                // Skip separators in the top-level strip — they're
                // for submenus, not menubar root.
                visible: modelData && modelData.type !== "separator" &&
                         (modelData.visible !== false)

                color: hover.containsMouse
                    ? Color.mSurfaceVariant
                    : "transparent"
                radius: Style.marginXS !== undefined ? Style.marginXS : 4
                border.width: 0

                implicitHeight: Style.barHeight - Style.marginS * 2
                implicitWidth: btnLabel.implicitWidth + Style.marginM * 2

                Text {
                    id: btnLabel
                    anchors.centerIn: parent
                    // Strip leading underscore (accelerator marker).
                    text: (modelData ? modelData.label : "").replace(/_/g, "")
                    color: btn.modelData && btn.modelData.enabled === false
                        ? Color.mOnSurfaceVariant
                        : Color.mOnSurface
                    font.family: Settings.data.ui.fontDefault || "Inter"
                    font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                }

                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    enabled: btn.modelData && btn.modelData.enabled !== false
                    onClicked: {
                        if (!btn.modelData) return;
                        if (btn.modelData.children && btn.modelData.children.length > 0) {
                            popup.menuItem = btn.modelData;
                            popup.anchorItem = btn;
                            popup.visible = true;
                        } else {
                            // Leaf at top level — fire click directly.
                            root.fireClick(btn.modelData);
                        }
                    }
                }
            }
        }
    }

    // ── Submenu popup ────────────────────────────────────────────────
    PopupWindow {
        id: popup
        property var menuItem: null
        property var anchorItem: null

        // Anchor below the clicked top-level button. Quickshell's
        // PopupWindow positions relative to a parent surface +
        // anchor.rect; we use the Bar's screen as parent and compute
        // a bar-relative rect from the anchor item.
        anchor.window: root.QsWindow.window
        anchor.rect: {
            if (!anchorItem) return Qt.rect(0, 0, 0, 0);
            const p = anchorItem.mapToItem(null, 0, anchorItem.height);
            return Qt.rect(p.x, p.y, anchorItem.width, 1);
        }
        anchor.edges: Edges.Bottom
        anchor.gravity: Edges.Bottom

        visible: false
        implicitWidth: Math.max(180, popupCol.implicitWidth + Style.marginM * 2)
        implicitHeight: popupCol.implicitHeight + Style.marginM * 2

        // Click outside dismisses the popup.
        MouseArea {
            anchors.fill: parent
            onClicked: popup.visible = false
            propagateComposedEvents: true
        }

        Rectangle {
            anchors.fill: parent
            color: Color.mSurface
            border.color: Color.mOutline
            border.width: 1
            radius: Style.marginS

            Column {
                id: popupCol
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Style.marginXS !== undefined ? Style.marginXS : 4
                spacing: 0

                Repeater {
                    model: popup.menuItem ? (popup.menuItem.children || []) : []
                    delegate: Item {
                        id: item
                        required property var modelData
                        readonly property bool isSeparator: modelData && modelData.type === "separator"
                        readonly property bool isVisible: !modelData || modelData.visible !== false
                        visible: isVisible
                        width: parent ? parent.width : 0
                        height: isSeparator ? Style.marginXS * 2 : (Style.barHeight - Style.marginS)

                        // Separator
                        Rectangle {
                            visible: item.isSeparator
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: Style.marginS
                            anchors.rightMargin: Style.marginS
                            height: 1
                            color: Color.mOutline
                            opacity: 0.4
                        }

                        // Action / submenu item
                        Rectangle {
                            visible: !item.isSeparator
                            anchors.fill: parent
                            color: itemHover.containsMouse
                                ? Color.mSurfaceVariant
                                : "transparent"
                            radius: Style.marginXS !== undefined ? Style.marginXS : 4

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Style.marginS
                                anchors.rightMargin: Style.marginS
                                spacing: Style.marginS

                                Text {
                                    Layout.fillWidth: true
                                    text: (item.modelData ? item.modelData.label : "").replace(/_/g, "")
                                    color: item.modelData && item.modelData.enabled === false
                                        ? Color.mOnSurfaceVariant
                                        : Color.mOnSurface
                                    font.family: Settings.data.ui.fontDefault || "Inter"
                                    font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                }

                                // Submenu indicator
                                Text {
                                    visible: item.modelData && item.modelData.children && item.modelData.children.length > 0
                                    text: "›"
                                    color: Color.mOnSurfaceVariant
                                    font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                                }
                            }

                            MouseArea {
                                id: itemHover
                                anchors.fill: parent
                                hoverEnabled: true
                                enabled: item.modelData && item.modelData.enabled !== false
                                onClicked: {
                                    if (!item.modelData) return;
                                    // v0.3.0-alpha: only fire on leaf
                                    // items. Nested submenus deferred —
                                    // the popup-of-popup work belongs
                                    // with the broader v0.3.x QML pass.
                                    const hasChildren = item.modelData.children && item.modelData.children.length > 0;
                                    if (!hasChildren) {
                                        root.fireClick(item.modelData);
                                        popup.visible = false;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Click forwarding ──────────────────────────────────────────────
    // Spawn `noctalia-appmenu-bridge atspi-click <service> <path>` as
    // a one-shot child. The bridge subcommand calls
    // `org.a11y.atspi.Action.DoAction(0)` on the AT-SPI accessible —
    // qtatspi convention is action 0 = "click", same effect as
    // clicking the item in-window.
    //
    // `item` is the menu-tree node from active.json — it carries
    // `service` (a11y bus name) and `path` (a11y object path), which
    // together address one accessible. v0.2's `(menuService,
    // menuPath, itemId)` tuple is gone with the DBusMenu retirement.
    function fireClick(item) {
        if (!item || !item.service || !item.path) {
            return;
        }
        clickProcess.command = [
            root.bridgeBin,
            "atspi-click",
            item.service,
            item.path
        ];
        clickProcess.running = true;
    }

    Process {
        id: clickProcess
        // command set per-call in fireClick(). running: false default.
    }
}
