#!/usr/bin/env python3
"""Exercise the workflow's byte-comparison step with matching and differing files."""
import os
from pathlib import Path
import re
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
workflow = (root / ".github/workflows/reproducibility.yml").read_text()
assert "cargo clean" not in "\n".join(
    line for line in workflow.splitlines() if not line.lstrip().startswith("#")
)
assert "--rebuild" in workflow
assert "${{ runner.temp }}/build-1" in workflow
assert "${{ runner.temp }}/build-2" in workflow
match = re.search(r"      - name: Compare bytes\n        run: \|\n((?:          .*\n|\n)+)", workflow)
assert match, "workflow comparison step missing"
script = "\n".join(line[10:] for line in match[1].splitlines())
with tempfile.TemporaryDirectory(prefix="r5-repro-") as directory:
    path = Path(directory)
    env = dict(os.environ, RUNNER_TEMP=directory)
    (path / "build-1").write_bytes(b"synthetic binary A\n")
    for data, expected in ((b"synthetic binary A\n", 0), (b"synthetic binary B\n", 1)):
        (path / "build-2").write_bytes(data)
        result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script], env=env, capture_output=True, text=True)
        assert result.returncode == expected, result.stdout + result.stderr
    (path / "build-2").unlink()
    result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script], env=env, capture_output=True, text=True)
    assert result.returncode != 0, "missing build must fail closed"
print("PASS: matching bytes pass; changed/missing bytes fail; rebuild and artifact paths retained")
