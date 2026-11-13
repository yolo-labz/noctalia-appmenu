#!/usr/bin/env python3
"""Check Sonar's actual roots and preserve the Rust integration-test inventory."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# ponytail: only the two single-line, comma-separated root properties are needed;
# use a Java-properties parser if these properties gain escapes/continuations.
properties = dict(
    line.split("=", 1)
    for line in (ROOT / "sonar-project.properties").read_text().splitlines()
    if line.startswith(("sonar.sources=", "sonar.tests="))
)
roots = {}
for key in ("sonar.sources", "sonar.tests"):
    roots[key] = [(ROOT / value.strip()).resolve() for value in properties[key].split(",")]
    for path in roots[key]:
        if not path.is_dir():
            raise SystemExit(f"FAIL: {key} root does not exist: {path.relative_to(ROOT)}")

sources, tests = roots["sonar.sources"], roots["sonar.tests"]
for source in sources:
    for test in tests:
        if source == test or source in test.parents or test in source.parents:
            raise SystemExit("FAIL: source and test roots overlap")

inventory = list((ROOT / "bridge/tests").rglob("*.rs"))
if not inventory:
    raise SystemExit("FAIL: Rust integration-test inventory is empty")
for path in inventory:
    if not any(test in path.resolve().parents for test in tests):
        raise SystemExit(f"FAIL: test outside sonar.tests: {path.relative_to(ROOT)}")
print(f"PASS: analysis roots exist and include all {len(inventory)} Rust integration-test files")
