#!/usr/bin/env bash
# verify-release.sh — spec 015 FR-007 driver.
#
# Runs every release gate under
# specs/015-ship-ready-completion/gates/ sequentially, prints one
# summary line per gate, and writes a machine-readable JSON ledger
# to /tmp/noctalia-appmenu-release-gate-v<VERSION>.json.
#
# Exit:
#   0 — every gate PASS (or SKIP when --allow-skip is set)
#   1 — at least one gate FAIL, or one gate SKIPped without --allow-skip
#
# Usage:
#   scripts/verify-release.sh                       # uses bridge/Cargo.toml version
#   scripts/verify-release.sh 1.0.24                # explicit
#   scripts/verify-release.sh 1.0.24 --allow-skip   # CI mode: SKIP is OK
#   scripts/verify-release.sh --self-test           # exercises the driver,
#                                                   # ignores gate verdicts
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
GATES_DIR="$REPO_ROOT/specs/015-ship-ready-completion/gates"
GATES=(visual routing self-heal deploy)

ALLOW_SKIP=0
SELF_TEST=0
VERSION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --allow-skip) ALLOW_SKIP=1; shift ;;
        --self-test)  SELF_TEST=1; shift ;;
        -h|--help)
            sed -n '2,17p' "$0" >&2
            exit 0 ;;
        *)
            if [[ -z "$VERSION" ]]; then VERSION="$1"; shift
            else echo "unknown arg: $1" >&2; exit 64; fi ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    VERSION="$(awk -F'"' '/^version =/ {print $2; exit}' \
        "$REPO_ROOT/bridge/Cargo.toml" 2>/dev/null || echo "unknown")"
fi

LEDGER="/tmp/noctalia-appmenu-release-gate-v${VERSION}.json"

# --- self-test ---
# Invokes each gate's own --self-test; passes only if every gate's
# scaffolding works on synthetic input. Does NOT exercise the live
# desktop. This is what CI runs.
if [[ $SELF_TEST -eq 1 ]]; then
    fail_count=0
    for gate in "${GATES[@]}"; do
        script="$GATES_DIR/${gate}.sh"
        if [[ ! -x "$script" ]]; then
            echo "[verify-release self-test gate=${gate} result=FAIL reason=missing-or-not-executable]" >&2
            fail_count=$((fail_count + 1))
            continue
        fi
        if "$script" --self-test >/dev/null 2>&1; then
            echo "[verify-release self-test gate=${gate} result=PASS]"
        else
            echo "[verify-release self-test gate=${gate} result=FAIL]" >&2
            fail_count=$((fail_count + 1))
        fi
    done
    [[ $fail_count -eq 0 ]] || exit 1
    echo "[verify-release self-test overall=PASS gates=${#GATES[@]}]"
    exit 0
fi

# --- live run ---

printf '[verify-release version=%s ledger=%s]\n' "$VERSION" "$LEDGER"

declare -a results exits durations
pass_count=0
fail_count=0
skip_count=0

for gate in "${GATES[@]}"; do
    script="$GATES_DIR/${gate}.sh"
    if [[ ! -x "$script" ]]; then
        result=MISSING
        exit_code=127
        duration_ms=0
        echo "[verify-release gate=${gate} result=MISSING script=${script}]" >&2
        fail_count=$((fail_count + 1))
    else
        start_ns=$(date +%s%N)
        set +e
        "$script" >/tmp/verify-release-${gate}.stdout 2>/tmp/verify-release-${gate}.stderr
        exit_code=$?
        set -e
        end_ns=$(date +%s%N)
        duration_ms=$(( (end_ns - start_ns) / 1000000 ))
        case $exit_code in
            0) result=PASS;  pass_count=$((pass_count + 1)) ;;
            2) result=SKIP;  skip_count=$((skip_count + 1)) ;;
            *) result=FAIL;  fail_count=$((fail_count + 1)) ;;
        esac
        printf '[verify-release gate=%s result=%s exit=%d duration_ms=%d]\n' \
            "$gate" "$result" "$exit_code" "$duration_ms"
        if [[ $result != PASS ]]; then
            sed 's/^/  | /' "/tmp/verify-release-${gate}.stderr" >&2 || true
        fi
    fi
    results+=("$result")
    exits+=("$exit_code")
    durations+=("$duration_ms")
done

# --- JSON ledger ---
{
    printf '{\n'
    printf '  "version": "%s",\n' "$VERSION"
    printf '  "ran_at": "%s",\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf '  "host": "%s",\n' "$(hostname)"
    printf '  "allow_skip": %s,\n' "$([[ $ALLOW_SKIP -eq 1 ]] && echo true || echo false)"
    printf '  "gates": [\n'
    last=$(( ${#GATES[@]} - 1 ))
    for i in "${!GATES[@]}"; do
        printf '    {"name": "%s", "result": "%s", "exit": %d, "duration_ms": %d}' \
            "${GATES[$i]}" "${results[$i]}" "${exits[$i]}" "${durations[$i]}"
        [[ $i -lt $last ]] && printf ','
        printf '\n'
    done
    printf '  ],\n'
    printf '  "summary": {"pass": %d, "fail": %d, "skip": %d}\n' \
        "$pass_count" "$fail_count" "$skip_count"
    printf '}\n'
} > "$LEDGER"

# --- verdict ---
if [[ $fail_count -gt 0 ]]; then
    printf '[verify-release overall=FAIL pass=%d fail=%d skip=%d]\n' \
        "$pass_count" "$fail_count" "$skip_count" >&2
    exit 1
fi

if [[ $skip_count -gt 0 ]] && [[ $ALLOW_SKIP -eq 0 ]]; then
    printf '[verify-release overall=FAIL pass=%d fail=%d skip=%d reason=skip-not-allowed]\n' \
        "$pass_count" "$fail_count" "$skip_count" >&2
    printf '  rerun with --allow-skip to treat SKIP as PASS (CI without niri/Wayland seat)\n' >&2
    exit 1
fi

printf '[verify-release overall=PASS pass=%d fail=%d skip=%d]\n' \
    "$pass_count" "$fail_count" "$skip_count"
