#!/usr/bin/env bash
# verify-tag-subject.sh — pre-push guard against v1.0.14-style tag drift.
#
# Refuses a `git push` if any *local-only* tag `vX.Y.Z` points at a commit
# whose subject line does NOT mention `vX.Y.Z` (or `X.Y.Z`).
#
# This is CLAUDE.md drift trigger E enforced at the git layer: in
# May 2026 we tagged v1.0.14 on the v1.0.13 commit because the maintainer
# (an AI agent) ran `git tag` from the wrong worktree. Static check that
# would have caught it: "the commit message of the tagged commit should
# mention the tag's version".
#
# Implementation note: we deliberately do NOT read stdin (pre-push refs
# list), because lefthook's `parallel: true` mode races stdin across
# commands. Instead we enumerate local tags that origin does not yet
# have — that is the set of tags about to be pushed.
#
# Exit codes:
#   0 — every local-only `v*` tag points at a commit whose subject mentions the version
#   1 — at least one mismatch detected; push must be aborted
set -euo pipefail

# Refresh origin's tag list non-destructively. Quiet because lefthook
# inlines stdout/stderr into the user's terminal.
git fetch --tags --quiet origin 2>/dev/null || true

failed=0
checked=0

while IFS= read -r tag; do
    [[ -z "$tag" ]] && continue
    # Skip if origin already has the tag (this push is not introducing it).
    if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
        continue
    fi

    checked=$((checked + 1))
    version="${tag#v}"
    target="$(git rev-parse "$tag^{commit}")"
    subject="$(git log -1 --pretty=%s "$target")"

    if [[ "$subject" == *"$version"* ]] || [[ "$subject" == *"$tag"* ]]; then
        echo "[verify-tag-subject] OK   $tag → $target (\"$subject\")"
    else
        cat >&2 <<EOF
[verify-tag-subject] FAIL: $tag points at a commit that does not
mention "$version" in its subject line.

  Commit:  $target
  Subject: $subject

This is the v1.0.14 drift trigger (CLAUDE.md trigger E).

Fix:
  1. Find the correct squash-merge commit:
       git log --oneline origin/main | grep -F '$version'
  2. Move the tag:
       git tag -d $tag
       git tag -a $tag <correct-sha> -m "Release $tag"
  3. Re-run the push.

Bypass is FORBIDDEN by CLAUDE.md hard ban #3 — do not use \`--no-verify\`.
EOF
        failed=1
    fi
done < <(git tag --list 'v*')

if (( checked == 0 )); then
    : # silent — no tags introduced by this push
fi

exit "$failed"
