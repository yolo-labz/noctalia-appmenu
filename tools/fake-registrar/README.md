# Fake Registrar

> **STATUS — historical.** [ADR-0024](../../docs/adr/ADR-0024-atspi-substrate.md)
> switched the bridge to AT-SPI; this fake registrar targets the
> retired DBusMenu/Registrar pipeline. Current integration testing
> lives in `bridge/tests/atspi_integration.rs` and uses an in-process
> AT-SPI tree stub. Kept here for archaeology / re-enablement if
> upstream Qt6 ever ships compositor-agnostic registrar
> auto-registration.

A minimal Python implementation of `com.canonical.AppMenu.Registrar` and `com.canonical.dbusmenu` that publishes a canned 3-item menubar (`File`, `Edit`, `Help`) at a known service+path. Used by the integration test harness to validate the bridge end-to-end without requiring a real Qt or GTK app.

## Run

```bash
nix develop
python3 tools/fake-registrar/registrar.py
```

In another shell:

```bash
busctl --user introspect com.canonical.AppMenu.Registrar /com/canonical/AppMenu/Registrar
busctl --user introspect org.noctalia.test.FakeApp /org/noctalia/test/FakeApp/Menu
```

## Caveats

- Spoofs `WindowRegistered` with a synthetic windowId of `1`. Bridge ignores windowIds (ADR-0004) and resolves the registering connection's PID via `GetConnectionUnixProcessID` — that returns this Python process's actual PID.
- No `LayoutUpdated` churn — fake menu is static.
- Not a substitute for a real `vala-panel-appmenu` daemon in production.
