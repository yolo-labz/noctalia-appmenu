# Changelog

All notable changes to noctalia-appmenu are documented here. Format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and adheres to
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features

- Initial scaffold: bridge + plugin + Nix module + CI/CD + speckit constitution.

### Bug fixes

- Bridge hard-fails at startup when `niri msg windows` is unreachable, instead
  of silently rendering an invisible menubar (audit P1).
- `default_config_path()` no longer falls back to `/tmp` when both
  `XDG_CONFIG_HOME` and `HOME` are unset; refuses with a clear error
  (audit P0).
- Registrar bus-name parsing now reports the offending name on failure
  (audit P1).

### Refactor

- `niri::handle_event` extracted as a pure transducer (cache-read, op-emit)
  so the schema-drift path is unit-testable without spawning `niri`.
- `active::snapshot` extracted as a pure reducer.
- `proxy::ActiveProxy` switched from `RwLock` to `Mutex` (no concurrent
  readers; lower acquisition cost).
- Named constants for `FOCUS_DEBOUNCE_DEFAULT_MS`, `REGISTRAR_DEBOUNCE_DEFAULT_MS`,
  `PUBLISH_SERVICE_DEFAULT`, `PUBLISH_PATH_DEFAULT`.

### Tests

- 14 new unit tests across `niri::tests`, `config::tests`, `active::tests`
  (was 1 integration test only).

### Tooling

- `cargo-deny` config (advisories / bans / licenses / sources).
- `typos` ecosystem allow-list.
- `semgrep` 5 custom Rust rules (no-unwrap-outside-tests, no-raw-env-var,
  tokio-spawn-must-handle-error, no-println-in-lib, no-todo-fixme).
- `gitleaks` 5 explicit rules (github PAT fine-grained, sonar token,
  Tailscale MagicDNS, internal LAN IPs, generic bearer).
- New CI workflows: `cargo-deny`, `typos`, `cargo-machete`, `dco`, `semgrep`.
- `.vscode/`, `.helix/` editor configs (force-added past the user's
  global git ignore).
- `.github/release.yml` grouped release-notes config.
- Expanded `justfile`: `audit`, `unused-deps`, `typos`, `semgrep`, `sbom`,
  `gitleaks`, `loc`, `ci-local`, `release-dry-run`, `verify-release`,
  `bridge-bench`, `bridge-flame`, `qmlformat`.
- `lib.rs`: `#![forbid(unsafe_code)]`, intra-doc-link warnings, ADR
  cross-references in module-level docs.

[Unreleased]: https://github.com/yolo-labz/noctalia-appmenu/compare/HEAD...HEAD
