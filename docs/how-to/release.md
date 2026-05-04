# Releasing

Hands-on for cutting `vX.Y.Z` of `yolo-labz/noctalia-appmenu`.

## Prerequisites

- All in-flight PRs merged or held back.
- Repository ruleset is in `active` enforcement (not the `disabled` bootstrap mode).
- `main` is at the commit you want to release; CI green.

## Tag and push

```bash
cd ~/Documents/Code/noctalia-appmenu
git fetch origin && git pull --ff-only origin main

VERSION=0.1.0
just release-tag $VERSION       # asserts clean tree + HEAD == origin/main, then signs the tag
git push origin v$VERSION
```

The push of `v$VERSION` triggers `.github/workflows/release.yml`:

1. Runs `step-security/harden-runner` (egress: audit).
2. Builds the bridge via `nix build .#noctalia-appmenu-bridge`.
3. Builds the plugin tarball with reproducible `tar` flags.
4. Generates SBOMs via `syft` (CycloneDX 1.7 + SPDX 2.3) and `cargo cyclonedx` (Rust-native CycloneDX).
5. Computes `dist/checksums.txt`.
6. Calls `actions/attest-build-provenance@v4.1.0` against the checksums.
7. Calls `actions/attest-sbom@v4.1.0` once per SBOM format.
8. Generates `CHANGELOG.md` via `git-cliff` from Conventional Commits.
9. Creates the GitHub release with all artefacts attached.

## Verify

End-user verification recipe (also in [SECURITY.md](../../SECURITY.md)):

```bash
gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz
```

## If the release botched

**Don't re-tag**. Cut `vX.Y.Z+1`. The yolo-labz invariant is non-negotiable — `slsa-verifier` validates against the commit SHA at signing time, and re-tagging produces stale provenance.

```bash
# Fix the underlying issue, merge to main, then:
just release-tag 0.1.1
git push origin v0.1.1
```

## Hand-edits to CHANGELOG.md

Forbidden. `git-cliff` owns the file. Edit commits, not the changelog.

## Pre-release tags

`v0.1.0-rc.1`, `v0.1.0-beta.1`, etc. The release workflow's preflight gate classifies these as prerelease and silently skips secrets it would otherwise loud-fail on. Useful for dry-running.
