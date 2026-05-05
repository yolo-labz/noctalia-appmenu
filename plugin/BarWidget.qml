// noctalia-appmenu — AppMenu bar widget (v0.1.6+: bar-widget API contract)
//
// Reads `~/.cache/noctalia-appmenu/active.json` (written by
// noctalia-appmenu-bridge on every focus change) and renders the
// focused application's name in the topbar.
//
// **v0.1 LIMITATION**: app-name only. Full menu rendering deferred
// to v0.2 — see ADR-0015 + spec 002 (bridge DBusMenu mirror).
//
// Why FileView, not D-Bus: the upstream Quickshell QML API does not
// expose a public `DBusObject` consumer for arbitrary services
// (verified against v0.2.1 type listing). A small JSON file written
// by the bridge sidesteps the missing primitive without forking
// Quickshell. v0.2's mirror lands a `DBusMenuHandle` at a fixed
// address, at which point this widget switches back to D-Bus
// directly.
//
// **Bar-widget API contract** (ADR-0018): noctalia-shell's BarSection
// instantiates plugin widgets with these properties injected — the
// widget MUST declare them or QML errors with
// `Cannot assign to non-existent property "widgetId"` on every load
// AND the widget never lays out because the pill-positioning logic
// reads them. v0.1.0..v0.1.5 omitted them. Reference contract:
// noctalia-shell `Modules/Bar/Widgets/KeepAwake.qml`.

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Services.UI

Item {
    id: root

    // ── Bar-widget API contract (injected by BarSection.qml) ────────
    // Required by the layout engine; do NOT remove.
    property ShellScreen screen
    property string widgetId: ""
    property string section: ""
    property int sectionWidgetIndex: -1
    property int sectionWidgetsCount: 0
    property var pluginApi: null

    // Per-instance widget settings come from the user's
    // Settings.data.bar.widgets.<section>[index] entry. Pulled the
    // same way KeepAwake.qml pulls them.
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

    // Widget settings — pull from per-instance widgetSettings with
    // sensible fallbacks. User customises via the bar.widgets entry.
    readonly property string fallbackText: widgetSettings.fallbackText !== undefined ? widgetSettings.fallbackText : ""
    readonly property int maxLabelWidth: widgetSettings.maxLabelWidth !== undefined ? widgetSettings.maxLabelWidth : 200
    readonly property bool showOnlyWhenFocused: widgetSettings.showOnlyWhenFocused !== undefined ? widgetSettings.showOnlyWhenFocused : true

    // ── Derived state from bridge JSON file ─────────────────────────
    property string appId: ""
    property string title: ""
    property string menuService: ""
    property string menuPath: ""

    // The bridge's JSON file. Populated by the active proxy task in
    // bridge/src/proxy.rs. Path resolution mirrors the bridge's:
    // $XDG_CACHE_HOME/noctalia-appmenu/active.json then
    // $HOME/.cache/noctalia-appmenu/active.json.
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

        onFileChanged: reload()
        onLoaded: {
            if (text.length === 0) {
                root.appId = ""
                root.title = ""
                return
            }
            try {
                const j = JSON.parse(text);
                root.appId = j.app_id || ""
                root.title = j.title || ""
                root.menuService = j.menu_service || ""
                root.menuPath = j.menu_path || ""
            } catch (e) {
                // Partial-write or empty file. Ignore until next change.
            }
        }
    }

    visible: appId !== "" || fallbackText !== ""
    implicitHeight: Style.barHeight
    implicitWidth: label.implicitWidth + Style.marginM * 2

    Text {
        id: label
        anchors.verticalCenter: parent.verticalCenter
        anchors.horizontalCenter: parent.horizontalCenter
        text: root.appId !== "" ? root.appId : root.fallbackText
        // Theme integration via noctalia tokens — Color.mOnSurface
        // tracks the active color scheme; switching to "Wallpaper" or
        // a different predefinedScheme reflows the widget instantly.
        color: Color.mOnSurface
        font.family: Settings.data.ui.fontDefault || "Inter"
        // Match noctalia's bar text sizing: Style._barBaseFontSize *
        // fontScale handles both the density (capsuleHeight) and the
        // fontScale multiplier in user settings.
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
        elide: Text.ElideRight
        Layout.maximumWidth: root.maxLabelWidth
    }
}
