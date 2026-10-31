"""Prove deny.toml loads and rejects, using throwaway fixture crates."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

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
