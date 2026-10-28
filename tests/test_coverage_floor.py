"""The Python coverage floor in CI must be at least 90 and match the CLAUDE.md DoD line.

A floor is only ever raised, never lowered, so a test pins the minimum. The DoD
line states the same number for the session reading CLAUDE.md, and the two must
not drift apart.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MINIMUM = 90
_FLAG = re.compile(r"--cov-fail-under=(\d+)")


def floors(text):
    return [int(m) for m in _FLAG.findall(text)]


def test_floors_finds_every_flag():
    assert floors("a --cov-fail-under=90 b --cov-fail-under=87") == [90, 87]


def test_floors_is_empty_without_a_flag():
    assert floors("pytest tests/ --cov=scripts") == []


def test_ci_has_exactly_one_python_floor_at_or_above_minimum():
    found = floors((ROOT / ".github" / "workflows" / "ci.yml").read_text())
    assert len(found) == 1, found
    assert found[0] >= MINIMUM, found


def test_claude_md_dod_states_the_ci_floor():
    ci = floors((ROOT / ".github" / "workflows" / "ci.yml").read_text())
    dod = floors((ROOT / "CLAUDE.md").read_text())
    assert dod == ci, (dod, ci)
