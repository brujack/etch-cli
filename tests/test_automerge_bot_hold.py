"""Behavioural tests for the auto-merge `hold` guard in .github/workflows/ci.yml.

The guard's `run:` text is extracted from the workflow (no PyYAML) and executed
under `bash -e` against a stub `gh`, so the test exercises the shipped script
rather than a copy. Point BOTHOLD_WORKFLOW at an older workflow to confirm the
test fails against the pre-change guard.
"""

import json
import os
import re
import stat
import subprocess
import textwrap

import pytest

WORKFLOW = os.environ.get("BOTHOLD_WORKFLOW", ".github/workflows/ci.yml")

STUB_GH = """#!/usr/bin/env bash
if [ "${GH_FAIL:-}" = "1" ]; then
  exit 1
fi
if [ "${GH_FAIL_LABELS:-}" = "1" ]; then
  for a in "$@"; do
    if [ "$a" = "labels" ]; then
      exit 1
    fi
  done
fi
expr=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--jq" ]; then
    expr="$2"
    break
  fi
  shift
done
jq -r "${expr}" "${GH_FIXTURE}"
"""


def extract_hold_run(path):
    """Return the dedented `run: |` text of the step whose id is `hold`."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().splitlines()
    id_idx = next(
        (i for i, ln in enumerate(lines) if re.match(r"^\s*(-\s+)?id:\s*hold\s*$", ln)),
        None,
    )
    assert id_idx is not None, "no `id: hold` step in " + path
    step_indent = len(lines[id_idx]) - len(lines[id_idx].lstrip())
    run_idx = None
    for i in range(id_idx + 1, len(lines)):
        ln = lines[i]
        if ln.strip() and not ln.lstrip().startswith("#"):
            indent = len(ln) - len(ln.lstrip())
            if indent < step_indent:
                break
            if indent == step_indent and re.match(r"^\s*run:\s*\|\s*$", ln):
                run_idx = i
                break
    assert run_idx is not None, "hold step has no `run: |` block"
    run_indent = len(lines[run_idx]) - len(lines[run_idx].lstrip())
    body = []
    for ln in lines[run_idx + 1 :]:
        if ln.strip() and (len(ln) - len(ln.lstrip())) <= run_indent:
            break
        body.append(ln)
    text = textwrap.dedent("\n".join(body)).strip("\n")
    assert text.strip(), "hold step run block is empty"
    return text + "\n"


def run_guard(tmp_path, fixture, gh_fail=False, labels_fail=False):
    stub_dir = tmp_path / "bin"
    stub_dir.mkdir()
    gh = stub_dir / "gh"
    gh.write_text(STUB_GH)
    gh.chmod(gh.stat().st_mode | stat.S_IXUSR)
    fixture_file = tmp_path / "fixture.json"
    fixture_file.write_text(json.dumps(fixture))
    out_file = tmp_path / "github_output"
    out_file.write_text("")
    env = {
        "PATH": f"{stub_dir}:{os.environ['PATH']}",
        "HOME": str(tmp_path),
        "PR_NUMBER": "1",
        "GH_TOKEN": "x",
        "GITHUB_TOKEN": "x",
        "GITHUB_OUTPUT": str(out_file),
        "GH_FIXTURE": str(fixture_file),
        "GH_FAIL": "1" if gh_fail else "0",
        "GH_FAIL_LABELS": "1" if labels_fail else "0",
    }
    proc = subprocess.run(
        ["bash", "-e", "-c", extract_hold_run(WORKFLOW)],
        env=env,
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
        check=False,
    )
    return proc.returncode, out_file.read_text()


def fx(is_bot, login, labels=()):
    return {
        "author": {"is_bot": is_bot, "login": login},
        "labels": [{"name": n} for n in labels],
    }


CASES = [
    pytest.param(fx(False, "human"), False, 0, "merge=true\n", id="1-human-no-label"),
    pytest.param(
        fx(True, "app/renovate", ["automerge-ok"]),
        False,
        0,
        "merge=true\n",
        id="2-renovate-labelled",
    ),
    pytest.param(
        fx(True, "app/renovate"), False, 0, "merge=false\n", id="3-renovate-unlabelled"
    ),
    pytest.param(
        fx(True, "app/dependabot"),
        False,
        0,
        "merge=false\n",
        id="4-dependabot-unlabelled",
    ),
    pytest.param(
        fx(True, "app/dependabot", ["automerge-ok"]),
        False,
        0,
        "merge=true\n",
        id="5-dependabot-labelled",
    ),
    pytest.param(fx(True, ""), False, 1, "", id="6-empty-login"),
    pytest.param(fx("maybe", "x"), False, 1, "", id="7-unreadable-is-bot"),
    pytest.param(fx(True, "app/renovate"), True, 1, "", id="8-gh-fails"),
    pytest.param(
        fx(True, "app/renovate", ["automerge-ok-pending"]),
        False,
        0,
        "merge=false\n",
        id="9-label-prefix-is-not-a-match",
    ),
    pytest.param(
        fx(True, "app/github-actions"),
        False,
        0,
        "merge=false\n",
        id="10-github-actions-unlabelled",
    ),
    # Real Renovate PRs carry `renovate` and `automerge-ok` (renovate.json
    # labels + addLabels). Single-label fixtures let a string-equality or a
    # position-dependent match pass while it holds every real Renovate PR.
    pytest.param(
        fx(True, "app/renovate", ["renovate", "automerge-ok"]),
        False,
        0,
        "merge=true\n",
        id="11-several-labels-automerge-ok-last",
    ),
    pytest.param(
        fx(True, "app/renovate", ["automerge-ok", "renovate"]),
        False,
        0,
        "merge=true\n",
        id="12-several-labels-automerge-ok-first",
    ),
]


@pytest.mark.parametrize("fixture,gh_fail,rc,output", CASES)
def test_hold_guard(tmp_path, fixture, gh_fail, rc, output):
    got_rc, got_out = run_guard(tmp_path, fixture, gh_fail)
    assert got_rc == rc
    assert got_out == output


def test_label_query_failure_exits_1_without_a_merge_line(tmp_path):
    """Author query succeeds, label query fails: broken instrument, not a hold."""
    got_rc, got_out = run_guard(
        tmp_path, fx(True, "app/renovate", ["automerge-ok"]), labels_fail=True
    )
    assert got_rc == 1
    assert "merge=" not in got_out
