"""Prove deny.toml loads and rejects, using throwaway fixture crates."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest
import tomllib

REPO = Path(__file__).resolve().parent.parent
FIXTURES = REPO / "tests/fixtures/deny"


def _cargo_deny() -> str:
    """Return the cargo-deny path; fail under CI, skip locally when absent."""
    path = shutil.which("cargo-deny")
    if path is None:
        if os.environ.get("CI"):
            pytest.fail("cargo-deny missing under CI")
        pytest.skip("cargo-deny not installed")
    return path


def _run(fixture: str, tmp_path: Path) -> subprocess.CompletedProcess[str]:
    """Run `cargo deny check licenses` on a copy of a fixture crate.

    The copy keeps cargo-deny's Cargo.lock write out of the repo tree.
    """
    cargo_deny = _cargo_deny()
    shutil.copytree(FIXTURES / fixture, tmp_path / fixture)
    return subprocess.run(
        [
            cargo_deny,
            "deny",
            "--config",
            str(REPO / "deny.toml"),
            "--manifest-path",
            str(tmp_path / fixture / "Cargo.toml"),
            "check",
            "licenses",
        ],
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )


def test_gpl_fixture_is_rejected(tmp_path):
    result = _run("gpl-fixture", tmp_path)
    assert result.returncode != 0
    assert "error[rejected]" in result.stdout + result.stderr


def test_mit_fixture_is_accepted(tmp_path):
    """Proves deny.toml loaded: cargo-deny's built-in fallback allows no licence."""
    result = _run("mit-fixture", tmp_path)
    assert result.returncode == 0, result.stdout + result.stderr


def test_cargo_deny_missing_fails_under_ci(monkeypatch):
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    monkeypatch.setenv("CI", "true")
    with pytest.raises(pytest.fail.Exception):
        _cargo_deny()


def test_cargo_deny_missing_skips_locally(monkeypatch):
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    monkeypatch.delenv("CI", raising=False)
    with pytest.raises(pytest.skip.Exception):
        _cargo_deny()


def test_one_ignore_list_no_workflow_has_its_own() -> None:
    """deny.toml is the only advisory ignore list. cargo-deny has no --ignore
    flag; this stops a re-added `cargo audit --ignore RUSTSEC-...` from
    becoming a second list."""
    files = sorted((REPO / ".github/workflows").glob("*.y*ml"))
    assert files
    offenders = [f.name for f in files if "--ignore RUSTSEC" in f.read_text()]
    assert offenders == []


def test_advisories_cover_transitive_crates() -> None:
    """cargo-deny 0.20.2 defaults `unsound` to "workspace", which hid
    RUSTSEC-2026-0306 and -0205 (etch-cli#159). The fixtures cannot see this
    setting, so pin both scope keys explicitly."""
    with (REPO / "deny.toml").open("rb") as handle:
        advisories = tomllib.load(handle)["advisories"]
    assert advisories.get("unsound") == "all"
    assert advisories.get("unmaintained") == "all"


# Every cargo-deny call in the workflows, exactly. Dropping a check from the
# blocking line, or adding -A/--allow, --exclude* or -t, narrows the run while
# the fixtures and deny.toml stay green.
DENY_CALLS = {
    "ci.yml": ["cargo deny check advisories licenses bans sources"],
    "cargo-deny-scheduled.yml": ["cargo deny check advisories"],
}


def test_workflow_cargo_deny_calls_are_pinned() -> None:
    found = {}
    for f in sorted((REPO / ".github/workflows").glob("*.y*ml")):
        calls = [
            line.strip().removeprefix("run:").strip()
            for line in f.read_text().splitlines()
            if "cargo deny" in line and not line.strip().startswith(("#", "- name:"))
        ]
        if calls:
            found[f.name] = calls
    assert found == DENY_CALLS


def test_licence_overrides_are_pinned() -> None:
    """A [[licenses.clarify]] or [licenses].exceptions entry can relabel a GPL
    crate as permissive, and the GPL/MIT fixtures cannot see it."""
    with (REPO / "deny.toml").open("rb") as handle:
        licenses = tomllib.load(handle)["licenses"]
    assert licenses.get("exceptions") == []
    assert licenses.get("clarify") == [
        {
            "name": "ring",
            "version": "*",
            "expression": "MIT AND ISC AND OpenSSL",
            "license-files": [{"path": "LICENSE", "hash": 0xBD0EED23}],
        }
    ]
