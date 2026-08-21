#!/usr/bin/env python3
"""
Parse nextest JUnit XML → normalized test-metrics.json.

Usage:
    python3 scripts/test_metrics.py --repo REPO --run-id RUN_ID [--junit junit.xml]
"""

import argparse
import json
import lzma
import math
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile
import zlib
from datetime import datetime, timezone

try:
    from compression.zstd import ZstdError

    _ZSTD_ERRORS = (ZstdError,)
except ImportError:  # compression.zstd is 3.14+ only; CI runs 3.13.
    _ZSTD_ERRORS = ()

# Every failure mode fetch_historical treats as "this one artifact is
# unreadable, skip it" — enumerated per compression method zipfile.ZipFile can
# decode (stored, deflate, bzip2, lzma, and zstd on 3.14+), plus decode/shape
# errors on the JSON payload itself:
#   OSError             - truncated/inaccessible download; bz2's decompressor
#                         also raises OSError (not a bz2-specific type) on
#                         corrupt bzip2 data
#   zipfile.BadZipFile  - corrupt zip structure / CRC mismatch
#   KeyError            - member missing (older artifact format)
#   ValueError          - malformed JSON (json.JSONDecodeError) or non-UTF-8
#                         content (UnicodeDecodeError, a UnicodeError/
#                         ValueError subclass)
#   zlib.error          - deflate bitstream corrupted mid-stream
#   NotImplementedError - compression method zipfile cannot decode at all
#   lzma.LZMAError      - lzma stream corrupted mid-stream
#   ZstdError           - zstd stream corrupted mid-stream (3.14+ only; the
#                         tuple degrades to omit it on older interpreters,
#                         see _ZSTD_ERRORS above)
# This is a closed, deliberately enumerated list of known corruption modes —
# not a catch-all. An exception outside this set is a real bug in this
# script (or a genuinely new corruption mode worth naming explicitly) and
# must propagate rather than be swallowed.
_ARTIFACT_READ_ERRORS = (
    OSError,
    zipfile.BadZipFile,
    KeyError,
    ValueError,
    zlib.error,
    NotImplementedError,
    lzma.LZMAError,
    *_ZSTD_ERRORS,
)


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
                flaky.append(
                    {"name": name, "attempts": len(reruns) + 1, "final": "pass"}
                )
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
        [
            "gh",
            "api",
            f"repos/brujack/{repo}/actions/artifacts",
            "--field",
            f"name={artifact_name}",
            "--field",
            "per_page=10",
            "--jq",
            "[.artifacts[].id]",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0 or not r.stdout.strip():
        return []

    ids = json.loads(r.stdout.strip() or "[]")
    runs = []
    for aid in ids:
        with tempfile.TemporaryDirectory() as d:
            zp = os.path.join(d, "a.zip")
            dl = subprocess.run(
                [
                    "gh",
                    "api",
                    f"repos/brujack/{repo}/actions/artifacts/{aid}/zip",
                    "--output",
                    zp,
                ],
                capture_output=True,
                check=False,
            )
            if dl.returncode != 0:
                continue
            try:
                with zipfile.ZipFile(zp) as z:
                    zinfo = z.getinfo("test-metrics.json")
                    if zinfo.flag_bits & 0x1:
                        # zipfile raises bare RuntimeError for an encrypted
                        # member (stdlib zipfile/__init__.py "is encrypted,
                        # password required") — too common a bug-class
                        # exception to blanket-catch, so detect it up front
                        # via the general-purpose flag bit instead of in the
                        # except clause below.
                        print(
                            f"WARNING: skipping artifact {aid}: encrypted "
                            f"member {zinfo.filename!r}",
                            file=sys.stderr,
                        )
                        continue
                    with z.open(zinfo) as f:
                        runs.append(json.load(f))
            except _ARTIFACT_READ_ERRORS as exc:
                print(f"WARNING: skipping artifact {aid}: {exc}", file=sys.stderr)
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
        if not isinstance(run, dict):
            # A valid-JSON-wrong-shape historical artifact (top-level list,
            # null, or a foreign document that happens to be named
            # test-metrics.json) would otherwise crash here with
            # AttributeError on `.get`. fetch_historical only guarantees the
            # artifact parsed as JSON, not that it has this shape. Warn
            # (matching fetch_historical's own skip warnings) rather than
            # dropping it silently — a silent drop here would make a
            # producer-schema regression look like "no slow tests" instead
            # of surfacing the bad data.
            print(
                f"WARNING: skipping non-dict historical run: {run!r}", file=sys.stderr
            )
            continue
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
                slow.append(
                    {"name": name, "duration_ms": ms, "z_score": round(ms / mean, 2)}
                )
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

    print(
        f"test-metrics.json: {stats['total']} tests, "
        f"{stats['flaky']} flaky, {len(slow)} slow"
    )


if __name__ == "__main__":
    main()
