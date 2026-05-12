# Contract: Home-Manager module option surface (Lane C)

**Status:** modified at v1.0.0 (see FR-014..FR-020)
**File:** `nix/module.nix`
**Consumers:** noctalia-shell users who enable the plugin via Home-Manager

## Option tree

```nix
programs.noctalia.plugins.appmenu = {
  enable = mkEnableOption "noctalia-appmenu plugin (AT-SPI substrate)";

  registrar = mkOption {
    type = types.enum [ "vala-panel" "none" ];
    default = "none";                          # CHANGED in v1.0.0 (was "vala-panel")
    description = ''
      DEPRECATED. Pre-AT-SPI v0.2 retained a vala-panel-appmenu daemon as
      the menu registrar. v0.3.0 retired DBusMenu in favour of AT-SPI
      (ADR-0024) and this option no longer affects menu rendering. The
      option is preserved for one cycle (v1.0.0) so existing configs do
      not eval-error; setting it to a non-default value emits a warning.
      Will be removed entirely in v1.1.
    '';
  };

  hideInWindowMenubar = mkOption {
    type = types.bool;
    default = false;
    description = ''
      Hide the in-window menubar of Qt and GTK apps so the global menu
      in the bar is the only menu surface. Under the AT-SPI substrate
      (v0.3.0+) this is achieved via per-toolkit hints, not by setting
      QT_QPA_PLATFORMTHEME=appmenu-qt5 or GTK_MODULES=appmenu-gtk-module
      (those settings have no effect under AT-SPI and are no longer
      written when this option is true).
    '';
  };

  widgetPlacement = mkOption {
    type = types.nullOr (types.enum [ "left" "right" ]);
    default = null;
    description = ''
      If set, the HM module writes the appropriate noctalia bar slot
      assignment. If null, the user manages the slot manually through
      noctalia's own UI.
    '';
  };
};
```

## New unconditional writes (when `enable = true`)

```nix
home.sessionVariables = {
  QT_ACCESSIBILITY = "1";        # FR-014: Qt registers a11y trees even without an active screen reader
};

# Asserted prerequisites
assertions = [
  {
    assertion = config.services.gnome.at-spi2-core.enable or false;
    message = ''
      programs.noctalia.plugins.appmenu requires the AT-SPI daemon.
      Enable it system-wide on NixOS:

          services.gnome.at-spi2-core.enable = true;
    '';
  }
];
```

> **Note:** `assertions` cannot fire in pure HM-on-non-NixOS scenarios. The module emits a `lib.warn` when `assertions` is unavailable (per HM's `assertions` evaluation rules).

## Removed in v1.0.0

- `noctalia-appmenu-registrar.service` systemd user unit (was created when `registrar = "vala-panel"`). The unit is no longer installed by default; the `vala-panel-appmenu` package is no longer pulled into `home.packages`.
- `home.sessionVariables.QT_QPA_PLATFORMTHEME = "appmenu-qt5"` (no longer written under any option setting).
- `home.sessionVariables.GTK_MODULES = "appmenu-gtk-module"` (no longer written).

## Plugin discovery (FR-020)

`xdg.configFile."noctalia/plugins/noctalia-appmenu".source = ${packages.plugin}/share/noctalia-shell/plugins/noctalia-appmenu` — unchanged in shape; verified to actually load via either:

1. directory scanning by noctalia-shell's loader (preferred — no `plugins.json` write needed), OR
2. explicit `xdg.configFile."noctalia/plugins.json"` write — only if directory-scanning is confirmed insufficient by reading noctalia-shell's loader source.

Lane C's spec resolves this via a small probe step at planning time.

## Migration guidance (users upgrading from v0.3.0 → v1.0.0)

| What changed | Action required |
|---|---|
| `registrar = "vala-panel"` deprecated | Remove the line OR set to `"none"`. Plugin keeps working. |
| `vala-panel-appmenu` no longer installed by HM module | If you relied on it for other apps, install via your system config explicitly. |
| `services.gnome.at-spi2-core.enable = true` now required | Add this to your NixOS config. HM emits an assertion at rebuild time if missing. |

## Test contract

- `nix flake check` evaluates the module against four scenarios (cartesian product of `enable = {true,false}` × `services.gnome.at-spi2-core.enable = {true,false}`) and asserts the correct assertion/warning fires.
- A documented "Verify the install" recipe in the README (FR-028) exercises the happy path end-to-end on a fresh NixOS host.
