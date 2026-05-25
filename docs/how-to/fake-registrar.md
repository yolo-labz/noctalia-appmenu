# Running the fake registrar

> **STATUS — historical.** [ADR-0024](../adr/ADR-0024-atspi-substrate.md)
> switched the bridge to the AT-SPI substrate; the fake registrar
> below targets the retired DBusMenu/Registrar pipeline. Kept for
> contributors archaeologising the v0.1..v0.2 design and for
> reference if upstream Qt6 ever ships compositor-agnostic
> registrar auto-registration. For current integration testing,
> see [`atspi.md`](../architecture/atspi.md) and
> `bridge/tests/atspi_integration.rs`.

For integration testing without a real Qt/GTK app.

## What it is

`tools/fake-registrar/registrar.py` — a Python daemon that:

1. Owns `com.canonical.AppMenu.Registrar` on the user session bus.
2. Publishes a canned 3-item DBusMenu (`File`, `Edit`, `Help`) at a fixed `(service, path)` pair.
3. Synthetically emits `WindowRegistered` for windowId `1`.

The bridge picks it up like any real registrar.

## Run

```bash
nix develop                                  # devShell ships dbus-python + pygobject3
python3 tools/fake-registrar/registrar.py
```

In another shell:

```bash
busctl --user list | grep AppMenu
gdbus introspect --session \
  --dest org.noctalia.test.FakeApp \
  --object-path /org/noctalia/test/FakeApp/Menu
```

## Combine with the bridge

```bash
# terminal 1
python3 tools/fake-registrar/registrar.py

# terminal 2
just bridge-run-fg

# terminal 3 — verify the proxy reflects the fake app
gdbus introspect --session \
  --dest org.noctalia.AppMenu \
  --object-path /org/noctalia/AppMenu/Active
```

The bridge's published `appId` should match the fake app's bus name (or its registering Python process's `app_id` if niri sees it).

## Caveats

- Does NOT cover real menu activation paths (DBusMenuItem `Event` calls). Add explicit assertions if a feature touches them.
- Spoofs windowId `1` — the bridge ignores windowIds anyway ([ADR-0004](../adr/ADR-0004-resolve-registrar-by-pid.md)) so this is harmless.
- Not a replacement for `vala-panel-appmenu` in production — real apps register their actual menubars; this fake doesn't.
