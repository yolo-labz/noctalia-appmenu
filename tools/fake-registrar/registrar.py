#!/usr/bin/env python3
"""
Fake com.canonical.AppMenu.Registrar daemon for integration tests.

Publishes a fixed (canned) DBusMenu tree at a known service+path and emits
a WindowRegistered signal so the bridge picks it up. Usage:

    python3 tools/fake-registrar/registrar.py            # default canned menu
    python3 tools/fake-registrar/registrar.py --pid 1234 # spoof PID for testing

Run inside `nix develop` (devShell ships dbus-python + pygobject3).
"""

from __future__ import annotations

import argparse
import os
import sys
import logging
from typing import Any

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib  # noqa: E402  pygobject3

REGISTRAR_BUS = "com.canonical.AppMenu.Registrar"
REGISTRAR_PATH = "/com/canonical/AppMenu/Registrar"
FAKE_MENU_BUS = "org.noctalia.test.FakeApp"
FAKE_MENU_PATH = "/org/noctalia/test/FakeApp/Menu"

LAYOUT_VERSION = 1


class FakeRegistrar(dbus.service.Object):
    def __init__(self, bus: dbus.Bus, path: str):
        super().__init__(bus, path)
        self.menus: dict[int, tuple[str, str]] = {}

    @dbus.service.method("com.canonical.AppMenu.Registrar", in_signature="uo", out_signature="")
    def RegisterWindow(self, window_id: int, menu_path: str) -> None:  # noqa: N802
        sender = "org.example"  # unknown without sender introspection
        self.menus[window_id] = (sender, menu_path)
        self.WindowRegistered(window_id, sender, menu_path)
        logging.info("registered windowId=%d menu_path=%s", window_id, menu_path)

    @dbus.service.method("com.canonical.AppMenu.Registrar", in_signature="u", out_signature="")
    def UnregisterWindow(self, window_id: int) -> None:  # noqa: N802
        self.menus.pop(window_id, None)
        self.WindowUnregistered(window_id)
        logging.info("unregistered windowId=%d", window_id)

    @dbus.service.method("com.canonical.AppMenu.Registrar", in_signature="u", out_signature="so")
    def GetMenuForWindow(self, window_id: int) -> tuple[str, str]:  # noqa: N802
        return self.menus.get(window_id, ("", "/"))

    @dbus.service.signal("com.canonical.AppMenu.Registrar", signature="uso")
    def WindowRegistered(self, window_id: int, service_name: str, menu_path: str) -> None:  # noqa: N802
        pass

    @dbus.service.signal("com.canonical.AppMenu.Registrar", signature="u")
    def WindowUnregistered(self, window_id: int) -> None:  # noqa: N802
        pass


class FakeMenu(dbus.service.Object):
    """Minimal com.canonical.dbusmenu impl — flat 3-item menubar."""

    def __init__(self, bus: dbus.Bus, path: str):
        super().__init__(bus, path)

    @dbus.service.method("com.canonical.dbusmenu", in_signature="iias", out_signature="u(ia{sv}av)")
    def GetLayout(self, _parent_id: int, _depth: int, _props: list[str]) -> Any:  # noqa: N802
        # Root with 3 top-level entries
        children = [
            (1, {"label": "File", "children-display": "submenu"}, []),
            (2, {"label": "Edit", "children-display": "submenu"}, []),
            (3, {"label": "Help", "children-display": "submenu"}, []),
        ]
        root_props: dict[str, Any] = {"children-display": "submenu"}
        return (LAYOUT_VERSION, (0, root_props, children))

    @dbus.service.method("com.canonical.dbusmenu", in_signature="isvu", out_signature="")
    def Event(self, _id: int, _event_id: str, _data: Any, _timestamp: int) -> None:  # noqa: N802
        return None

    @dbus.service.signal("com.canonical.dbusmenu", signature="ui")
    def LayoutUpdated(self, _revision: int, _parent_id: int) -> None:  # noqa: N802
        pass


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--debug", action="store_true")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.debug else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()

    if bus.name_has_owner(REGISTRAR_BUS):
        logging.error("%s is already owned — kill the real registrar first", REGISTRAR_BUS)
        return 1

    bus_name = dbus.service.BusName(REGISTRAR_BUS, bus)
    fake_app = dbus.service.BusName(FAKE_MENU_BUS, bus)
    registrar = FakeRegistrar(bus, REGISTRAR_PATH)  # noqa: F841
    menu = FakeMenu(bus, FAKE_MENU_PATH)  # noqa: F841

    # Register a synthetic window 1
    registrar.RegisterWindow(1, FAKE_MENU_PATH)

    logging.info(
        "fake registrar listening — pid=%d, fake_menu_service=%s, fake_menu_path=%s",
        os.getpid(),
        FAKE_MENU_BUS,
        FAKE_MENU_PATH,
    )

    GLib.MainLoop().run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
