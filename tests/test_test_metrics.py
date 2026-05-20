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
        result = compute_slow({"a": 500.0}, hist)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "a")
        self.assertGreaterEqual(result[0]["z_score"], 2.0)

    def test_normal_test_not_flagged(self):
        hist = [{"all_timings": {"a": ms}} for ms in [100, 105, 98, 102, 101]]
        result = compute_slow({"a": 106.0}, hist)
        self.assertEqual(result, [])

    def test_near_zero_std_skipped(self):
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
