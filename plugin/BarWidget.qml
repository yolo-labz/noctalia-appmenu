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
import Quickshell.Wayland
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
    /// niri window id of the bridge's currently-focused window. Used
    /// by `fireClick` to send `niri msg action focus-window --id <id>`
    /// before invoking `Action.DoAction(0)`, so multi-window apps
    /// (Firefox in particular) route the action to the captured
    /// window rather than to whichever window has app-internal focus
    /// at click time. See issue #109. `0` when the bridge has not
    /// yet reported a focused window or the older bridge is running.
    property int focusWinid: 0

    /// Snapshot of `focusWinid` taken at popup-open time. Read by
    /// `fireClick` (which may run *after* the user hovers over a
    /// sibling window, drifting `focusWinid`). Reset to 0 on popup
    /// close to surface bugs loudly rather than silently route to
    /// the previous window.
    property int _capturedWinid: 0

    // v1.0.9 — close any open popup the moment the focused app
    // changes. Catches alt-tab away (and the noctalia-shell focus
    // change that lands on a different `app_id`) without needing the
    // outside-click shield to react. The shield handles same-app
    // clicks (Firefox's own UI) where `appId` stays the same.
    onAppIdChanged: {
        if (popup && popup.isOpen) {
            popup.close();
        }
    }
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

    /// FR-013 (spec 004) — multi-screen popup-routing guard data
    /// source. Bound to the focused Wayland toplevel's output name
    /// via `Quickshell.Wayland.ToplevelManager`. Defensive: empty
    /// string when no toplevel-tracking data is available (single-
    /// screen host or upstream API absent), which makes the popup
    /// guard permissive (falls back to current behaviour). Lane A
    /// may later supply this from `active.json`'s `focused_output`
    /// field; the popup consumes either source via the same property.
    readonly property string focusedScreenName: {
        try {
            const tm = ToplevelManager;
            const active = tm ? tm.activeToplevel : null;
            const screens = active && active.screens ? active.screens : null;
            if (screens && screens.length > 0 && screens[0] && screens[0].name) {
                return screens[0].name;
            }
        } catch (_) {
            // Quickshell.Wayland.ToplevelManager not present — guard
            // stays permissive.
        }
        return "";
    }

    /// Whether the widget should claim any layout space at all.
    /// The widget never fabricates menus — it renders whatever the
    /// bridge writes to `active.json`, and collapses to zero width when
    /// `menu` is null. Provenance lives in the snapshot's `source`
    /// field, which the widget does not branch on:
    ///   - `source: "atspi"`           — the app's real native menubar,
    ///   - `source: "desktop-fallback"` — an identity-derived fallback
    ///     the bridge built (app `.desktop` actions + niri window
    ///     controls) for apps with no AT-SPI menubar (ADR-0031).
    ///     This SUPERSEDES the v1.0.2 honest-or-hidden Empty
    ///     posture (PR #47); the bridge no longer fakes keystroke items,
    ///     so the fallback is honest and `hasMenu` becomes true for it,
    ///   - `source: "empty"`           — no menu; the bar hides.
    ///
    /// `fallbackText` (per-instance widget setting) opts into showing
    /// a static label even when no menu is present — for users who
    /// want to claim bar real estate as a label slot. Default empty.
    readonly property bool hasMenu: topLevel.length > 0
    readonly property bool shouldRender: (hasMenu || fallbackText !== "") && !_failedState

    // ── Spec 003 isolation envelope (FR-008..010 + Swarm I) ──────────
    //
    // `_failedState` flips to true when ANY user-facing entry point
    // (applySnapshot, IpcHandler.update, FileView.onLoaded) throws.
    // While true, `shouldRender` is forced false so the widget
    // collapses to its zero-paint stable slot. Resets on the next
    // valid snapshot (set inside _applyPending success path).
    //
    // Pattern mirrors GNOME 45+ "fail closed" extension model and
    // noctalia upstream's PluginService.recordPluginError + disablePlugin
    // pair (Services/Noctalia/PluginService.qml:639). Local equivalent
    // since noctalia's runtime trap doesn't auto-disable us yet.
    property bool _failedState: false

    // `_pendingSnapshot` is the latest received payload waiting to be
    // applied on the next Qt event-loop tick. Qt.callLater coalesces
    // multiple emits to one apply (free debounce per Qt docs). We MUST
    // route every snapshot through this defer because:
    //
    //   1. IpcHandler.update fires synchronously on the bridge IPC
    //      marshal stack; mutating root.topLevel mid-marshal can race
    //      with the QML engine's binding eval. Bar.qml:158-172 hit a
    //      SIGSEGV in QV4::Object::insertMember from the same class
    //      and fixed it via Qt.callLater(_initModels).
    //   2. FileView.onLoaded fires while the QML engine is finalising
    //      Component construction at startup; deferring to the next
    //      tick lets initialisation settle.
    //   3. Throws inside the apply path now land on a fresh stack
    //      frame, bounded by the per-call try/catch envelope. They
    //      cannot poison the bridge IPC dispatcher or the FileView
    //      reload pipeline.
    property var _pendingSnapshot: undefined

    /// Apply a parsed bridge snapshot object (the JSON written to
    /// `active.json` and pushed via IPC) to the widget's exposed
    /// state. Pure — both the FileView (cold-start) and IpcHandler
    /// (steady-state push, PR #44) call into here so the two paths
    /// stay byte-identical.
    ///
    /// PR #51 — identity-stable topLevel guard. Mirrors the upstream
    /// fix for noctalia#2546 (dock/workspace icons flickering on every
    /// `titleChanged` because the model was wholesale-replaced even
    /// when nothing actually changed). We compare `id`/`label`/`enabled`
    /// of each top-level entry; if all match, we skip the assignment
    /// entirely and the Repeater never tears down its delegates. The
    /// bridge currently re-pushes the full menu tree on every focus
    /// change even when the focused app didn't change, so this guard
    /// is the difference between "delegates rebuilt N times per
    /// minute" and "delegates rebuilt only on real menu changes."
    /// Public API entry — same shape and contract as before, but the
    /// implementation now defers the actual mutation through
    /// Qt.callLater. This keeps the call site simple (callers still
    /// just `root.applySnapshot(j)`) while routing every state mutation
    /// through the isolation envelope (`_pendingSnapshot` + `_applyPending`).
    function applySnapshot(j) {
        // null is a sentinel "clear state". `undefined` is the unset
        // sentinel for `_pendingSnapshot` so we encode null explicitly.
        root._pendingSnapshot = (j === null) ? null : j;
        Qt.callLater(root._applyPending);
    }

    /// Internal: runs on the next Qt event-loop tick. Wraps the entire
    /// state mutation in a try/catch envelope. On throw, flips
    /// `_failedState` so `shouldRender` becomes false and the widget
    /// falls back to its zero-paint stable slot. Subsequent valid
    /// snapshots clear the failed state.
    ///
    /// Coalescing: if `applySnapshot` is called multiple times in one
    /// tick, only the LAST `_pendingSnapshot` value is observed —
    /// `Qt.callLater` deduplicates within the tick and the variable
    /// retains the latest write. This gives free debounce.
    function _applyPending() {
        const pending = root._pendingSnapshot;
        if (pending === undefined) {
            // Already drained on a prior tick; nothing to do.
            return;
        }
        // Drain BEFORE running so a throw doesn't trap us in a retry
        // loop on the same bad payload.
        root._pendingSnapshot = undefined;
        try {
            root._applySnapshotInner(pending);
            if (root._failedState) {
                console.log("[appmenu] envelope cleared; resuming render");
                root._failedState = false;
            }
        } catch (e) {
            console.error("[appmenu] envelope caught in _applyPending:", e,
                          "stack:", (e && e.stack) || "(no stack)");
            root._failedState = true;
        }
    }

    /// The actual state mutation. Called only from `_applyPending`
    /// inside the envelope. Throws on malformed input are caught
    /// upstream.
    function _applySnapshotInner(j) {
        if (!j) {
            root.appId = "";
            root.title = "";
            root.menuService = "";
            root.menuPath = "";
            root.focusWinid = 0;
            if (root.topLevel.length > 0) root.topLevel = [];
            return;
        }
        root.appId = j.app_id || "";
        root.title = j.title || "";
        root.menuService = j.menu_service || "";
        root.menuPath = j.menu_path || "";
        root.focusWinid = j.focus_winid || 0;
        // Walk into menu.children. Defaults to empty when bridge
        // wrote `menu: null` (e.g. focused app has no menu and no
        // synthetic fallback applied).
        const newTopLevel = (j.menu && j.menu.children) ? j.menu.children : [];
        if (!root._sameTopLevel(root.topLevel, newTopLevel)) {
            root.topLevel = newTopLevel;
        }
    }

    /// Cheap structural-equality check for top-level menu arrays.
    ///
    /// Compares id/label/enabled at the top level (drives the bar
    /// strip), AND first-level children's count + labels (drives the
    /// dropdown body). Spec 009 FR-005: the prior implementation
    /// skipped children entirely, so a `MenuError::Stale` re-walk
    /// that produced an updated subtree under unchanged top-level
    /// labels was silently dropped — `topLevel = newTopLevel` was
    /// short-circuited and the Repeater never refreshed, leaving the
    /// popup with stale `modelData.children` references on the next
    /// open.
    ///
    /// First-level children only — Qt re-emits the full tree on
    /// `accessible-children-changed`, so a deep child change almost
    /// always rolls up to a first-level structural difference. A
    /// shallow comparison keeps the dedup cheap (PR #51 anti-flicker
    /// invariant: avoid wholesale model reassignment unless the shape
    /// actually changed).
    function _sameTopLevel(a, b) {
        if (a === b) return true;
        if (!a || !b) return false;
        if (a.length !== b.length) return false;
        for (let i = 0; i < a.length; i++) {
            if (a[i].id !== b[i].id) return false;
            if (a[i].label !== b[i].label) return false;
            if (a[i].enabled !== b[i].enabled) return false;
            const ac = (a[i].children || []);
            const bc = (b[i].children || []);
            if (ac.length !== bc.length) return false;
            for (let j = 0; j < ac.length; j++) {
                if ((ac[j] && ac[j].label) !== (bc[j] && bc[j].label)) {
                    return false;
                }
            }
        }
        return true;
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

        // Quickshell's IpcHandler MOC dispatcher cannot transit
        // QVariant across the IPC socket. Untyped `function update(json)`
        // is rejected at registration with "Type of argument 1 (json:
        // QVariant) cannot be used across IPC". Typing the param as
        // `string` is the canonical idiom — every IpcHandler in
        // upstream `noctalia-shell/Services/Control/IPCService.qml`
        // uses this pattern (e.g. `function send(json: string)` for
        // toast). Bridge writes JSON-encoded strings.
        function update(json: string): void {
            // Spec 003 envelope (FR-008): wrap the entry-point so
            // a JSON parse error or malformed payload trips the
            // failed-state flag instead of poisoning the IPC
            // dispatcher. `applySnapshot` itself is already
            // Qt.callLater-deferred so the inner mutation cannot
            // re-enter the IpcHandler stack.
            try {
                const j = JSON.parse(json);
                root.applySnapshot(j);
            } catch (e) {
                console.error("[appmenu] envelope caught in IpcHandler.update:", e,
                              "stack:", (e && e.stack) || "(no stack)",
                              "json-prefix:", json ? json.substring(0, 80) : "(empty)");
                root._failedState = true;
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
            // Spec 003 envelope (FR-008): same protection as
            // IpcHandler.update. FileView.onLoaded fires synchronously
            // when the inotify watch trips; a throw here used to
            // propagate to noctalia's parent FileView reload pipeline.
            try {
                // FileView's `text` is a FUNCTION call (ADR-0021).
                const content = text();
                if (!content || content.length === 0) {
                    root.applySnapshot(null);
                    return;
                }
                const j = JSON.parse(content);
                root.applySnapshot(j);
            } catch (e) {
                // Partial-write or malformed file — skip this load,
                // wait for next inotify event. Don't trip _failedState
                // for this case: cold-start often races bridge writes
                // and the next reload will succeed naturally.
                console.log("[appmenu] FileView.onLoaded skipped:", e ? e.message : "(unknown)");
            }
        }
    }

    // ── Layout ───────────────────────────────────────────────────────
    //
    // PR #51 — STABLE-SLOT, ANIMATED, NO FLICKER.
    //
    // Earlier alpha (PR #47..#50) toggled `visible` and swung
    // `implicitWidth` between 0 and `strip.implicitWidth + margins`
    // on every focus change. Pedro reported full-screen flicker as
    // bar state changed — root cause documented in research note
    // `Documents/Notes/Research/noctalia-appmenu-2026-05-10-v2/01-quickshell-flicker.md`:
    //
    //   1. noctalia v4 puts the entire bar (and dropdowns) on a single
    //      full-screen PanelWindow (`MainScreen.qml`) so the dimmed
    //      backdrop and inverted-corner shadow can share one surface.
    //      See noctalia#2216 ("MainScreen panels and desktop widgets
    //      always damage entire screen", closed → addressed in v5).
    //   2. ANY layout change to ANY bar widget marks the whole shared
    //      surface damaged. niri redraws the whole output. AMD GPUs
    //      manifest the redraw as visible flicker (Pedro's class).
    //   3. Our plugin was the worst offender on Pedro's bar — the
    //      only widget that swung 0↔~600px on every focus change.
    //
    // Fix triplet (mirrors noctalia ActiveWindow.qml — the canonical
    // "widget that comes and goes by focus" pattern):
    //
    //   • `reserveSlot` (default true): widget always claims
    //     `maxStripWidth + margins`. Width is constant regardless
    //     of focused app, so the bar layout pass never re-runs and
    //     the bar surface is never marked damaged on focus change.
    //   • `Behavior on implicitWidth` 180ms InOutCubic: even with
    //     reserveSlot=false, residual width changes (Firefox menu
    //     vs GIMP menu have different `strip.implicitWidth`) are
    //     spread over many frames as small deltas instead of a
    //     single-frame jump.
    //   • `Behavior on opacity` 180ms OutCubic + `visible: shouldRender
    //     || opacity > 0`: the Item stays in the QML layout tree until
    //     the fade completes. Layout invalidation is therefore deferred
    //     and smoothed, and the GPU compositor renders opacity=0
    //     subtrees as a no-op.
    //
    // `reserveSlot=false` is opt-out for users who want the
    // collapse-to-zero behaviour and accept the AMD flicker.
    //
    // `fallbackText` opt-in still works: when set with reserveSlot=false,
    // the slot is sized to `maxLabelWidth` instead, and the text label
    // is always painted.
    readonly property bool reserveSlot: widgetSettings.reserveSlot !== undefined
        ? widgetSettings.reserveSlot
        : true

    implicitHeight: Style.barHeight
    implicitWidth: {
        if (reserveSlot) return maxStripWidth + Style.marginM * 2;
        if (hasMenu) return maxStripWidth + Style.marginM * 2;
        if (fallbackText !== "") return maxLabelWidth + Style.marginM * 2;
        return 0;
    }
    // Stay in layout tree until fade completes. Toggling `visible`
    // mid-fade reintroduces a single-frame layout invalidation; this
    // bridges the two states so the layout pass is deferred until
    // opacity has actually settled.
    visible: shouldRender || opacity > 0
    opacity: shouldRender ? 1.0 : 0.0
    Behavior on opacity {
        NumberAnimation { duration: Style.animationNormal !== undefined ? Style.animationNormal : 180; easing.type: Easing.OutCubic }
    }
    Behavior on implicitWidth {
        NumberAnimation { duration: Style.animationNormal !== undefined ? Style.animationNormal : 180; easing.type: Easing.InOutCubic }
    }

    // ── Static fallback label (opt-in) ─────────────────────────────
    // Only renders when `fallbackText` is configured per-widget. With
    // empty fallbackText (default) we hide entirely instead of showing
    // app_id-as-text, per Pedro's split-the-loss UX.
    Text {
        id: fallbackLabel
        visible: !root.hasMenu && root.fallbackText !== ""
        anchors.fill: parent
        anchors.leftMargin: Style.marginM
        anchors.rightMargin: Style.marginM
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignLeft
        text: root.fallbackText
        color: Color.mOnSurface
        font.family: Settings.data.ui.fontDefault || "Inter"
        font.pixelSize: Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
        elide: Text.ElideRight
    }

    // ── v0.2 menu strip: horizontal row of top-level menu buttons ──
    //
    // PR #50 fix: anchor strip to FILL the parent vertically (was
    // verticalCenter only). Each button Rectangle now also fills the
    // full bar height — earlier `implicitHeight: barHeight - marginS*2`
    // produced a centered button shorter than the visible row, leaving
    // dead-zones above and below where MouseArea wouldn't fire. Pedro
    // hovered "File" but the click silently dropped because the cursor
    // landed in the dead-zone above the rectangle.
    // PR #172 — ONE grouping capsule wraps the whole menu strip. The
    // app-menu is a single bar widget, so it gets a single noctalia
    // capsule (like the ActiveWindow widget) rather than N per-item
    // pills — which looked busy and dark-on-dark for real multi-item
    // menus (File/View/Playback/Help…). Subtle `capsuleColor` group pill
    // (matches Clock / the other bar widgets); the ACTIVE item gets the
    // bright `mHover` cyan highlight INSET inside it (readable,
    // macOS-menubar feel). Drawn at `capsuleHeight`, vertically centred;
    // the strip itself still fills bar height so the click area has no
    // dead-zone. All shell tokens `!== undefined`-guarded.
    Rectangle {
        id: menuCapsule
        visible: strip.visible
        anchors.left: parent.left
        anchors.leftMargin: Style.marginS
        anchors.verticalCenter: parent.verticalCenter
        width: strip.implicitWidth + Style.marginS * 2
        height: Style.capsuleHeight !== undefined
            ? Style.capsuleHeight
            : (Style.barHeight - Style.marginS * 2)
        radius: Style.radiusM !== undefined ? Style.radiusM : (height / 2)
        color: Style.capsuleColor !== undefined ? Style.capsuleColor : Color.mSurfaceVariant
        border.color: Style.capsuleBorderColor !== undefined ? Style.capsuleBorderColor : "transparent"
        border.width: Style.capsuleBorderWidth !== undefined ? Style.capsuleBorderWidth : 0
    }

    Row {
        id: strip
        visible: root.topLevel.length > 0
        anchors.left: menuCapsule.left
        anchors.leftMargin: Style.marginS
        anchors.top: parent.top
        anchors.bottom: parent.bottom
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

                // PR #169 — bar-integrated menubar styling. The
                // top-level item is a macOS-style bare-text menubar
                // entry (no persistent capsule), but matched to the
                // Noctalia bar's visual language: Medium weight + bar
                // font size, and a capsule HIGHLIGHT on hover/open using
                // the shell's own `mHover` capsule treatment (cyan bg +
                // dark text, radiusM, animationFast). Tokens come from
                // `qs.Commons` (noctalia-shell); each is guarded with a
                // `!== undefined` fallback so a token rename in a future
                // noctalia release degrades gracefully instead of
                // throwing in the QML binding.
                //
                // `active` = hovered OR this is the open menu's anchor —
                // so the open top-level item stays highlighted like a
                // real menubar.
                readonly property bool active: hover.containsMouse
                    || (popup.isOpen && popup.anchorItem === btn)
                readonly property bool isEnabled: !(modelData && modelData.enabled === false)

                color: "transparent"            // bare container; highlight is the child below
                border.width: 0

                // Fill full strip height (= bar height) for the hit area.
                // Eliminates the click dead-zone the earlier
                // "barHeight - marginS*2" sizing introduced — MouseArea
                // covers everything a user associates with the button.
                // The VISIBLE highlight is drawn at capsule height
                // (centered) to match the shell's other bar capsules,
                // which are shorter than the full bar height.
                height: strip.height
                implicitWidth: btnLabel.implicitWidth + Style.marginM * 2

                // PR #172 — per-item highlight is now an INSET selected
                // segment WITHIN the grouping capsule: transparent at
                // rest (the item is bare text on the group pill), and the
                // shell's bright `Color.mHover` cyan when hovered/open.
                // Inset by `marginXS` on the height + a smaller `radiusS`
                // so it nests visibly inside the outer `menuCapsule`
                // rather than covering it — the macOS-menubar
                // "highlighted item" look. No per-item border (the group
                // capsule owns the chrome).
                Rectangle {
                    id: highlight
                    anchors.centerIn: parent
                    width: parent.width
                    height: (Style.capsuleHeight !== undefined
                        ? Style.capsuleHeight
                        : (Style.barHeight - Style.marginS * 2))
                        - (Style.marginXS !== undefined ? Style.marginXS * 2 : 8)
                    radius: Style.radiusS !== undefined ? Style.radiusS : (height / 2)
                    color: btn.active && btn.isEnabled
                        ? (Color.mHover !== undefined ? Color.mHover : Color.mSurfaceVariant)
                        : "transparent"
                    Behavior on color {
                        ColorAnimation {
                            duration: Style.animationFast !== undefined ? Style.animationFast : 150
                            easing.type: Easing.InOutQuad
                        }
                    }
                }

                Text {
                    id: btnLabel
                    anchors.centerIn: parent
                    // Strip leading underscore (accelerator marker).
                    text: (modelData ? modelData.label : "").replace(/_/g, "")
                    color: !btn.isEnabled
                        ? Color.mOnSurfaceVariant
                        : (btn.active
                            ? (Color.mOnHover !== undefined ? Color.mOnHover : Color.mOnSurface)
                            : Color.mOnSurface)
                    font.family: Settings.data.ui.fontDefault || "Inter"
                    // Match the shell's bar widgets: Medium weight + the
                    // density-aware barFontSize (≡ our prior
                    // _barBaseFontSize * fontScale; prefer the shell token).
                    font.weight: Style.fontWeightMedium !== undefined ? Style.fontWeightMedium : Font.Medium
                    font.pixelSize: Style.barFontSize !== undefined
                        ? Style.barFontSize
                        : Math.max(1, Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0))
                    Behavior on color {
                        ColorAnimation {
                            duration: Style.animationFast !== undefined ? Style.animationFast : 150
                            easing.type: Easing.InOutQuad
                        }
                    }
                }

                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton
                    enabled: btn.modelData && btn.modelData.enabled !== false
                    onClicked: {
                        if (!btn.modelData) return;
                        // Trace clicks (PR #50) — surfaced in
                        // `journalctl --user -u noctalia-shell`.
                        // Helps diagnose "click does nothing" reports
                        // by confirming whether the MouseArea fires.
                        console.log("[appmenu] click on top-level:",
                                    btn.modelData.label,
                                    "children:",
                                    (btn.modelData.children
                                        ? btn.modelData.children.length
                                        : 0));
                        // v1.0.9 — clicking the SAME button while its
                        // popup is open toggles it closed (standard
                        // menubar UX). Clicking a DIFFERENT button while
                        // a popup is open re-aims the popup to the new
                        // anchor; `openAt` handles the reposition.
                        if (popup.isOpen && popup.anchorItem === btn) {
                            popup.close();
                        } else if (btn.modelData.children && btn.modelData.children.length > 0) {
                            // Issue #109 — capture the niri window id of
                            // the *currently-focused* window at popup-open
                            // time. Reused in fireClick so any action
                            // fired through the popup routes back to this
                            // exact window, even if the user hovers over
                            // a sibling window before clicking a leaf.
                            root._capturedWinid = root.focusWinid;
                            popup.openAt(btn, btn.modelData);
                        } else if (root._isLikelyTopLevelGroup(btn.modelData)
                                   && btn.modelData.service
                                   && btn.modelData.path
                                   && btn.modelData.service !== "::synthetic") {
                            // ADR-0034 — Firefox/GTK lazy-menu expand on
                            // demand. A top-level that claims zero children
                            // is a lazily-realised menu: Firefox/GTK only
                            // populate items over AT-SPI once the menu is
                            // actually opened, so the bridge's passive walk
                            // (and the old RefreshActive re-walk) both saw
                            // zero → "click does nothing". Spawn the bridge
                            // `atspi-expand` subcommand: it DoAction-expands
                            // the menu, walks the now-realised items,
                            // collapses, and prints them as JSON. We open the
                            // popup with those items — bar-anchored and
                            // click-safe (realised paths persist, verified).
                            // Single-shot per open: an in-flight expand drops
                            // the duplicate click.
                            if (root._pendingRetryButton !== null) {
                                console.log("[appmenu] expand already in flight — ignoring duplicate click:",
                                            btn.modelData.label);
                                return;
                            }
                            console.log("[appmenu] empty top-level — atspi-expand:",
                                        btn.modelData.label);
                            root._pendingRetryButton = btn;
                            root._selfHealCount++;
                            const xcmd = [root.bridgeBin, "atspi-expand",
                                          btn.modelData.service, btn.modelData.path];
                            if (root.focusWinid > 0)
                                xcmd.push("--winid", String(root.focusWinid));
                            expandProcess.command = xcmd;
                            expandProcess.running = true;
                        } else {
                            // Leaf at top level OR submenu the bridge
                            // could not walk (Firefox lazy AT-SPI realises
                            // some menus only on user interaction — see
                            // bridge `KNOWN_NO_MENUBAR_APPS` / v1.0.8
                            // notes). Fire the AT-SPI click so the app
                            // owns the popup. Always close any leftover
                            // popup so the bar matches the new state.
                            if (popup.isOpen) popup.close();
                            root.fireClick(btn.modelData);
                        }
                    }
                }
            }
        }
    }

    // ── Submenu dropdown — sibling top-level PanelWindow ─────────────
    //
    // PR #52 — replaces the inline `PopupWindow` (PR #49 / alpha.13) with
    // a sibling layer-shell `PanelWindow` declared at the BarWidget
    // root. Why: research note 02-popupwindow-input.md identified the
    // bar-click-dead-zone Pedro reported as a Wayland PROTOCOL property
    // of `Quickshell.PopupWindow`:
    //
    //   • grabFocus=true → Qt::Popup → xdg_popup.grab(wl_seat) →
    //     compositor MUST route ALL pointer/keyboard input to the
    //     popup until popup_done. Bar surface receives zero events.
    //   • grabFocus=false → Qt::ToolTip → Qt-Quick scene-graph capture
    //     keeps pointer events on the popup root. propagateComposedEvents
    //     bubbles only within ONE QML scene; cannot cross wl_surface.
    //
    // Either way the bar feels frozen while the menu is open. There is
    // no compositor knob and no QML knob; the fix is to STOP using
    // PopupWindow for bar dropdowns.
    //
    // AppmenuPopupWindow is the sibling layer-shell surface
    // (`WlrLayer.Top`, `keyboardFocus: None`, `exclusionMode: Ignore`).
    // Wayland routes input surface-by-surface based on cursor position,
    // so the bar stays clickable while the menu is up. Outside-click is
    // caught by a full-screen MouseArea inside the popup window itself.
    // v1.0.16 Option G — outside-click dismissal lives INSIDE the
    // AppmenuPopupWindow surface (which is itself full-screen +
    // transparent + Overlay layer + a MouseArea-fill catches outside
    // clicks). No separate shield. Per spec 014.
    AppmenuPopupWindow {
        id: popup
        screen: root.screen
        focusedScreenName: root.focusedScreenName

        onItemActivated: function (item) {
            root.fireClick(item);
        }

        // Issue #109 — clear the captured winid the instant the popup
        // closes. Surfaces a routing bug as a no-op pre-focus rather
        // than silently re-using the previous popup's window id.
        // Spec 015 FR-005 — emit the self-heal counter on close;
        // gates/self-heal.sh greps for `retried=0` in steady state.
        onIsOpenChanged: {
            if (!isOpen) {
                console.log("[appmenu] popup-close",
                            "retried=" + root._selfHealCount);
                root._capturedWinid = 0;
                root._selfHealCount = 0;
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
        // Issue #109 — pass the captured winid to the bridge so it
        // can pre-focus the right niri window before DoAction. When
        // unknown (older bridge, synthetic items), `_capturedWinid`
        // is 0 and the bridge skips the pre-focus step.
        const cmd = [
            root.bridgeBin,
            "atspi-click",
            item.service,
            item.path
        ];
        if (root._capturedWinid > 0) {
            cmd.push("--winid", String(root._capturedWinid));
        }
        clickProcess.command = cmd;
        clickProcess.running = true;
    }

    Process {
        id: clickProcess
        // command set per-call in fireClick(). running: false default.
    }

    // ── v1.0.21 empty-popup self-heal ─────────────────────────────────
    // Pedro image #5: bar shows top-level strip but clicking opens
    // nothing because the bridge's first AT-SPI walk lost the lazy
    // realisation race and serialised top-level entries with empty
    // children. Self-heal: trigger RefreshActive, settle, retry-open
    // the same button once. Heuristic — only nudge entries whose label
    // matches the standard top-level menu vocabulary (File / Edit / …)
    // so we never re-walk for a real leaf the bridge intentionally
    // serialised without children.

    /// Buttons captured for the retry-open path. Cleared after one
    /// retry — refusing to loop indefinitely if the re-walk also
    /// returns zero children.
    property var _pendingRetryButton: null

    /// Spec 015 FR-005 telemetry — bumped each time self-heal fires
    /// in this widget's lifetime. Emitted on popup close so steady-
    /// state runs surface as `retried=0` in journal. SC-003 gate
    /// scans for that line over 50 steady-state opens.
    property int _selfHealCount: 0

    /// Returns true when `item.label` looks like a top-level group
    /// (File, Edit, View, History, Bookmarks, Profiles, Tools, Help,
    /// Window, Go, Settings, Help) — heuristic, intentionally
    /// permissive. False for empty / leaf-only labels.
    function _isLikelyTopLevelGroup(item) {
        if (!item || !item.label) return false;
        const label = String(item.label).replace(/_/g, "").trim();
        const known = ["File", "Edit", "View", "History", "Bookmarks",
                       "Profiles", "Tools", "Help", "Window", "Go",
                       "Settings", "Format", "Insert", "Run", "Build",
                       "Project", "Debug", "Terminal", "Tabs", "Tab"];
        return known.indexOf(label) !== -1;
    }

    /// ADR-0034 — Firefox/GTK lazy-menu expand-on-demand. Spawned by the
    /// top-level click handler for an empty group: the bridge
    /// `atspi-expand` subcommand DoAction-expands the menu, walks the
    /// now-realised items, collapses it, and prints them as a JSON array
    /// on stdout. `StdioCollector` captures that; on finish we open the
    /// popup with the realised children (bar-anchored, click-safe — the
    /// realised paths persist). On empty / parse-failure we fall through
    /// to a native `DoAction` so the click still does *something*.
    Process {
        id: expandProcess
        // command set per-call in the top-level click handler.
        stdout: StdioCollector {
            id: expandCollector
            onStreamFinished: {
                const btn = root._pendingRetryButton;
                root._pendingRetryButton = null;
                if (!btn || !btn.modelData) return;
                const md = btn.modelData;
                let kids = [];
                try {
                    kids = JSON.parse(expandCollector.text || "[]");
                } catch (e) {
                    console.log("[appmenu] atspi-expand JSON parse failed:", e);
                }
                if (kids && kids.length > 0) {
                    console.log("[appmenu] expand-on-demand realised",
                                kids.length, "items for:", md.label);
                    // Clone the node with realised children — `modelData`
                    // from the active.json model is read-only.
                    const realised = {
                        id: md.id, label: md.label, type: md.type,
                        enabled: md.enabled, visible: md.visible,
                        icon_name: md.icon_name, service: md.service,
                        path: md.path, children: kids
                    };
                    root._capturedWinid = root.focusWinid;
                    popup.openAt(btn, realised);
                } else {
                    console.log("[appmenu] expand-on-demand empty for:",
                                md.label, "— native click fallback");
                    if (popup.isOpen) popup.close();
                    root.fireClick(md);
                }
            }
        }
    }
}
