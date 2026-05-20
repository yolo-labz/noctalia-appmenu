#!/usr/bin/env bash
# verify-tokens.sh — spec 015 FR-010 token-discipline guard.
#
# Refuses raw design literals in plugin/*.qml. Mirrors the VP-060..VP-064
# rows of specs/015-ship-ready-completion/checklists/visual-parity.md.
#
# Usage:
#   scripts/verify-tokens.sh                       # checks every plugin/*.qml
#   scripts/verify-tokens.sh path1.qml path2.qml   # checks given files only
#                                                  # (used by lefthook
#                                                  # `pre-commit` against
#                                                  # `{staged_files}`)
#
# Exit: 0 on clean, 1 on any violation. Violation lines printed to stderr
# in `file:line: rule: matched-text` form so editors can jump to the hit.
#
# Excludes:
#   - any match on a line starting with `//` (single-line comment)
#   - VP-061 carve-out: matches inside `_measureText { ... }` are allowed
#     (the bar-strip measurement widget intentionally sizes against the
#     bar's own font scale, per visual-spec)
#
# Defensive-fallback pattern is allowed AND preferred:
#   `duration: Style.animationFast !== undefined ? Style.animationFast : 150`
# Because the literal `150` only appears after the ternary, the grep
# below (which requires the literal to follow `duration:` directly) won't
# trigger on it. That is intentional — the fallback is the migration
# bridge for older noctalia builds where `Style.animationFast` may be
# undefined.

set -euo pipefail

if [[ $# -eq 0 ]]; then
    mapfile -t FILES < <(ls plugin/*.qml 2>/dev/null)
else
    FILES=("$@")
fi

if [[ ${#FILES[@]} -eq 0 ]]; then
    echo "verify-tokens.sh: no plugin/*.qml files to check" >&2
    exit 0
fi

FAIL=0
report() {
    local rule="$1" file="$2" line="$3" text="$4"
    printf '%s:%s: %s: %s\n' "$file" "$line" "$rule" "$text" >&2
    FAIL=1
}

# Strip lines that are single-line `//` comments at any indent.
filter_comments() { grep -vE '^[^:]*:[0-9]+:[[:space:]]*//'; }

for f in "${FILES[@]}"; do
    [[ -f $f ]] || continue
    case "$f" in
        plugin/*.qml) ;;
        *) continue ;;
    esac

    # VP-060 — raw 6-hex literal.
    while IFS= read -r line; do
        ln=${line%%:*}; rest=${line#*:}; text=${rest#*:}
        report VP-060 "$f" "$ln" "$text"
    done < <(grep -nE '#[0-9a-fA-F]{6}' "$f" | filter_comments)

    # VP-061 — font.pixelSize literal outside _measureText.
    while IFS= read -r line; do
        ln=${line%%:*}; rest=${line#*:}; text=${rest#*:}
        # Skip the bar-strip _measureText carve-out by quick context test:
        # if the previous 5 lines contain `_measureText`, allow.
        ctx_start=$((ln > 5 ? ln - 5 : 1))
        if sed -n "${ctx_start},${ln}p" "$f" | grep -q '_measureText'; then
            continue
        fi
        report VP-061 "$f" "$ln" "$text"
    done < <(grep -nE 'font\.pixelSize[[:space:]]*:[[:space:]]*[0-9]' "$f" | filter_comments)

    # VP-062 — `radius:` followed by a literal integer (no dot prefix so
    # we skip `border.radius:` which is not an own-property assignment).
    while IFS= read -r line; do
        ln=${line%%:*}; rest=${line#*:}; text=${rest#*:}
        report VP-062 "$f" "$ln" "$text"
    done < <(grep -nE '(^|[^.[:alnum:]_])radius[[:space:]]*:[[:space:]]*[0-9]' "$f" | filter_comments)

    # VP-063 — `border.width:` literal != 0.
    while IFS= read -r line; do
        ln=${line%%:*}; rest=${line#*:}; text=${rest#*:}
        report VP-063 "$f" "$ln" "$text"
    done < <(grep -nE 'border\.width[[:space:]]*:[[:space:]]*[1-9]' "$f" | filter_comments)

    # VP-064 — bare `duration:` literal not behind defensive fallback.
    while IFS= read -r line; do
        ln=${line%%:*}; rest=${line#*:}; text=${rest#*:}
        # Allow the defensive pattern: text contains `!== undefined` AND a
        # `Style.animation` token before the trailing literal.
        if echo "$text" | grep -qE 'Style\.animation[A-Za-z]+[[:space:]]*!==[[:space:]]*undefined'; then
            continue
        fi
        report VP-064 "$f" "$ln" "$text"
    done < <(grep -nE '\bduration[[:space:]]*:[[:space:]]*[0-9]' "$f" | filter_comments)
done

if [[ $FAIL -ne 0 ]]; then
    cat >&2 <<'EOF'

verify-tokens: token-discipline violations found (spec 015 VP-060..VP-064).
Fix one of:
  - replace literal with the matching Style.* token, or
  - use the defensive fallback pattern, e.g.:
      duration: Style.animationNormal !== undefined ? Style.animationNormal : 180
  - if the literal is truly load-bearing (cross-shell-version compat),
    document it as a row in specs/015-ship-ready-completion/visual-audit.md
    with explicit rationale, then add the path to the carve-out logic
    above. Never broaden the carve-out without an audit row.

EOF
    exit 1
fi

echo "verify-tokens: OK (${#FILES[@]} file(s) clean)"
