# ADR-0026 — CycloneDX 1.6 in v1.0.0-rc.x releases (syft constraint)

- **Status:** accepted
- **Date:** 2026-05-12
- **Deciders:** Pedro H S Balbino
- **Supersedes:** none
- **Amends:** FR-021 of spec 004-project-completion
- **Tracking PR / branch:** `79-sbom-cdx16` (this ADR's PR)

## Context

Spec 004 §FR-021 + the yolo-labz release-engineering standard mandate **CycloneDX 1.7** SBOMs alongside SPDX 2.3, attested via `actions/attest-sbom`. v0.3.0 shipped CycloneDX 1.6 by accident (Lane D's PR #76 fixed the workflow to emit 1.7 and updated the attestation step's claim).

The `v1.0.0-rc.1` release workflow ran on 12/05/2026 19:36 UTC and failed at the SBOM step:

```
[0000] ERROR 1 error occurred:
        * unsupported output format "cyclonedx-json@1.7", supported formats are:
   - cyclonedx-json @ 1.2, 1.3, 1.4, 1.5, 1.6
   - cyclonedx-xml @ 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6
```

Investigation:

- **syft v1.43.0** (pinned in `release.yml`): max CycloneDX is 1.6.
- **syft v1.44.0** (latest at 2026-05-12, released 2026-05-01): also max 1.6. No 1.7 support shipped.
- **CycloneDX 1.7 spec**: released September 2025 by the OWASP CycloneDX project. Schema diffs from 1.6 are minor (added `compositions.assemblyAggregate`, refined `lifecycles`); upstream syft has not yet adopted.

## Decision

For all `v1.0.0-rc.x` and `v1.0.0` releases, emit **CycloneDX 1.6** alongside SPDX 2.3. Continue to comply with every other clause of the yolo-labz release-engineering standard (dual SBOM, GitHub-native attestation, `gh attestation verify` parity, byte-identical reproducibility, no re-tag of any release).

Revisit when **syft v1.45+** ships `cyclonedx-json @ 1.7`. The transition is a one-line workflow edit + attestation step rename + an ADR superseding this one.

## Consequences

### Positive

- The `v1.0.0-rc.x` releases ship without further delay.
- Attestations remain valid + verifiable by `gh attestation verify`.
- The verification step asserts `specVersion=1.6` so a future syft upgrade that silently flips to 1.7 cannot ship a 1.6-claimed-as-1.7 artefact.

### Negative

- Constitution §VI says the standard is non-negotiable. This is a documented exception, not a quiet deviation.
- Downstream consumers that gate on CycloneDX 1.7 specifically (none known) will fail. None of our published consumers do.
- Carries a v1.0.x debt: a future minor release that flips to 1.7 needs an ADR update + workflow edit + release rerun.

### Wire-level details

- `cyclonedx-json @ 1.6` is published October 2024 (per CycloneDX spec history). All major SBOM consumers (Dependency-Track, OWASP's reference impl, GitHub's SBOM ingestion) accept 1.6.
- SPDX 2.3 stays unchanged.
- The `Verify CycloneDX SBOM is X.Y` step asserts the format matches, so a syft-side change is caught at release-cut time, not by downstream.

## Migration plan

1. **Now:** patch `release.yml` to emit `cyclonedx-json@1.6`; rename "Verify CycloneDX SBOM is 1.7" → "1.6" and the assert; rename "Attest SBOM (CycloneDX 1.7)" → "1.6".
2. **Re-cut:** new tag `v1.0.0-rc.2` (constitution: no re-tag).
3. **Monitor:** weekly check of `gh release list --repo anchore/syft` for v1.45+ release notes mentioning CycloneDX 1.7.
4. **Promote:** when syft ships 1.7, open PR superseding this ADR, bump release.yml to `cyclonedx-json@1.7`, cut the next patch release with the new format.

## References

- [syft releases](https://github.com/anchore/syft/releases)
- [CycloneDX 1.7 specification](https://cyclonedx.org/specification/overview/)
- Constitution §VI (release-engineering compliance, non-negotiable; this ADR is the documented exception per the §VI clause)
- Spec 004 §FR-021 (relaxed by this ADR)
- ADR-0024 (substrate, unrelated but referenced for completeness)
