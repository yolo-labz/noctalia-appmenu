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
    enable = lib.mkEnableOption "noctalia-appmenu (macOS-style global menu)";

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
        '';
      };
    };

    registrar = lib.mkOption {
      type = lib.types.enum ["vala-panel" "none"];
      default = "vala-panel";
      description = ''
        Which registrar daemon to install + run.
        - "vala-panel": pkgs.vala-panel-appmenu's appmenu-registrar (default).
        - "none": user provides their own.
      '';
    };

    hideInWindowMenubar = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        When true, set QT_QPA_PLATFORMTHEME=appmenu-qt5 and
        GTK_MODULES=appmenu-gtk-module in the user's session env so
        Qt6 / GTK apps export their menubars over D-Bus instead of
        rendering them in-window.
      '';
    };

    # `widgetPlacement` is declared but currently NOT consumed —
    # bar widget placement lives in the user's noctalia-shell
    # `settings.json` (Settings.data.bar.widgets.<section>) and
    # noctalia-shell's HM module (not this one) owns serialising it.
    # Cross-module config rewrite is out of scope for v0.1; users add
    # the widget id directly to their bar config:
    #     bar.widgets.left = [
    #       { id = "plugin:noctalia-appmenu"; }
    #     ];
    # Kept as an option for forward-compat — wiring lands in v0.2
    # alongside the DBusMenu mirror so the plugin can drive bar
    # placement declaratively from one home-manager module.
    widgetPlacement = lib.mkOption {
      type = lib.types.enum ["left" "center" "right"];
      default = "left";
      description = ''
        Reserved for v0.2: which noctalia bar section the AppMenu
        widget lives in. v0.1 ignores this — add
        `{ id = "plugin:noctalia-appmenu"; }` to your noctalia-shell
        bar.widgets list directly.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages =
      [
        cfg.package
        cfg.bridge.package
      ]
      ++ lib.optionals (cfg.registrar == "vala-panel") [
        pkgs.vala-panel-appmenu
        pkgs.appmenu-gtk-module
      ];

    # Plugin payload landed in user's noctalia plugins dir.
    xdg.configFile."noctalia/plugins/noctalia-appmenu" = {
      source = "${cfg.package}/share/noctalia-shell/plugins/noctalia-appmenu";
      recursive = true;
    };

    # Bridge config — emit via pkgs.formats.toml's generator (handles
    # quoting + types correctly). `lib.generators.toTOML` does not
    # exist in nixpkgs (the formatter lives in `pkgs.formats`).
    xdg.configFile."noctalia-appmenu-bridge/config.toml".source = let
      defaults = {
        focus_debounce_ms = 75;
        registrar_debounce_ms = 250;
        niri_binary = "${pkgs.niri}/bin/niri";
        publish_service = "org.noctalia.AppMenu";
        publish_path = "/org/noctalia/AppMenu/Active";
      };
      merged = defaults // cfg.bridge.config;
    in
      (pkgs.formats.toml {}).generate "noctalia-appmenu-bridge.toml" merged;

    # systemd --user units
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

    systemd.user.services.noctalia-appmenu-registrar = lib.mkIf (cfg.registrar == "vala-panel") {
      Unit = {
        Description = "vala-panel-appmenu Registrar";
        After = ["graphical-session.target"];
        PartOf = ["graphical-session.target"];
      };
      Service = {
        ExecStart = "${pkgs.vala-panel-appmenu}/libexec/vala-panel-appmenu/appmenu-registrar";
        Restart = "on-failure";
        RestartSec = "2s";
      };
      Install = {WantedBy = ["graphical-session.target"];};
    };

    # Session env for menubar export (opt-in)
    home.sessionVariables = lib.mkIf cfg.hideInWindowMenubar {
      QT_QPA_PLATFORMTHEME = "appmenu-qt5";
      GTK_MODULES = "appmenu-gtk-module";
    };
  };
}
