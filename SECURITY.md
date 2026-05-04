# Security policy

## Reporting

Report vulnerabilities through GitHub's private vulnerability advisory flow:

**https://github.com/yolo-labz/noctalia-appmenu/security/advisories/new**

Do not file public issues for security problems.

We aim to triage within 7 days and ship a fix within 30 days for high/critical issues. PGP keys are intentionally not provided — the GitHub PVR flow is end-to-end encrypted and audit-logged.

## Supported versions

Only the latest minor release on the `main` branch receives security fixes. We do not backport to older releases — if you cannot update, pin the latest tag and apply the fix yourself.

## Supply-chain posture

Releases are produced by the GitHub Actions self-hosted runner on `vm103.home302server` (Proxmox), gated by Repository Rulesets requiring CodeQL, OSV-Scanner, OpenSSF Scorecard, SonarQube, and reproducibility checks to pass before merge to `main`.

Each release artefact ships:

- **Build provenance** via `actions/attest-build-provenance@v2` — verifiable with `gh attestation verify <artefact> --owner yolo-labz`.
- **SBOMs** in two formats:
  - CycloneDX 1.7 (`*.cdx.json`)
  - SPDX 2.3 (`*.spdx.json`)
- **Sigstore-keyless cosign** signature on every binary blob.
- **Reproducible build** — the bridge crate is built with `SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)` and `RUSTFLAGS="-C link-arg=-Wl,--build-id=none"`. The `reproducibility.yml` workflow rebuilds the artefact from a clean checkout and asserts byte-equality with the release blob.

Verify before manual installation:

```bash
# 1. Provenance
gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz

# 2. Cosign signature (fallback / offline)
cosign verify-blob \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate-identity-regexp '^https://github\.com/yolo-labz/noctalia-appmenu/' \
  --signature noctalia-appmenu-bridge.sig \
  --bundle noctalia-appmenu-bridge.bundle \
  ./noctalia-appmenu-bridge

# 3. SBOM (any review tool)
syft scan ./noctalia-appmenu-bridge -o cyclonedx-json | diff - sbom.cdx.json
```

## Threat model

### In scope

- A malicious local process attempting to spoof `org.noctalia.AppMenu.Active` and inject menu items into the bar.
- A malicious upstream in `cargo`/`nixpkgs` shipping a tampered version of `zbus`, `niri-ipc`, or one of the tracked actions.
- A bad commit landing on `main` (covered by required-checks ruleset, signed-commits, and CODEOWNERS).

### Out of scope

- The user's display server (niri) being compromised. Anything past the niri socket is trusted.
- Apps the user has chosen to run. We render whatever `com.canonical.dbusmenu` payload the active app published — we do not validate menu content.
- Display-server rooting (XWayland, Pipewire screencast). Standard niri threat model applies.

## Hardening checklist (CI-enforced)

Every release workflow:

- pins every action by full 40-char SHA with trailing `# vX.Y.Z` comment;
- declares `permissions: {}` at workflow level, re-grants per-job;
- runs `step-security/harden-runner` in `audit` mode (flipped to `block` once egress is observed clean);
- declares `timeout-minutes` on every job;
- uses `persist-credentials: false` on `actions/checkout` unless pushing.

Bridge runtime hardening:

- systemd user unit runs with `NoNewPrivileges=true`, `ProtectSystem=strict`, `ProtectHome=read-only`, `RestrictAddressFamilies=AF_UNIX`, `MemoryDenyWriteExecute=true`.
- The bridge does not open listening sockets — it only speaks D-Bus to the user session bus.
- It does not exec subprocesses except `niri msg` (binary path resolved once at startup, not from `$PATH`).

## SCorecard ceiling

Realistic ceiling for this repo is ~8.7/10 — solo-dev contributors check is structurally capped. We accept the loss; full disclosure in this file.
