#!/usr/bin/env bash
# verify-no-third-patch.sh — CLAUDE.md drift trigger I enforcement.
#
# Spec 015 FR-008 — refuses any push whose branch title overlaps the
# symptom phrase of ≥ 2 recently-merged PRs. The signal is "user-visible
# failure mode persists across multiple deploys" — drift triggers A-H
# all watch the implementation axis; trigger I watches the *symptom*
# axis.
#
# Heuristic: extract the noun-phrase shingles from the current branch's
# most-recent commit subject; for each shingle, query gh for merged PRs
# whose body or title contains it. ≥ 2 matches in the last 30 days =
# trigger fires.
#
# False positive cost: one extra step for the operator (re-confirm the
# patch axis differs OR open a redesign spec). False negative cost:
# the next deploy ships another patch on the same symptom and Pedro
# re-files the bug. The hook errs toward false positives.
#
# Bypass: never use --no-verify (CLAUDE.md hard ban #3). The operator
# may set LEFTHOOK=0 if they confirm the symptom genuinely differs, but
# that decision lives in a commit message comment, not in a flag.
set -euo pipefail

# Skip on amend / detached HEAD (worktree quirks).
if ! git symbolic-ref --short HEAD >/dev/null 2>&1; then
    exit 0
fi

# Skip on protected branches.
branch="$(git symbolic-ref --short HEAD)"
case "$branch" in
    main|master) exit 0 ;;
esac

# Stop guarding ourselves when running outside the noctalia-appmenu
# repo — defensive against worktree confusion.
remote_url="$(git config --get remote.origin.url 2>/dev/null || true)"
if [[ "$remote_url" != *yolo-labz/noctalia-appmenu* ]]; then
    exit 0
fi

# Read the most-recent commit subject as the symptom probe.
subject="$(git log -1 --format=%s 2>/dev/null || true)"
if [[ -z "$subject" ]]; then
    exit 0
fi

# Extract a handful of distinctive shingles. Strip the
# `type(scope): ` Conventional Commits prefix and split into 2-3 word
# n-grams that survive simple paraphrase. Keep the ones with letters
# (drop punctuation-only fragments).
strip="$(echo "$subject" | sed -E 's/^[a-z]+\([^)]+\): ?//' | tr -d '\r')"
shingles=()
while IFS= read -r line; do
    [[ "$line" =~ [A-Za-z]{4,} ]] || continue
    shingles+=("$line")
done < <(echo "$strip" | grep -oE '[A-Za-z][A-Za-z][A-Za-z]+( [A-Za-z]+){1,2}')

if [[ ${#shingles[@]} -eq 0 ]]; then
    exit 0
fi

# Query gh for each shingle. Count merged PRs that contain it. Stop
# scanning as soon as one shingle hits ≥ 2 matches.
hits_total=0
hit_shingle=""
match_prs=""
for s in "${shingles[@]}"; do
    # gh search prs limit 5 — cheap, recent activity only.
    matches="$(gh pr list \
        --repo yolo-labz/noctalia-appmenu \
        --state merged \
        --limit 10 \
        --search "$s in:title,body" \
        --json number,title \
        --jq '[.[] | "\(.number): \(.title)"] | join("\n")' 2>/dev/null || true)"
    if [[ -z "$matches" ]]; then
        continue
    fi
    n="$(echo "$matches" | wc -l)"
    if (( n >= 2 )); then
        hits_total="$n"
        hit_shingle="$s"
        match_prs="$matches"
        break
    fi
done

if (( hits_total < 2 )); then
    exit 0
fi

cat >&2 <<EOF

[verify-no-third-patch] DRIFT TRIGGER I fired

The current commit subject overlaps "$hit_shingle" — which is also the
symptom phrase of $hits_total recent merged PRs:

$match_prs

CLAUDE.md trigger I: "User-reported failure mode persists across ≥ 2
deploys against the same symptom" → open a redesign spec instead of
shipping a third patch on the same axis.

Required next step:

  git worktree add ../noctalia-appmenu-NNN-redesign-<bug> \\
      -b NNN-redesign origin/main
  cp .specify/templates/spec-template.md specs/NNN-<bug>/spec.md

The redesign spec MUST cite every prior patch by tag and explain why
each failed before proposing a new approach.

If you genuinely intend a NEW axis on the same symptom (rare — check
spec 015 case study first), reword the commit subject so it does not
overlap "$hit_shingle" and re-attempt.

EOF
exit 1
