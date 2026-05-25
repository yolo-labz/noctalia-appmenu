#!/usr/bin/env bash
# Refuse a commit that introduces an ADR file under docs/adr/ without
# matching entries in docs/SUMMARY.md (mdbook sidebar) and
# docs/adr/README.md (in-book ADR index table).
#
# Background: PR #141 (24/05/2026) backfilled 15 ADRs (0016..0030) that
# had landed over the v0.2..v1.0.25 arc but were silently invisible in
# the book navigation. mdbook builds files not listed in SUMMARY, they
# just don't show in the sidebar — silent failure. This guard converts
# silent drift into a loud commit refusal.
#
# Wired into lefthook.yml `pre-commit` so it runs locally before push
# AND in the lefthook stage of CI on PRs.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

declare -a missing_summary=()
declare -a missing_index=()

while IFS= read -r adr_path; do
  adr_file=$(basename "$adr_path")           # ADR-0030-frame-scoped-menu-resolution.md
  adr_id=$(echo "$adr_file" | grep -oE '^ADR-[0-9]+')   # ADR-0030

  # SUMMARY.md uses the relative path adr/<file>
  if ! grep -qF "adr/$adr_file" docs/SUMMARY.md; then
    missing_summary+=("$adr_id ($adr_file)")
  fi

  # docs/adr/README.md uses the bare filename in markdown links: [0030](<file>)
  if ! grep -qF "$adr_file" docs/adr/README.md; then
    missing_index+=("$adr_id ($adr_file)")
  fi
done < <(find docs/adr -maxdepth 1 -name 'ADR-*.md' -type f | sort)

rc=0

if [ ${#missing_summary[@]} -gt 0 ]; then
  echo "ERROR: ADR file(s) on disk but missing from docs/SUMMARY.md:" >&2
  printf '  - %s\n' "${missing_summary[@]}" >&2
  echo "Fix: add a '- [ADR-NNNN — Title](adr/<file>)' bullet under the 'Architecture decision records' section." >&2
  rc=1
fi

if [ ${#missing_index[@]} -gt 0 ]; then
  echo "ERROR: ADR file(s) on disk but missing from docs/adr/README.md:" >&2
  printf '  - %s\n' "${missing_index[@]}" >&2
  echo "Fix: add a '| [NNNN](<file>) | Title | Status |' row to the Index table." >&2
  rc=1
fi

if [ "$rc" -eq 0 ]; then
  count=$(find docs/adr -maxdepth 1 -name 'ADR-*.md' -type f | wc -l)
  echo "verify-adr-summary: $count ADR(s) accounted for in SUMMARY + README."
fi

exit "$rc"
