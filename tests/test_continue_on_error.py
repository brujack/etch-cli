"""Allow-list for every `continue-on-error` site in .github/workflows/*.yml.

A `continue-on-error: true` on a security scan turns a failing gate into a green
tick; gitleaks and Snyk were masked this way unnoticed. Each site must be listed
in ALLOWED with the reason it is safe, so adding a new mask is a reviewed
decision in this file rather than a one-line change in a workflow.

Workflows are read as text (no PyYAML). A site is keyed by (file name, nearest
preceding `name:` value). Point COE_WORKFLOW_DIR at a directory of older
workflows to confirm the test fails against a pre-change tree.
"""

import glob
import os
import re

WORKFLOW_DIR = os.environ.get("COE_WORKFLOW_DIR", ".github/workflows")

SITE_RE = re.compile(r"^\s*continue-on-error:\s*(?!false\s*$)\S")
NAME_RE = re.compile(r"^\s*(-\s+)?name:\s*(.+?)\s*$")

ALLOWED = {
    (
        "ci.yml",
        "Coverage with network tests (informational)",
    ): "network-dependent tests are informational; the main coverage step gates.",
    (
        "ci.yml",
        "Semver Check",
    ): "job-level and kept out of auto-merge needs:; every new action adds an enum variant.",
    (
        "mutation-testing.yml",
        "Run mutants",
    ): "the Score gate step (if: always()) scores the result.",
    (
        "release.yml",
        "Check semver compatibility",
    ): "advisory on release, as in semver-check.",
    (
        "release.yml",
        "Commit CHANGELOG",
    ): "removed by PR B (Task 4).",
}


def find_sites(filename, text):
    """Return (filename, nearest preceding name or None) for each site in text."""
    sites = []
    current_name = None
    for line in text.splitlines():
        match = NAME_RE.match(line)
        if match:
            current_name = match.group(2).strip("'\"")
        if SITE_RE.match(line):
            sites.append((filename, current_name))
    return sites


def scan_workflows():
    """Return every site across the workflow directory."""
    sites = []
    for path in sorted(glob.glob(os.path.join(WORKFLOW_DIR, "*.yml"))):
        with open(path, encoding="utf-8") as fh:
            sites.extend(find_sites(os.path.basename(path), fh.read()))
    return sites


def test_scanner_finds_at_least_one_site():
    assert scan_workflows(), "no continue-on-error site found; scanner is broken"


def test_every_site_is_allow_listed():
    unlisted = [s for s in scan_workflows() if s not in ALLOWED]
    assert not unlisted, f"continue-on-error not in ALLOWED: {unlisted}"


def test_every_allowed_entry_still_occurs():
    found = set(scan_workflows())
    stale = [k for k in ALLOWED if k not in found]
    assert not stale, f"ALLOWED entries with no matching site: {stale}"


def test_every_reason_is_non_empty():
    assert all(reason.strip() for reason in ALLOWED.values())


def test_scanner_keys_unnamed_site_as_none():
    sample = "jobs:\n  x:\n    continue-on-error: true\n"
    assert find_sites("s.yml", sample) == [("s.yml", None)]


def test_scanner_ignores_explicit_false():
    sample = "- name: Keep\n  continue-on-error: false\n"
    assert find_sites("s.yml", sample) == []
