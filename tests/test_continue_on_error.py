"""Allow-list for every `continue-on-error` site in .github/workflows and composite actions.

A `continue-on-error: true` on a security scan turns a failing gate into a green
tick; gitleaks and Snyk were masked this way unnoticed. Each site must be listed
in ALLOWED with the reason it is safe, so adding a new mask is a reviewed
decision in this file rather than a one-line change in a workflow.

Workflows are read as text (no PyYAML). A site is keyed by (path relative to .github/, nearest
preceding `name:` value). To check against an older tree, call
scan_workflows with a directory holding those workflows; the directory must be
named `workflows` for the keys to match.
"""

import glob
import os
import re
from collections import Counter

WORKFLOW_DIR = ".github/workflows"

SITE_RE = re.compile(r"^\s*continue-on-error:\s*(?!false\s*$)\S")
MENTION_RE = re.compile(r"continue-on-error")
FALSE_RE = re.compile(r"^\s*continue-on-error:\s*false\s*$")
NAME_RE = re.compile(r"^\s*(-\s+)?name:\s*(.+?)\s*$")

ALLOWED = {
    (
        "workflows/ci.yml",
        "Coverage with network tests (informational)",
    ): "network-dependent tests are informational; the main coverage step gates.",
    (
        "workflows/ci.yml",
        "Semver Check",
    ): "job-level and kept out of auto-merge needs:; every new action adds an enum variant.",
    (
        "workflows/mutation-testing.yml",
        "Run mutants",
    ): "the Score gate step (if: always()) scores the result.",
    (
        "workflows/release.yml",
        "Check semver compatibility",
    ): "advisory on release, as in semver-check.",
    (
        "workflows/release.yml",
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


def workflow_files(workflow_dir=WORKFLOW_DIR):
    """Return every workflow and composite-action file, .yml and .yaml."""
    found = []
    for pattern in ("*.yml", "*.yaml"):
        found += glob.glob(os.path.join(workflow_dir, pattern))
    if workflow_dir == ".github/workflows":
        for action in ("action.yml", "action.yaml"):
            found += glob.glob(
                os.path.join(".github", "actions", "**", action), recursive=True
            )
    return sorted(found)


def relative_name(path, workflow_dir=WORKFLOW_DIR):
    """Return the file's path relative to the directory above workflow_dir."""
    return os.path.relpath(path, os.path.dirname(workflow_dir) or ".")


def find_unrecognised(text):
    """Return lines that mention the key in a form SITE_RE cannot classify.

    Plain `continue-on-error: false` and whole-line comments are fine; any
    other spelling (quoted key, flow mapping, ...) must fail closed.
    """
    return [
        line
        for line in text.splitlines()
        if MENTION_RE.search(line)
        and not line.lstrip().startswith("#")
        and not SITE_RE.match(line)
        and not FALSE_RE.match(line)
    ]


def scan_workflows(workflow_dir=WORKFLOW_DIR):
    """Return every site across the workflow files."""
    sites = []
    for path in workflow_files(workflow_dir):
        with open(path, encoding="utf-8") as fh:
            sites.extend(find_sites(relative_name(path, workflow_dir), fh.read()))
    return sites


def scan_unrecognised(workflow_dir=WORKFLOW_DIR):
    """Return (file, line) for every unclassifiable mention across the files."""
    found = []
    for path in workflow_files(workflow_dir):
        with open(path, encoding="utf-8") as fh:
            found.extend(
                (relative_name(path, workflow_dir), x)
                for x in find_unrecognised(fh.read())
            )
    return found


def allow_list_problems(sites):
    """Return sites that differ from ALLOWED as an exact multiset."""
    expected = Counter(dict.fromkeys(ALLOWED, 1))
    actual = Counter(sites)
    return actual - expected, expected - actual


def test_scanner_finds_at_least_one_site():
    assert scan_workflows(), "no continue-on-error site found; scanner is broken"


def test_sites_match_allow_list_exactly():
    extra, missing = allow_list_problems(scan_workflows())
    assert not extra, f"continue-on-error not in ALLOWED (or duplicated): {extra}"
    assert not missing, f"ALLOWED entries with no matching site: {missing}"


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


def _write(tmp_path, relname, text):
    """Write text under tmp_path and return the workflows dir to scan."""
    wf = tmp_path / "workflows"
    target = tmp_path / relname
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")
    return str(wf)


def test_yaml_extension_is_scanned(tmp_path):
    wf = _write(
        tmp_path,
        "workflows/evil.yaml",
        "- name: Sneaky\n  continue-on-error: true\n",
    )
    assert scan_workflows(wf) == [("workflows/evil.yaml", "Sneaky")]


def test_sites_are_keyed_by_path_not_basename(tmp_path):
    wf = _write(
        tmp_path,
        "workflows/ci.yml",
        "- name: Run mutants\n  continue-on-error: true\n",
    )
    assert scan_workflows(wf) == [("workflows/ci.yml", "Run mutants")]


def test_flow_mapping_spelling_fails_closed():
    assert find_unrecognised("- name: x\n  {continue-on-error: true}\n")
    assert find_unrecognised('- name: x\n  "continue-on-error": true\n')


def test_plain_false_and_comments_are_recognised():
    assert not find_unrecognised("- name: x\n  continue-on-error: false\n")
    assert not find_unrecognised("  # continue-on-error: true\n")
    assert not find_unrecognised("  continue-on-error: true\n")


def test_real_tree_has_no_unrecognised_spelling():
    assert not scan_unrecognised()


def test_duplicate_site_with_allowed_key_is_rejected():
    sample = (
        "- name: Run mutants\n  continue-on-error: true\n"
        "- name: Run mutants\n  continue-on-error: true\n"
    )
    key = ("workflows/mutation-testing.yml", "Run mutants")
    extra, _ = allow_list_problems(find_sites(key[0], sample))
    assert extra == {key: 1}
