self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.noctalia.plugins.appmenu;
  bridgePkg = self.packages.${pkgs.system}.noctalia-appmenu-bridge;
  pluginPkg = self.packages.${pkgs.system}.noctalia-appmenu-plugin;
in {
  options.programs.noctalia.plugins.appmenu = {
    enable = lib.mkEnableOption "noctalia-appmenu (macOS-style global menu — AT-SPI substrate, ADR-0024)";

    package = lib.mkOption {
      type = lib.types.package;
      default = pluginPkg;
      description = "Plugin payload derivation (manifest.json + QML).";
    };

    bridge = {
      package = lib.mkOption {
        type = lib.types.package;
        default = bridgePkg;
        description = "Bridge daemon derivation.";
      };

      niriPackage = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        example = lib.literalExpression "pkgs.niri";
        description = ''
          Explicit niri client package to use for `niri msg event-stream`
          / `niri msg windows`. When null (default) the bridge invokes
          bare `niri` (PATH-resolved from the systemd user manager's
          PATH, which includes `/run/current-system/sw/bin` on NixOS) —
          this means the bridge follows whatever niri version the
          system has installed, not the niri pinned in this flake's
          nixpkgs input.

          **Why default null (PATH):** the bridge's flake pins niri
          via its own `nixpkgs` input. When that input lags the system
          niri (e.g. flake at 25.11, system at 26.04), the pinned client
          crashes parsing newer compositor events (`CastsChanged` etc).
          PATH resolution avoids version drift. PR #46's respawn loop
          tolerates the crash but the spam fills journalctl every 30s.

          Set explicitly to override (e.g. for testing a specific niri
          version, or when the user lacks niri on PATH).
        '';
      };

      config = lib.mkOption {
        type = lib.types.attrsOf lib.types.anything;
        default = {};
        description = ''
          TOML config merged into noctalia-appmenu-bridge. Schema:

            focus_debounce_ms       = 75
            registrar_debounce_ms   = 250
            niri_binary             = "niri"
            publish_service         = "org.noctalia.AppMenu"
            publish_path            = "/org/noctalia/AppMenu/Active"

          User keys here override the defaults (including
          `niri_binary` if `niriPackage` is left null).
        '';
      };
    };

    registrar = lib.mkOption {
      type = lib.types.enum ["vala-panel" "none"];
      default = "none";
      description = ''
        DEPRECATED in v1.0.0 — will be removed entirely in v1.1.

        Pre-AT-SPI (v0.2 and earlier) installed `vala-panel-appmenu`'s
        `appmenu-registrar` daemon to host the DBusMenu registrar that
        Qt/GTK apps registered against. v0.3.0 retired DBusMenu in
        favour of an AT-SPI menubar walker (ADR-0024); the registrar
        process is no longer required and `vala-panel-appmenu` /
        `appmenu-gtk-module` are no longer pulled into `home.packages`.

        Setting this option to a non-default value ("vala-panel")
        emits a warning at evaluation time. The intended migration
        path is to drop the line from your config (the default —
        "none" — is the right value for AT-SPI).

        Removal timeline: v1.1 will remove the option declaration; a
        config that still sets it then will fail with an unknown-option
        error.
      '';
    };

    hideInWindowMenubar = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Hide the in-window menubar of Qt and GTK apps so the noctalia
        bar surface is the only menu surface.

        Under the AT-SPI substrate (v0.3.0+, ADR-0024) the in-window
        suppression is cooperative behaviour driven by the toolkit-side
        Qt platform theme (e.g. `qt.platformTheme.name = "kde"`) and is
        no longer driven by env-var writes from this module. The v0.2-era
        writes of `QT_QPA_PLATFORMTHEME = "appmenu-qt5"` and
        `GTK_MODULES = "appmenu-gtk-module"` have been removed — both
        targeted DBusMenu only and have no effect on the AT-SPI walker.

        The option is preserved for config compatibility; setting it is
        a no-op in v1.0.0 and may drive a per-toolkit hint in a future
        release. Removal scheduled alongside `registrar` in v1.1.
      '';
    };

    # `widgetPlacement` is declared but currently NOT consumed —
    # bar widget placement lives in the user's noctalia-shell
    # `settings.json` (Settings.data.bar.widgets.<section>) and
    # noctalia-shell's HM module (not this one) owns serialising it.
    # Cross-module config rewrite is out of scope for v1.0.0; users add
    # the widget id directly to their bar config:
    #     bar.widgets.left = [
    #       { id = "plugin:noctalia-appmenu"; }
    #     ];
    # Kept as an option for forward-compat — wiring lands in a later
    # release alongside an upstream noctalia-shell change that allows
    # bar placement to be driven declaratively from one HM module.
    widgetPlacement = lib.mkOption {
      type = lib.types.enum ["left" "center" "right"];
      default = "left";
      description = ''
        Reserved for a future release: which noctalia bar section the
        AppMenu widget lives in. v1.0.0 ignores this — add
        `{ id = "plugin:noctalia-appmenu"; }` to your noctalia-shell
        `bar.widgets.<section>` list directly, alongside enabling the
        plugin via `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true`.
      '';
    };
  };

  # FR-015 / FR-016: deprecation + AT-SPI prerequisite warnings.
  #
  # NOTE (ADR-0027): the previous implementation referenced `osConfig`
  # to probe `services.gnome.at-spi2-core.enable` from inside an
  # eval-time `lib.warnIf`. When the module is loaded via
  # `home-manager.users.<u>.imports` (the standard NixOS+HM
  # composition), reading `osConfig` from a top-level binding triggers
  # the surrounding NixOS module merge to evaluate
  # `home-manager.users.<u>._module.freeformType`, which in turn needs
  # the value of every option in the user's HM scope — including this
  # one. Result: infinite recursion at eval time (reproduced on Pedro's
  # desktop 12/05/2026 against `nh os switch`).
  #
  # The fix is to keep all conditional logic inside `config = lib.mkIf
  # cfg.enable {...}` (which the module system evaluates lazily on the
  # `enable` flag) and surface the AT-SPI prerequisite via a
  # `warnings` entry instead of `lib.warnIf`. `warnings` is a builtin
  # HM/NixOS option whose value is collected from all modules and
  # printed at activation time, with no eager-eval coupling to the
  # surrounding NixOS config.
  config = lib.mkIf cfg.enable {
    # FR-016: home.packages drops vala-panel-appmenu +
    # appmenu-gtk-module. They were dead post-ADR-0024 but the
    # v0.3.0 module still pulled them in unconditionally when the
    # `registrar` option was at its default.
    home.packages = [
      cfg.package
      cfg.bridge.package
    ];

    # FR-014: QT_ACCESSIBILITY=1 unconditional when the plugin is
    # enabled. Qt only registers its accessibility tree with the
    # a11y bus when this var is set in the user session; without
    # it, the AT-SPI walker sees an empty tree and the bar menu
    # silently stays empty.
    #
    # FR-017: the v0.2-era writes of `QT_QPA_PLATFORMTHEME =
    # "appmenu-qt5"` and `GTK_MODULES = "appmenu-gtk-module"`
    # (previously gated by `hideInWindowMenubar`) have been removed
    # — both targeted DBusMenu and have no effect under AT-SPI.
    home.sessionVariables = {
      QT_ACCESSIBILITY = "1";
    };

    # FR-015 (per ADR-0027): activation-time AT-SPI prerequisite
    # surfaced via `warnings`, NOT `lib.warnIf` and NOT
    # `assertions`. Both of the latter trigger eager evaluation
    # of the surrounding NixOS config when this HM module is
    # nested inside `home-manager.users.<u>.imports`, producing
    # an infinite-recursion freeformType cycle. `warnings` is
    # collected at activation time, after the merge resolves, so
    # it does not participate in the cycle.
    #
    # The user is expected to set
    # `services.gnome.at-spi2-core.enable = true` system-wide;
    # without it, Qt/GTK apps register no a11y tree and the bar
    # silently shows the synthetic `.desktop`-derived menu only.
    # The warning is also restated in the README's "Verify the
    # install" section (FR-028).
    warnings = lib.optional (cfg.registrar != "none") ''
      programs.noctalia.plugins.appmenu.registrar = "${cfg.registrar}"
      is DEPRECATED in v1.0.0 and will be removed in v1.1. The
      vala-panel-appmenu registrar daemon is dead under the
      AT-SPI substrate (ADR-0024); drop the option from your
      config (default is "none").
    '';

    # FR-020: install the plugin manifest directory. noctalia-shell's
    # PluginService gates plugin load on
    # `plugins.json::states.<id>.enabled = true` — verified against
    # the upstream HM module's contract (see
    # `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled`).
    # That file is owned by the upstream HM option and CANNOT be
    # written from here without a single-writer conflict, so users
    # must add the enable entry themselves alongside this option:
    #
    #     programs.noctalia.plugins.appmenu.enable = true;
    #     programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true;
    #
    # Users wiring noctalia-shell some other way must add the entry
    # to ~/.config/noctalia/plugins.json manually.
    xdg.configFile."noctalia/plugins/noctalia-appmenu" = {
      source = "${cfg.package}/share/noctalia-shell/plugins/noctalia-appmenu";
      recursive = true;
    };

    # Bridge config — emit via pkgs.formats.toml's generator
    # (handles quoting + types correctly). `lib.generators.toTOML`
    # does not exist in nixpkgs (the formatter lives in
    # `pkgs.formats`).
    xdg.configFile."noctalia-appmenu-bridge/config.toml".source = let
      # niri_binary default behaviour:
      # - cfg.bridge.niriPackage == null (default): use bare
      #   `niri`, PATH-resolved from systemd user manager
      #   (includes /run/current-system/sw/bin on NixOS). This
      #   tracks whatever niri version the host has installed →
      #   no flake-input drift.
      # - cfg.bridge.niriPackage set: use that store-path
      #   explicitly.
      niriBinary =
        if cfg.bridge.niriPackage != null
        then "${cfg.bridge.niriPackage}/bin/niri"
        else "niri";
      defaults = {
        focus_debounce_ms = 75;
        registrar_debounce_ms = 250;
        niri_binary = niriBinary;
        publish_service = "org.noctalia.AppMenu";
        publish_path = "/org/noctalia/AppMenu/Active";
      };
      merged = defaults // cfg.bridge.config;
    in
      (pkgs.formats.toml {}).generate "noctalia-appmenu-bridge.toml" merged;

    # FR-016: `noctalia-appmenu-registrar` systemd user unit
    # removed. The unit ran vala-panel-appmenu's
    # `appmenu-registrar` and was dead code post-ADR-0024. The
    # bridge's own systemd user unit is the only daemon installed
    # under v1.0.0+.
    systemd.user.services.noctalia-appmenu-bridge = {
      Unit = {
        Description = "noctalia-appmenu sidecar bridge";
        After = ["graphical-session.target"];
        PartOf = ["graphical-session.target"];
      };
      Service = {
        ExecStart = "${cfg.bridge.package}/bin/noctalia-appmenu-bridge";
        Restart = "on-failure";
        RestartSec = "2s";

        # Hardening (per SECURITY.md)
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectControlGroups = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        RestrictAddressFamilies = "AF_UNIX";
        RestrictNamespaces = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
      };
      Install = {WantedBy = ["graphical-session.target"];};
    };
  };
}
