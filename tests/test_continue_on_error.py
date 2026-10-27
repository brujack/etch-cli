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


ITEM_RE = re.compile(r"^(\s*)-\s")
JOB_RE = re.compile(r"^  [A-Za-z0-9_-]+:\s*$")


def find_sites(filename, text):
    """Return (filename, name of the enclosing step or job, or None) per site.

    The name resets at every list item and job key, and only a `name:` at the
    step's own key indent counts, so an unnamed step cannot inherit a neighbour's
    name and a nested `name:` (e.g. under `with:`) cannot override the step's.
    """
    sites = []
    current_name = None
    key_indent = None
    for line in text.splitlines():
        item = ITEM_RE.match(line)
        if item:
            current_name = None
            key_indent = len(item.group(1)) + 2
        elif JOB_RE.match(line):
            current_name = None
            key_indent = 4
        match = NAME_RE.match(line)
        if match:
            indent = len(line) - len(line.lstrip())
            if match.group(1):
                indent += 2
            if indent == key_indent:
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


def test_unnamed_step_does_not_inherit_previous_step_name():
    sample = (
        "steps:\n"
        "  - name: Run mutants\n"
        "    run: x\n"
        "  - uses: some/action@v1\n"
        "    continue-on-error: true\n"
    )
    assert find_sites("mutation-testing.yml", sample) == [
        ("mutation-testing.yml", None)
    ]


def test_unnamed_job_does_not_inherit_previous_job_name():
    sample = (
        "jobs:\n"
        "  a:\n"
        "    name: Semver Check\n"
        "    runs-on: x\n"
        "  b:\n"
        "    continue-on-error: true\n"
    )
    assert find_sites("ci.yml", sample) == [("ci.yml", None)]


def test_nested_name_does_not_override_step_name():
    sample = (
        "steps:\n"
        "  - name: Real\n"
        "    with:\n"
        "      name: nested\n"
        "    continue-on-error: true\n"
    )
    assert find_sites("s.yml", sample) == [("s.yml", "Real")]


def test_scanner_ignores_explicit_false():
    sample = "- name: Keep\n  continue-on-error: false\n"
    assert find_sites("s.yml", sample) == []
