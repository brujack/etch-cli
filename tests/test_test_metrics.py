#!/usr/bin/env python3
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from scripts.test_metrics import compute_slow, fetch_historical, parse_junit

# compression.zstd (and writable zipfile.ZIP_ZSTANDARD support) is 3.14+
# only; CI runs 3.13. Guard the zstd-specific test rather than failing there.
# find_spec() raises ModuleNotFoundError rather than returning None when the
# PARENT package is absent, and 3.13 has no `compression` package at all — so
# import the submodule directly and let ImportError answer the question.
try:
    import compression.zstd  # noqa: F401
except ImportError:
    _HAS_ZSTD = False
else:
    _HAS_ZSTD = True

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
        _flaky, _timings, stats = parse_junit(_tmp(JUNIT_PASS))
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

    def test_non_dict_run_skipped_not_crashed(self):
        """A valid-JSON-wrong-shape historical artifact (e.g. a top-level
        list or null) must be skipped, not crash with AttributeError on
        `.get` — the real dict entries around it still contribute."""
        hist = [{"all_timings": {"a": 100.0}} for _ in range(4)] + [
            None,
            ["not", "a", "dict"],
        ]
        result = compute_slow({"a": 500.0}, hist)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "a")

    def test_non_dict_run_warns_not_silent(self):
        """Skipping a wrong-shape run must be visible on stderr, matching
        fetch_historical's own skip warnings — a silent drop here would make
        a producer-schema regression (every baseline gone) look identical to
        a clean "no slow tests" result."""
        hist = [None]
        stderr_capture = io.StringIO()
        with contextlib.redirect_stderr(stderr_capture):
            compute_slow({"a": 500.0}, hist)
        self.assertIn("skipping non-dict historical run", stderr_capture.getvalue())


def _fake_run_factory(artifact_ids, writers):
    """Build a subprocess.run replacement for fetch_historical's two call shapes:
    the artifact-id listing call (has --jq) and the per-artifact download call
    (has --output). `writers` is a list of callables, one per artifact id in
    order, each given the destination path to populate.
    """
    state = {"downloads": 0}

    def fake_run(cmd, **kwargs):
        if "--jq" in cmd:
            return subprocess.CompletedProcess(
                cmd, 0, stdout=json.dumps(artifact_ids), stderr=""
            )
        out_path = cmd[cmd.index("--output") + 1]
        writers[state["downloads"]](out_path)
        state["downloads"] += 1
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    return fake_run


def _make_corrupt_deflate_zip(path):
    """Write a real DEFLATE-compressed zip, then flip one byte deep inside the
    compressed payload. This leaves the zip structure (local header, central
    directory, EOCD) intact — zipfile can open the archive and find the member
    — while making the deflate bitstream itself invalid. Reproduces the
    zlib.error a genuinely mid-stream-corrupted upload-artifact zip raises,
    distinct from a truncated file (OSError) or a CRC mismatch (BadZipFile)."""
    payload = json.dumps(
        {"all_timings": {f"t{i}": float(i) for i in range(50)}}
    ).encode()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as z:
        z.writestr("test-metrics.json", payload)
    with open(path, "rb") as f:
        raw = bytearray(f.read())
    idx = raw.find(b"PK\x03\x04")
    name_len = int.from_bytes(raw[idx + 26 : idx + 28], "little")
    extra_len = int.from_bytes(raw[idx + 28 : idx + 30], "little")
    data_start = idx + 30 + name_len + extra_len
    raw[data_start + 2] ^= 0xFF
    with open(path, "wb") as f:
        f.write(raw)


def _write_valid_artifact(path, timing_name="x", timing_ms=1.0):
    with zipfile.ZipFile(path, "w") as z:
        z.writestr(
            "test-metrics.json", json.dumps({"all_timings": {timing_name: timing_ms}})
        )


def _patch_u16(raw, offset, value):
    raw[offset : offset + 2] = int(value).to_bytes(2, "little")


def _make_unsupported_compression_zip(path):
    """Write a normal zip, then set the compression-method field (2 bytes) to
    99 — a value zipfile.ZipFile() happily parses out of both the local file
    header and the central directory record (metadata only, no validation) —
    but raises NotImplementedError("That compression method is not
    supported") on when actually opening/reading the member."""
    payload = json.dumps({"all_timings": {"x": 1.0}}).encode()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as z:
        z.writestr("test-metrics.json", payload)
    with open(path, "rb") as f:
        raw = bytearray(f.read())
    lh_idx = raw.find(b"PK\x03\x04")
    _patch_u16(raw, lh_idx + 8, 99)
    cd_idx = raw.find(b"PK\x01\x02")
    _patch_u16(raw, cd_idx + 10, 99)
    with open(path, "wb") as f:
        f.write(raw)


def _make_corrupt_lzma_zip(path):
    """Write a real LZMA-compressed (ZIP_LZMA) zip, then flip one byte deep
    inside the compressed payload (empirically reliable at this offset —
    earlier bytes round-trip through LZMA's stream header unaffected).
    Reproduces the lzma.LZMAError a genuinely mid-stream-corrupted
    LZMA-compressed artifact raises — distinct from zlib.error (deflate) and
    from a CRC mismatch (BadZipFile)."""
    payload = json.dumps(
        {"all_timings": {f"t{i}": float(i) for i in range(50)}}
    ).encode()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_LZMA) as z:
        z.writestr("test-metrics.json", payload)
    with open(path, "rb") as f:
        raw = bytearray(f.read())
    idx = raw.find(b"PK\x03\x04")
    name_len = int.from_bytes(raw[idx + 26 : idx + 28], "little")
    extra_len = int.from_bytes(raw[idx + 28 : idx + 30], "little")
    data_start = idx + 30 + name_len + extra_len
    raw[data_start + 9] ^= 0xFF
    with open(path, "wb") as f:
        f.write(raw)


def _make_corrupt_zstd_zip(path):
    """Write a real Zstandard-compressed (ZIP_ZSTANDARD) zip, then flip one
    byte deep inside the compressed payload (empirically reliable at this
    offset). Reproduces the compression.zstd.ZstdError a genuinely
    mid-stream-corrupted zstd-compressed artifact raises. Only called from a
    test gated on _HAS_ZSTD (interpreter has 3.14+'s compression.zstd)."""
    payload = json.dumps(
        {"all_timings": {f"t{i}": float(i) for i in range(50)}}
    ).encode()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_ZSTANDARD) as z:
        z.writestr("test-metrics.json", payload)
    with open(path, "rb") as f:
        raw = bytearray(f.read())
    idx = raw.find(b"PK\x03\x04")
    name_len = int.from_bytes(raw[idx + 26 : idx + 28], "little")
    extra_len = int.from_bytes(raw[idx + 28 : idx + 30], "little")
    data_start = idx + 30 + name_len + extra_len
    raw[data_start + 9] ^= 0xFF
    with open(path, "wb") as f:
        f.write(raw)


def _make_encrypted_flag_zip(path):
    """Write a normal zip, then set general-purpose flag bit 0 (the
    encryption bit) in both the local file header and the central directory
    record. zipfile.ZipFile.getinfo() surfaces this via ZipInfo.flag_bits
    without complaint; only ZipFile.open() raises (bare RuntimeError, "is
    encrypted, password required") — which is why fetch_historical must
    check the flag before opening, not catch the RuntimeError after."""
    payload = json.dumps({"all_timings": {"x": 1.0}}).encode()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as z:
        z.writestr("test-metrics.json", payload)
    with open(path, "rb") as f:
        raw = bytearray(f.read())
    lh_idx = raw.find(b"PK\x03\x04")
    lh_flags = int.from_bytes(raw[lh_idx + 6 : lh_idx + 8], "little")
    _patch_u16(raw, lh_idx + 6, lh_flags | 0x1)
    cd_idx = raw.find(b"PK\x01\x02")
    cd_flags = int.from_bytes(raw[cd_idx + 8 : cd_idx + 10], "little")
    _patch_u16(raw, cd_idx + 8, cd_flags | 0x1)
    with open(path, "wb") as f:
        f.write(raw)


class TestFetchHistorical(unittest.TestCase):
    def test_skips_bad_artifact_logs_and_continues(self):
        """A corrupt historical artifact must be skipped (with a logged warning
        naming the artifact id), not crash the whole fetch — the loop's purpose
        is best-effort aggregation across N historical runs."""

        def write_corrupt(path):
            with open(path, "wb") as f:
                f.write(b"not a valid zip file")

        fake_run = _fake_run_factory([1, 2], [write_corrupt, _write_valid_artifact])
        stderr_capture = io.StringIO()

        with (
            patch("scripts.test_metrics.subprocess.run", side_effect=fake_run),
            contextlib.redirect_stderr(stderr_capture),
        ):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)
        # Assert the artifact id is named, not just that some warning fired —
        # a regression dropping the {aid} interpolation would still pass a
        # bare "skipping artifact" check.
        self.assertIn("skipping artifact 1", stderr_capture.getvalue())

    def test_skips_corrupted_deflate_payload_and_continues(self):
        """A zip whose structure is intact but whose deflate stream is corrupted
        mid-payload raises zlib.error — not OSError, BadZipFile, KeyError, or
        ValueError. It must still be skipped so a later good artifact in the
        same fetch is aggregated, not abort the whole loop."""
        fake_run = _fake_run_factory(
            [1, 2], [_make_corrupt_deflate_zip, _write_valid_artifact]
        )

        with patch("scripts.test_metrics.subprocess.run", side_effect=fake_run):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)

    def test_skips_non_utf8_member_and_continues(self):
        """A member containing invalid UTF-8 bytes raises UnicodeDecodeError,
        which is not a json.JSONDecodeError subclass. It must still be skipped
        so a later good artifact in the same fetch is aggregated."""

        def write_non_utf8(path):
            with zipfile.ZipFile(path, "w") as z:
                z.writestr(
                    "test-metrics.json", b'{"all_timings": {"a": "\x80\x81bad"}}'
                )

        fake_run = _fake_run_factory([1, 2], [write_non_utf8, _write_valid_artifact])

        with patch("scripts.test_metrics.subprocess.run", side_effect=fake_run):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)

    def test_skips_unsupported_compression_method_and_continues(self):
        """A member whose compression-method field names an unsupported
        method raises NotImplementedError when opened/read — not OSError,
        BadZipFile, KeyError, ValueError, or zlib.error. It must still be
        skipped so a later good artifact in the same fetch is aggregated."""
        fake_run = _fake_run_factory(
            [1, 2], [_make_unsupported_compression_zip, _write_valid_artifact]
        )

        with patch("scripts.test_metrics.subprocess.run", side_effect=fake_run):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)

    def test_skips_encrypted_member_and_continues(self):
        """A member with the encrypted general-purpose flag bit set must be
        skipped via the up-front flag_bits check (not a bare RuntimeError
        catch), so a later good artifact in the same fetch is aggregated."""
        fake_run = _fake_run_factory(
            [1, 2], [_make_encrypted_flag_zip, _write_valid_artifact]
        )
        stderr_capture = io.StringIO()

        with (
            patch("scripts.test_metrics.subprocess.run", side_effect=fake_run),
            contextlib.redirect_stderr(stderr_capture),
        ):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)
        self.assertIn("skipping artifact 1", stderr_capture.getvalue())
        self.assertIn("encrypted", stderr_capture.getvalue())

    def test_skips_corrupted_lzma_payload_and_continues(self):
        """A zip whose structure is intact but whose LZMA stream is corrupted
        mid-payload raises lzma.LZMAError — not any of the other enumerated
        types. It must still be skipped so a later good artifact in the same
        fetch is aggregated."""
        fake_run = _fake_run_factory(
            [1, 2], [_make_corrupt_lzma_zip, _write_valid_artifact]
        )

        with patch("scripts.test_metrics.subprocess.run", side_effect=fake_run):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)

    @unittest.skipUnless(_HAS_ZSTD, "compression.zstd is 3.14+ only")
    def test_skips_corrupted_zstd_payload_and_continues(self):
        """A zip whose structure is intact but whose Zstandard stream is
        corrupted mid-payload raises compression.zstd.ZstdError. It must
        still be skipped so a later good artifact in the same fetch is
        aggregated. Skipped on interpreters without compression.zstd
        (< 3.14) — CI runs 3.13."""
        fake_run = _fake_run_factory(
            [1, 2], [_make_corrupt_zstd_zip, _write_valid_artifact]
        )

        with patch("scripts.test_metrics.subprocess.run", side_effect=fake_run):
            result = fetch_historical("etch-cli")

        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["all_timings"]["x"], 1.0)

    def test_propagates_unexpected_exception(self):
        """An exception outside the narrowed (OSError, BadZipFile, KeyError,
        ValueError, zlib.error, NotImplementedError, lzma.LZMAError,
        ZstdError) set is a real bug, not corrupt historical data — it must
        propagate rather than be silently swallowed by the artifact loop.
        AttributeError is the sentinel here (not RuntimeError): zipfile
        itself raises a bare RuntimeError for an encrypted member, a
        skip-worthy condition handled by the flag_bits pre-check, so
        RuntimeError is the wrong type to prove "must not be swallowed"
        with."""
        fake_run = _fake_run_factory([1], [_write_valid_artifact])

        with (
            patch("scripts.test_metrics.subprocess.run", side_effect=fake_run),
            patch("scripts.test_metrics.json.load", side_effect=AttributeError("boom")),
        ):
            with self.assertRaises(AttributeError):
                fetch_historical("etch-cli")


if __name__ == "__main__":
    unittest.main()
