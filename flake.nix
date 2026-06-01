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

        # Keep insta `.snap` golden fixtures (bridge/tests/snapshots/*.snap)
        # in the build source. `cleanCargoSource` keeps only the Cargo/.rs
        # set and strips `.snap`, which makes the headless snapshot tests
        # (bridge/tests/atspi_integration.rs) fail in the Nix sandbox with
        # "no snapshot found" (all-`+` diff). `filterCargoSources` is the
        # filter `cleanCargoSource` wraps; the suffix clause re-adds goldens.
        bridgeSrc = pkgs.lib.cleanSourceWith {
          src = ./bridge;
          name = "bridge-src";
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (pkgs.lib.hasSuffix ".snap" path);
        };

        # FR-018: single source-of-truth for the package version. Read
        # once from bridge/Cargo.toml; both bridge + plugin derivations
        # consume it. Bumping the Cargo manifest is enough — the flake
        # follows automatically and a divergence is impossible.
        cargoToml = pkgs.lib.importTOML ./bridge/Cargo.toml;
        version = cargoToml.package.version;

        # FR-019: SOURCE_DATE_EPOCH injected from outside the sandbox.
        # self.lastModified is the flake's last-modified timestamp set
        # by Nix from the working tree (and overridable by the release
        # workflow via env-var on the calling shell). Replaces the
        # previous in-sandbox `git log` shellout, which required
        # `pkgs.git` at build time and silently degraded to a hardcoded
        # 2025-01-01 fallback when git was unavailable. Now fully
        # deterministic and pure-eval safe.
        sourceDateEpoch = toString inputs.self.lastModified;

        bridge = craneLib.buildPackage {
          pname = "noctalia-appmenu-bridge";
          inherit version;
          src = bridgeSrc;
          cargoExtraArgs = "--locked";

          nativeBuildInputs = [pkgs.pkg-config];
          buildInputs = [];

          SOURCE_DATE_EPOCH = sourceDateEpoch;

          meta = with pkgs.lib; {
            description = "Sidecar bridge for noctalia-appmenu (niri-IPC focus tracker + AT-SPI menubar walker + active-app menu mirror)";
            homepage = "https://github.com/yolo-labz/noctalia-appmenu";
            license = licenses.asl20;
            mainProgram = "noctalia-appmenu-bridge";
            maintainers = [];
          };
        };

        plugin = pkgs.stdenvNoCC.mkDerivation {
          pname = "noctalia-appmenu-plugin";
          inherit version;
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
