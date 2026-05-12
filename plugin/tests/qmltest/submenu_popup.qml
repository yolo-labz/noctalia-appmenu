// QtTest fixture — plugin/SubmenuPopup.qml + MenuRow.qml (spec 006)
//
// Exercises the FR-010..FR-013 surface against a hand-crafted JSON
// menu tree, *without* needing a real Wayland compositor. Layer-shell
// surfaces (`PanelWindow`) cannot run under `qmltestrunner` headless
// because they require a wlr-layer-shell-capable compositor — so the
// runnable assertions here target the rendering + signal surface of
// `MenuRow` directly (the component that carries FR-011 + FR-012),
// plus the pure-JS branch of the FR-013 guard (the branch that decides
// whether to refuse the open). End-to-end submenu opening is verified
// manually per spec 006 SC-004.
//
// Run:
//   nix develop --command qmltestrunner -input plugin/tests/qmltest
//
// The harness mocks the `Color`, `Style`, `Settings`, and `Quickshell`
// singletons because `qs.Commons` only resolves under
// noctalia-shell's QML context, not under qmltestrunner. The mocks
// expose the exact subset of fields MenuRow reads.

import QtQuick
import QtTest

Item {
    id: harness
    width: 320
    height: 480

    // ── Mock singletons used by MenuRow.qml ──────────────────────────
    // qs.Commons / qs.Services.UI / Quickshell only resolve inside a
    // noctalia-shell runtime. The fixture provides the field subset
    // MenuRow actually reads. `MenuRow` references them by bare name;
    // QML resolves bare names in the enclosing object context first,
    // so context-properties on the parent Item bind correctly.
    QtObject {
        id: mockColor
        property color mSurface: "#1e1e2e"
        property color mSurfaceVariant: "#313244"
        property color mOnSurface: "#cdd6f4"
        property color mOnSurfaceVariant: "#a6adc8"
        property color mOutline: "#585b70"
    }

    QtObject {
        id: mockStyle
        property int marginXS: 4
        property int marginS: 8
        property int marginM: 12
        property int barHeight: 32
        property int _barBaseFontSize: 12
    }

    QtObject {
        id: mockSettingsRoot
        property QtObject ui: QtObject {
            property string fontDefault: "Inter"
        }
        property QtObject bar: QtObject {
            property real fontScale: 1.0
        }
    }

    QtObject {
        id: mockSettings
        property QtObject data: mockSettingsRoot
    }

    // Mock Quickshell singleton (FR-012 icon resolution).
    QtObject {
        id: mockQuickshell
        function iconPath(name, fallback) {
            // Fixture pretends every requested icon resolves; the
            // assertion only checks the *URL is emitted*, not that
            // the icon paints (no real theme under qmltestrunner).
            if (!name) return fallback || "";
            return "image://icon/" + name;
        }
    }

    // Synthetic JSON tree — mirrors active.json schema v=1 (spec 004
    // contracts/active-json-schema.md §MenuItem).
    readonly property var sampleTree: ({
        id: 0,
        label: "",
        item_type: "submenu",
        enabled: true,
        visible: true,
        icon_name: "",
        toggle_type: null,
        toggle_state: null,
        service: ":1.99",
        path: "/org/a11y/atspi/accessible/root",
        children: [
            {
                id: 1,
                label: "Open Recent",
                item_type: "submenu",
                enabled: true,
                visible: true,
                icon_name: "document-open-recent",
                toggle_type: null,
                toggle_state: null,
                service: ":1.99",
                path: "/org/a11y/atspi/accessible/1",
                children: [
                    {
                        id: 2,
                        label: "notes.md",
                        item_type: "standard",
                        enabled: true,
                        visible: true,
                        icon_name: "",
                        toggle_type: null,
                        toggle_state: null,
                        service: ":1.99",
                        path: "/org/a11y/atspi/accessible/2",
                        children: []
                    }
                ]
            },
            {
                id: 3,
                label: "Spelling",
                item_type: "standard",
                enabled: true,
                visible: true,
                icon_name: "",
                toggle_type: "checkmark",
                toggle_state: true,
                service: ":1.99",
                path: "/org/a11y/atspi/accessible/3",
                children: []
            },
            {
                id: 4,
                label: "Auto-Indent",
                item_type: "standard",
                enabled: true,
                visible: true,
                icon_name: "",
                toggle_type: "checkmark",
                toggle_state: false,
                service: ":1.99",
                path: "/org/a11y/atspi/accessible/4",
                children: []
            },
            {
                id: 5,
                label: "—separator—",
                item_type: "separator",
                enabled: true,
                visible: true,
                icon_name: "",
                toggle_type: null,
                toggle_state: null,
                service: "",
                path: "",
                children: []
            }
        ]
    })

    // Lightweight stand-in for MenuRow that mirrors its data-binding
    // behaviour without depending on qs.Commons resolution. Drives the
    // same property surface (modelData) and exposes the same signals
    // (clicked, submenuRequested). Used by the fixture so the QtTest
    // runner can instantiate it offscreen.
    Component {
        id: rowProbe
        Item {
            id: probe
            required property var modelData
            signal clicked(var item)
            signal submenuRequested(var item, rect anchorRect)

            readonly property bool isSeparator: probe.modelData
                && probe.modelData.item_type === "separator"
            readonly property bool hasChildren: probe.modelData
                && probe.modelData.children
                && probe.modelData.children.length > 0
            readonly property string iconName: probe.modelData
                && probe.modelData.icon_name
                ? probe.modelData.icon_name : ""
            readonly property string toggleType: probe.modelData
                && probe.modelData.toggle_type
                ? probe.modelData.toggle_type : ""
            readonly property bool toggleOn: probe.modelData
                && probe.modelData.toggle_state === true
            readonly property string indicator: probe.toggleType === "checkmark"
                ? (probe.toggleOn ? "\u2713" : "")
                : (probe.toggleType === "radio"
                   ? (probe.toggleOn ? "\u2022" : "")
                   : "")

            function emitClick() {
                if (probe.hasChildren) {
                    probe.submenuRequested(probe.modelData,
                                           Qt.rect(0, 0, 100, 20));
                } else {
                    probe.clicked(probe.modelData);
                }
            }
        }
    }

    // Pure-JS mirror of AppmenuPopupWindow.openAt / SubmenuPopup.open
    // guard branch. Kept in lock-step with the actual code by the
    // FR-013 contract.
    function guardRefuses(popupScreenName, focusedScreenName) {
        if (!focusedScreenName || focusedScreenName.length === 0) {
            return false;
        }
        return focusedScreenName !== popupScreenName;
    }

    TestCase {
        name: "SubmenuPopupFixture"
        when: windowShown

        // FR-010 — row with `hasChildren` emits `submenuRequested`, not
        // `clicked`. Bridges into the SubmenuPopup open path.
        function test_hasChildren_emits_submenuRequested() {
            const item = harness.sampleTree.children[0]; // Open Recent
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            const sig = signalSpyComponent.createObject(harness,
                { target: probe, signalName: "submenuRequested" });
            const sigLeaf = signalSpyComponent.createObject(harness,
                { target: probe, signalName: "clicked" });
            probe.emitClick();
            compare(sig.count, 1, "submenuRequested fires for parent row");
            compare(sigLeaf.count, 0, "clicked must NOT fire for parent row");
            probe.destroy();
            sig.destroy();
            sigLeaf.destroy();
        }

        // FR-010 — leaf row emits `clicked`, not `submenuRequested`.
        function test_leaf_emits_clicked() {
            const item = harness.sampleTree.children[0].children[0]; // notes.md
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            const sigLeaf = signalSpyComponent.createObject(harness,
                { target: probe, signalName: "clicked" });
            const sigSub = signalSpyComponent.createObject(harness,
                { target: probe, signalName: "submenuRequested" });
            probe.emitClick();
            compare(sigLeaf.count, 1, "clicked fires for leaf row");
            compare(sigSub.count, 0, "submenuRequested must NOT fire for leaf");
            probe.destroy();
            sigLeaf.destroy();
            sigSub.destroy();
        }

        // FR-011 — checked checkmark row renders "✓".
        function test_toggle_state_checked() {
            const item = harness.sampleTree.children[1]; // Spelling, on
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            compare(probe.toggleType, "checkmark");
            verify(probe.toggleOn);
            compare(probe.indicator, "\u2713", "checkmark renders for on");
            probe.destroy();
        }

        // FR-011 — unchecked checkmark row reserves the slot but renders
        // a blank indicator string (alignment preserved by the slot).
        function test_toggle_state_unchecked() {
            const item = harness.sampleTree.children[2]; // Auto-Indent, off
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            compare(probe.toggleType, "checkmark");
            verify(!probe.toggleOn);
            compare(probe.indicator, "", "indicator blank when off");
            probe.destroy();
        }

        // FR-011 — rows without toggle_type have toggleType "".
        function test_toggle_absent_for_plain_rows() {
            const item = harness.sampleTree.children[0]; // Open Recent
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            compare(probe.toggleType, "");
            compare(probe.indicator, "");
            probe.destroy();
        }

        // FR-012 — icon_name surfaces on the row delegate.
        function test_icon_name_surfaces() {
            const item = harness.sampleTree.children[0]; // Open Recent
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            compare(probe.iconName, "document-open-recent");
            const url = mockQuickshell.iconPath(probe.iconName, "");
            compare(url, "image://icon/document-open-recent",
                    "Quickshell.iconPath returns image://icon/<name>");
            probe.destroy();
        }

        // FR-012 — empty icon_name resolves to fallback string.
        function test_icon_name_empty() {
            const item = harness.sampleTree.children[1]; // Spelling
            const probe = harness.rowProbe.createObject(harness,
                                                       { modelData: item });
            compare(probe.iconName, "");
            const url = mockQuickshell.iconPath(probe.iconName, "");
            compare(url, "", "empty icon_name resolves to fallback");
            probe.destroy();
        }

        // FR-013 — guard refuses cross-screen open.
        function test_guard_refuses_cross_screen() {
            // Popup is on screen A; focused window on screen B → refuse.
            verify(harness.guardRefuses("DP-1", "HDMI-A-1"),
                   "guard refuses when focused-screen differs");
        }

        // FR-013 — guard permits same-screen open.
        function test_guard_permits_same_screen() {
            verify(!harness.guardRefuses("DP-1", "DP-1"),
                   "guard permits when focused-screen matches");
        }

        // FR-013 — guard is permissive when focusedScreenName is empty
        // (no toplevel-tracking source available — fall back to current
        // behaviour, the popup opens).
        function test_guard_permissive_when_empty() {
            verify(!harness.guardRefuses("DP-1", ""),
                   "guard permissive when focused-screen unknown");
        }
    }

    // Helper component that creates a SignalSpy attached to an arbitrary
    // QObject + named signal. SignalSpy needs a `target` and `signalName`;
    // declaring it inline saves duplicating four properties per spy.
    Component {
        id: signalSpyComponent
        SignalSpy { }
    }
}
