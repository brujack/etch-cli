> **Status: DONE**

# Test Metrics CI — etch-cli Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Instrument etch-cli CI to capture flaky tests (nextest retries) and timing variance, uploading a normalized JSON artifact after every run.

**Architecture:** Add a `ci` nextest profile with `retries=2` and JUnit output. A Python post-processor parses the JUnit XML, queries the last 10 historical artifacts for z-score computation, and emits `test-metrics.json`. Uploaded as a 90-day GitHub Actions artifact on every CI run (`if: always()`).

**Tech Stack:** `cargo nextest`, JUnit XML, Python 3 (stdlib only), `gh` CLI, GitHub Actions `upload-artifact@v5`

---

## Files

- **Create:** `.config/nextest.toml`
- **Create:** `scripts/test_metrics.py`
- **Create:** `tests/test_test_metrics.py`
- **Modify:** `.github/workflows/ci.yml` — test job: replace `make test` step, add post-processor + artifact upload steps

---

## Task 1: nextest.toml + test_metrics.py (TDD)

**Files:**

- Create: `.config/nextest.toml`
- Create: `scripts/test_metrics.py`
- Create: `tests/test_test_metrics.py`

- [ ] **Step 1: Create `.config/nextest.toml`**

```toml
[profile.ci]
retries = { backoff = "fixed", count = 2 }

[profile.ci.junit]
path = "junit.xml"
```

- [ ] **Step 2: Verify nextest picks up the profile**

```bash
cargo nextest run --profile ci --list 2>&1 | head -5
```

Expected: lists tests without error. No tests run yet (just listing).

- [ ] **Step 3: Write failing tests for `parse_junit`**

Create `tests/test_test_metrics.py`:

```python
#!/usr/bin/env python3
import json
import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from scripts.test_metrics import parse_junit, compute_slow

JUNIT_PASS = """\
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="etch-lib" tests="2" failures="0" errors="0" time="1.234">
    <testcase name="it_passes" classname="mod::tests" time="0.500"/>
    <testcase name="it_is_slow" classname="mod::tests" time="1.000"/>
  </testsuite>
</testsuites>
"""

JUNIT_FLAKY = """\
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="etch-lib" tests="1" failures="0" errors="0" time="0.300">
    <testcase name="it_flakes" classname="mod::tests" time="0.300">
      <flakyFailure message="flake" type="failure">attempt 1</flakyFailure>
      <flakyFailure message="flake" type="failure">attempt 2</flakyFailure>
    </testcase>
  </testsuite>
</testsuites>
"""

JUNIT_FAIL = """\
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="etch-lib" tests="1" failures="1" errors="0" time="0.100">
    <testcase name="it_fails" classname="mod::tests" time="0.100">
      <failure message="assertion failed" type="failure">expected true</failure>
    </testcase>
  </testsuite>
</testsuites>
"""


def _tmp(content):
    f = tempfile.NamedTemporaryFile(mode="w", suffix=".xml", delete=False)
    f.write(content)
    f.close()
    return f.name


class TestParseJunit(unittest.TestCase):
    def test_passing_tests_stats(self):
        flaky, timings, stats = parse_junit(_tmp(JUNIT_PASS))
        self.assertEqual(stats["total"], 2)
        self.assertEqual(stats["passed"], 2)
        self.assertEqual(stats["failed"], 0)
        self.assertEqual(stats["flaky"], 0)

    def test_passing_test_timings(self):
        _, timings, _ = parse_junit(_tmp(JUNIT_PASS))
        self.assertIn("mod::tests.it_passes", timings)
        self.assertAlmostEqual(timings["mod::tests.it_passes"], 500.0, places=0)

    def test_flaky_test_detected(self):
        flaky, _, stats = parse_junit(_tmp(JUNIT_FLAKY))
        self.assertEqual(stats["flaky"], 1)
        self.assertEqual(len(flaky), 1)
        self.assertEqual(flaky[0]["attempts"], 3)
        self.assertEqual(flaky[0]["final"], "pass")

    def test_flaky_test_name(self):
        flaky, _, _ = parse_junit(_tmp(JUNIT_FLAKY))
        self.assertEqual(flaky[0]["name"], "mod::tests.it_flakes")

    def test_failed_test_counted(self):
        _, _, stats = parse_junit(_tmp(JUNIT_FAIL))
        self.assertEqual(stats["failed"], 1)
        self.assertEqual(stats["passed"], 0)

    def test_empty_junit(self):
        path = _tmp('<?xml version="1.0"?><testsuites/>')
        flaky, timings, stats = parse_junit(path)
        self.assertEqual(stats["total"], 0)
        self.assertEqual(timings, {})
        self.assertEqual(flaky, [])


class TestComputeSlow(unittest.TestCase):
    def test_no_history_returns_empty(self):
        self.assertEqual(compute_slow({"a": 500.0}, []), [])

    def test_fewer_than_3_history_returns_empty(self):
        hist = [{"all_timings": {"a": 100.0}}, {"all_timings": {"a": 110.0}}]
        self.assertEqual(compute_slow({"a": 500.0}, hist), [])

    def test_detects_slow_test(self):
        hist = [{"all_timings": {"a": 100.0}} for _ in range(5)]
        # duration 500ms vs mean 100ms with near-zero std => very high z
        result = compute_slow({"a": 500.0}, hist)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "a")
        self.assertGreaterEqual(result[0]["z_score"], 2.0)

    def test_normal_test_not_flagged(self):
        hist = [{"all_timings": {"a": ms}} for ms in [100, 105, 98, 102, 101]]
        result = compute_slow({"a": 106.0}, hist)
        self.assertEqual(result, [])

    def test_near_zero_std_skipped(self):
        # All same value → std=0 → skip to avoid divide by zero
        hist = [{"all_timings": {"a": 100.0}} for _ in range(5)]
        result = compute_slow({"a": 100.0}, hist)
        self.assertEqual(result, [])

    def test_results_sorted_by_z_desc(self):
        hist = [{"all_timings": {"a": 100.0, "b": 200.0}} for _ in range(5)]
        result = compute_slow({"a": 500.0, "b": 300.0}, hist)
        if len(result) >= 2:
            self.assertGreaterEqual(result[0]["z_score"], result[1]["z_score"])


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 4: Run tests — confirm they fail**

```bash
python3 -m unittest tests.test_test_metrics -v 2>&1 | head -20
```

Expected: `ModuleNotFoundError: No module named 'scripts.test_metrics'`

- [ ] **Step 5: Create `scripts/test_metrics.py`**

```python
#!/usr/bin/env python3
"""
Parse nextest JUnit XML → normalized test-metrics.json.

Usage:
    python3 scripts/test_metrics.py --repo REPO --run-id RUN_ID [--junit junit.xml]
"""
import argparse
import json
import math
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from datetime import datetime, timezone


def parse_junit(path: str):
    """Return (flaky_tests, all_timings_dict, stats_dict)."""
    tree = ET.parse(path)
    root = tree.getroot()
    suites = root.findall("testsuite") if root.tag == "testsuites" else [root]

    flaky, timings = [], {}
    total = passed = failed = 0

    for suite in suites:
        for tc in suite.findall("testcase"):
            name = f"{tc.get('classname', '')}.{tc.get('name', '')}"
            timings[name] = round(float(tc.get("time", 0)) * 1000, 1)

            reruns = tc.findall("flakyFailure") + tc.findall("rerunFailure")
            failures = tc.findall("failure") + tc.findall("error")
            total += 1

            if reruns and not failures:
                flaky.append({"name": name, "attempts": len(reruns) + 1, "final": "pass"})
                passed += 1
            elif failures:
                failed += 1
            else:
                passed += 1

    stats = {
        "total": total,
        "passed": passed,
        "failed": failed,
        "flaky": len(flaky),
        "total_duration_ms": round(sum(timings.values()), 1),
    }
    return flaky, timings, stats


def fetch_historical(repo: str, artifact_name: str = "test-metrics") -> list:
    """Download last 10 test-metrics artifacts. Returns list of parsed JSON dicts."""
    r = subprocess.run(
        ["gh", "api", f"repos/brujack/{repo}/actions/artifacts",
         "--field", f"name={artifact_name}", "--field", "per_page=10",
         "--jq", "[.artifacts[].id]"],
        capture_output=True, text=True, check=False,
    )
    if r.returncode != 0 or not r.stdout.strip():
        return []

    ids = json.loads(r.stdout.strip() or "[]")
    runs = []
    for aid in ids:
        with tempfile.TemporaryDirectory() as d:
            zp = os.path.join(d, "a.zip")
            dl = subprocess.run(
                ["gh", "api", f"repos/brujack/{repo}/actions/artifacts/{aid}/zip",
                 "--output", zp],
                capture_output=True, check=False,
            )
            if dl.returncode != 0:
                continue
            try:
                with zipfile.ZipFile(zp) as z, z.open("test-metrics.json") as f:
                    runs.append(json.load(f))
            except Exception:
                continue
    return runs


def compute_slow(timings: dict, historical: list, z_threshold: float = 2.0) -> list:
    """Return tests with z-score >= z_threshold. Needs >= 3 historical data points."""
    by_name: dict[str, list] = {}
    for run in historical:
        for name, ms in run.get("all_timings", {}).items():
            by_name.setdefault(name, []).append(ms)

    slow = []
    for name, ms in timings.items():
        hist = by_name.get(name, [])
        if len(hist) < 3:
            continue
        mean = sum(hist) / len(hist)
        std = math.sqrt(sum((x - mean) ** 2 for x in hist) / len(hist))
        if std < 1.0:
            continue
        z = (ms - mean) / std
        if z >= z_threshold:
            slow.append({"name": name, "duration_ms": ms, "z_score": round(z, 2)})

    return sorted(slow, key=lambda x: -x["z_score"])


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--repo", required=True, help="GitHub repo name (e.g. etch-cli)")
    p.add_argument("--run-id", required=True, help="GitHub Actions run ID")
    p.add_argument("--junit", default="junit.xml", help="Path to JUnit XML file")
    p.add_argument("--runner", default="nextest")
    p.add_argument("--artifact-name", default="test-metrics")
    args = p.parse_args()

    if not os.path.exists(args.junit):
        print(f"ERROR: {args.junit} not found", file=sys.stderr)
        sys.exit(1)

    flaky, timings, stats = parse_junit(args.junit)
    historical = fetch_historical(args.repo, args.artifact_name)
    slow = compute_slow(timings, historical)

    result = {
        "repo": args.repo,
        "run_id": args.run_id,
        "date": datetime.now(timezone.utc).isoformat(),
        "runner": args.runner,
        "flaky_tests": flaky,
        "slow_tests": slow,
        "all_timings": timings,
        "stats": stats,
    }

    with open("test-metrics.json", "w") as f:
        json.dump(result, f, indent=2)

    print(f"test-metrics.json: {stats['total']} tests, "
          f"{stats['flaky']} flaky, {len(slow)} slow")


if __name__ == "__main__":
    main()
```

- [ ] **Step 6: Run tests — confirm they all pass**

```bash
python3 -m unittest tests.test_test_metrics -v 2>&1
```

Expected: `OK (12 tests)`

- [ ] **Step 7: Commit**

```bash
git add .config/nextest.toml scripts/test_metrics.py tests/test_test_metrics.py
git commit -m "feat: add nextest CI profile and test-metrics post-processor

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Modify CI workflow

**Files:**

- Modify: `.github/workflows/ci.yml` — test job only

- [ ] **Step 1: Read the current test job**

```bash
grep -n "Run tests\|make test\|nextest" .github/workflows/ci.yml
```

Note the line numbers of the `Run tests` step.

- [ ] **Step 2: Replace the `Run tests` step and add post-processor + upload**

In `.github/workflows/ci.yml`, replace the `Run tests` step (currently `run: make test`) with:

```yaml
- name: Run tests
  run: make lint && cargo nextest run --profile ci
- name: Generate test metrics
  if: always()
  env:
      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
      python3 scripts/test_metrics.py \
        --repo etch-cli \
        --run-id "${{ github.run_id }}" \
        --junit junit.xml
- name: Upload test metrics
  if: always()
  uses: actions/upload-artifact@v5
  with:
      name: test-metrics
      path: test-metrics.json
      retention-days: 90
```

The `make lint && cargo nextest run --profile ci` replaces `make test` because the Makefile's `test` target runs `cargo nextest run` (no profile). Running lint + nextest directly gives the CI profile with retries and JUnit output.

- [ ] **Step 3: Verify YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML valid"
```

Expected: `YAML valid`

- [ ] **Step 4: Dry-run test locally to confirm nextest produces junit.xml**

```bash
cargo nextest run --profile ci --no-run 2>&1 | head -5
ls junit.xml 2>/dev/null && echo "junit.xml exists" || echo "not yet (needs a real run)"
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add test-metrics collection with nextest CI profile

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Post-merge docs update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index in etch-cli README**

In `docs/superpowers/README.md`, update the test-metrics row to Done (add plan link, change status).

- [ ] **Step 2: Add Done banner to this plan file**

Add at the top of `docs/superpowers/plans/2026-05-19-test-metrics.md`:

```markdown
> **Status: DONE**
```

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-19-test-metrics.md
git commit -m "docs: mark test-metrics plan done

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```
