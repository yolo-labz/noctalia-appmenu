# Configuration reference

Bridge config lives at `$XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml` (default `~/.config/noctalia-appmenu-bridge/config.toml`). All keys have sensible defaults — empty file is fine.

## Keys

| Key | Type | Default | Description |
|---|---|---|---|
| `focus_debounce_ms` | `u64` (ms) | `75` | Trail-edge debounce on niri focus events. ([ADR-0009](../adr/ADR-0009-debouncing-policy.md)) |
| `registrar_debounce_ms` | `u64` (ms) | `250` | Trail-edge debounce on registrar churn (e.g. KDE `KMainWindow` rebuilds). |
| `niri_binary` | `path` | `niri` | Path to the `niri` binary; resolved at startup. Set to absolute path if niri lives outside `$PATH`. |
| `publish_service` | `string` | `org.noctalia.AppMenu` | D-Bus bus name we own. **Constant across releases — don't change.** The QML widget hard-codes it. |
| `publish_path` | `string` | `/org/noctalia/AppMenu/Active` | D-Bus object path of the active proxy. **Constant across releases.** |

## Example

Minimal — just an override:

```toml
focus_debounce_ms = 100
```

Full:

```toml
focus_debounce_ms = 100
registrar_debounce_ms = 200
niri_binary = "/run/current-system/sw/bin/niri"
publish_service = "org.noctalia.AppMenu"
publish_path = "/org/noctalia/AppMenu/Active"
```

## Resolution rules

The bridge resolves the config path in this order:

1. `--config <path>` (CLI flag) — highest precedence.
2. `$XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml`
3. `$HOME/.config/noctalia-appmenu-bridge/config.toml`

If none of (1)–(3) yield a path, the bridge **errors out** and refuses to start — see audit P0 in `CHANGELOG.md`. There's no `/tmp` fallback (security).

If the resolved path doesn't exist on disk, the in-memory defaults apply.

## Home-Manager module

Users on NixOS configure via `programs.noctalia.plugins.appmenu.bridge.config`:

```nix
programs.noctalia.plugins.appmenu = {
  enable = true;
  bridge.config = {
    focus_debounce_ms = 50;
  };
};
```

The module renders the TOML and lays it at `~/.config/noctalia-appmenu-bridge/config.toml` via `xdg.configFile`.
