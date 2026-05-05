// noctalia-appmenu — AppMenu bar widget (v0.1.3+: FileView-based)
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

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: root

    // Per-instance settings — read from Settings.data.bar.widgets entry
    property string fallbackText: ""
    property int maxLabelWidth: 200
    property bool showOnlyWhenFocused: true

    // Derived state from bridge's JSON file. Updated by FileView's
    // automatic file-watch (inotify under the hood).
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
    implicitHeight: parent ? parent.height : 28
    implicitWidth: label.implicitWidth + 16

    Text {
        id: label
        anchors.verticalCenter: parent.verticalCenter
        anchors.horizontalCenter: parent.horizontalCenter
        text: root.appId !== "" ? root.appId : root.fallbackText
        color: "#cdd6f4"        // ctp-text — TODO: noctalia theme tokens
        font.family: "Inter"
        font.pixelSize: 13
        elide: Text.ElideRight
        Layout.maximumWidth: root.maxLabelWidth
    }
}
