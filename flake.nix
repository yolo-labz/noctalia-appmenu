{
  description = "noctalia-appmenu — macOS-style global menu for noctalia on niri (Qt6 + GTK, Wayland)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux"];

      perSystem = {
        config,
        pkgs,
        system,
        ...
      }: let
        rustPkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [inputs.rust-overlay.overlays.default];
        };

        rustToolchain = rustPkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "clippy" "rustfmt" "llvm-tools-preview"];
        };

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        bridgeSrc = craneLib.cleanCargoSource ./bridge;

        bridge = craneLib.buildPackage {
          pname = "noctalia-appmenu-bridge";
          version = "0.1.0";
          src = bridgeSrc;
          cargoExtraArgs = "--locked";

          nativeBuildInputs = [pkgs.pkg-config];
          buildInputs = [];

          # Reproducibility
          preBuild = ''
            export SOURCE_DATE_EPOCH=$(${pkgs.git}/bin/git log -1 --format=%ct 2>/dev/null || echo 1735689600)
          '';

          meta = with pkgs.lib; {
            description = "Sidecar bridge for noctalia-appmenu (niri-IPC + DBusMenu registrar consumer + active-app proxy)";
            homepage = "https://github.com/yolo-labz/noctalia-appmenu";
            license = licenses.asl20;
            mainProgram = "noctalia-appmenu-bridge";
            maintainers = [];
          };
        };

        plugin = pkgs.stdenvNoCC.mkDerivation {
          pname = "noctalia-appmenu-plugin";
          version = "0.1.0";
          src = ./plugin;
          dontBuild = true;
          installPhase = ''
            mkdir -p $out/share/noctalia-shell/plugins/noctalia-appmenu
            cp -r . $out/share/noctalia-shell/plugins/noctalia-appmenu/
          '';
          meta.license = pkgs.lib.licenses.asl20;
        };
      in {
        packages = {
          inherit bridge plugin;
          noctalia-appmenu-bridge = bridge;
          noctalia-appmenu-plugin = plugin;
          default = bridge;
        };

        checks = {
          inherit bridge;
          bridge-clippy = craneLib.cargoClippy {
            inherit (bridge) pname version;
            src = bridgeSrc;
            cargoArtifacts = craneLib.buildDepsOnly {src = bridgeSrc;};
            cargoClippyExtraArgs = "--all-features --all-targets -- --deny warnings";
          };
          bridge-fmt = craneLib.cargoFmt {
            inherit (bridge) pname version;
            src = bridgeSrc;
          };
          bridge-test = craneLib.cargoTest {
            inherit (bridge) pname version;
            src = bridgeSrc;
            cargoArtifacts = craneLib.buildDepsOnly {src = bridgeSrc;};
          };
        };

        devShells.default = craneLib.devShell {
          inherit (config) checks;
          packages = with pkgs; [
            # Rust tooling beyond toolchain
            cargo-llvm-cov
            cargo-cyclonedx
            cargo-deny
            cargo-machete
            cargo-nextest

            # Nix tooling
            alejandra
            statix
            deadnix

            # Git / quality
            lefthook
            gitleaks
            commitlint
            actionlint
            zizmor
            typos
            semgrep

            # QML / Qt — qmllint ships with qttools
            qt6.qtdeclarative
            qt6.qttools

            # niri (for integration tests + manual smoke)
            niri

            # Docs
            mdbook

            # Helpers
            just
            ripgrep
            jq
            yq-go
            gh
            git-cliff
            tokei

            # Fake registrar tooling
            python3
            python3Packages.dbus-python
            python3Packages.pygobject3

            # D-Bus debugging — busctl ships with systemd; glib
            # provides gdbus. Quickshell `qs` CLI is upstream-only
            # at git.outfoxxed.me/quickshell — not in nixpkgs;
            # users add it via overlay if they need it for plugin
            # live-reload tests.
            glib
          ];

          shellHook = ''
            export RUST_LOG=noctalia_appmenu_bridge=debug,zbus=info
            echo "noctalia-appmenu devShell — $(rustc --version)"
          '';
        };

        formatter = pkgs.alejandra;
      };

      flake = {
        homeManagerModules.default = import ./nix/module.nix inputs.self;
      };
    };
}
