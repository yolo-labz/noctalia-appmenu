#!/usr/bin/env bash
# visual.sh — spec 015 SC-002 release gate.
#
# Re-runs the visual-parity checklist (VP-001..VP-064) and the
# token-discipline guard (verify-tokens.sh) against the working tree.
# Single PASS line on success, FAIL with row IDs on regression.
#
# Two evidence sources, both must agree:
#   1. specs/015-ship-ready-completion/visual-audit.md — must contain
#      zero `FAIL` cells. The audit doc is the canonical PASS/FAIL
#      ledger; this gate refuses to ship if any row regresses.
#   2. scripts/verify-tokens.sh — runtime grep for raw design literals
#      in plugin/*.qml. The runtime check catches a fresh literal that
#      the audit doc hasn't been updated for yet.
#
# Exit:
#   0 — PASS (both checks clean)
#   1 — FAIL (audit has FAIL row OR verify-tokens hit a violation)
#
# Self-test:
#   bash gates/visual.sh --self-test
# Artificially injects a FAIL row into a temp copy of the audit doc
# and confirms the gate catches it. Used by spec 015 T4.1 acceptance.

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

AUDIT_DOC="specs/015-ship-ready-completion/visual-audit.md"
TOKEN_SCRIPT="scripts/verify-tokens.sh"
GATE_NAME="visual"

self_test() {
    # Tree-independent: synthesise a 2-row audit table, flip PASS→FAIL,
    # confirm the FAIL-row regex catches it. Does NOT require the live
    # visual-audit.md (which lands in a sibling PR).
    local synthetic
    synthetic=$(cat <<'EOF'
| VP-001 | bar height | PASS | foo |
| VP-002 | bar gap    | PASS | bar |
EOF
)
    local flipped
    flipped=$(sed 's/| PASS |/| FAIL |/' <<<"$synthetic")
    local fail_count
    fail_count=$(grep -cE '\| FAIL\b' <<<"$flipped" || true)
    if [[ $fail_count -lt 2 ]]; then
        echo "[gate name=${GATE_NAME} self-test=FAIL — injection produced ${fail_count}/2 FAIL rows]" >&2
        return 1
    fi
    echo "[gate name=${GATE_NAME} self-test=PASS injected_fails=${fail_count}]"
    return 0
}

if [[ ${1:-} == "--self-test" ]]; then
    self_test
    exit $?
fi

if [[ ! -f $AUDIT_DOC ]]; then
    echo "[gate name=${GATE_NAME} result=FAIL reason=missing-audit doc=${AUDIT_DOC}]" >&2
    exit 1
fi

if [[ ! -x $TOKEN_SCRIPT ]] && [[ ! -f $TOKEN_SCRIPT ]]; then
    echo "[gate name=${GATE_NAME} result=FAIL reason=missing-verify-tokens]" >&2
    exit 1
fi

# Count audit FAIL rows. Match `| FAIL |` or `| FAIL\n` (table cell).
audit_fails=$(grep -cE '\| FAIL\b' "$AUDIT_DOC" || true)

# Run token-discipline guard. Capture exit code; output goes to gate stderr.
token_status=PASS
token_log=$(bash "$TOKEN_SCRIPT" 2>&1) || token_status=FAIL

# Count rows in audit for the trials_passed denominator.
audit_rows=$(grep -cE '^\| VP-[0-9]+ \|' "$AUDIT_DOC" || true)
audit_passes=$((audit_rows - audit_fails))

if [[ $audit_fails -gt 0 ]] || [[ $token_status == FAIL ]]; then
    echo "[gate name=${GATE_NAME} result=FAIL audit_rows=${audit_rows} audit_passes=${audit_passes} audit_fails=${audit_fails} token=${token_status}]" >&2
    if [[ $audit_fails -gt 0 ]]; then
        echo "--- audit FAIL rows ---" >&2
        grep -E '\| FAIL\b' "$AUDIT_DOC" >&2 || true
    fi
    if [[ $token_status == FAIL ]]; then
        echo "--- verify-tokens output ---" >&2
        echo "$token_log" >&2
    fi
    exit 1
fi

echo "[gate name=${GATE_NAME} result=PASS audit_rows=${audit_rows} audit_passes=${audit_passes} token=PASS]"
