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


def compute_slow(timings: dict, historical: list, z_threshold: float = 3.0) -> list:
    """Return tests whose current duration is anomalously high vs historical baseline.

    Uses z-score when std >= 1.0. When std < 1.0 (very stable baseline), falls back
    to a 3x ratio check so genuinely slow outliers are still detected.
    Requires >= 3 historical data points to produce any output.
    """
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
            # Stable baseline: flag only clear magnitude outliers (>3x mean).
            # This handles zero-std baselines without division-by-zero.
            if mean > 0 and ms > mean * 3:
                slow.append({"name": name, "duration_ms": ms, "z_score": round(ms / mean, 2)})
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
