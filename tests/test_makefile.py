"""Tests for the Makefile build and install targets, using `make -n` only.

`make -n` prints the recipe lines without running them, so nothing is built or
installed. The Makefile's top-level `$(shell ...)` lines still run, including
`rustup which --toolchain nightly`. Given a fresh HOME, rustup builds a new
~/.rustup there and spends about 7 s doing it, so RUSTUP_HOME and CARGO_HOME are
pinned to the invoking user's real ones; a command-line override of
CARGO_NIGHTLY would not help, because make still evaluates the `:=` right-hand
side. Neither cargo nor a nightly toolchain needs to be installed.
The environment is constructed here: HOME points at a temp dir, git's exported
variables and make's flag variables are removed, and XDG_BIN_HOME is set only by
the test that wants it.
"""

import os
import subprocess
from pathlib import Path

ROOT = str(Path(__file__).resolve().parent.parent)
STRIPPED = (
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "MAKEFLAGS",
    "MFLAGS",
    "XDG_BIN_HOME",
)


REAL_HOME = Path.home()


def _make_env(home, xdg_bin_home):
    env = {k: v for k, v in os.environ.items() if k not in STRIPPED}
    env.setdefault("RUSTUP_HOME", str(REAL_HOME / ".rustup"))
    env.setdefault("CARGO_HOME", str(REAL_HOME / ".cargo"))
    env["HOME"] = str(home)
    if xdg_bin_home is not None:
        env["XDG_BIN_HOME"] = xdg_bin_home
    return env


def dry_run(target, home, xdg_bin_home=None):
    """Return the stdout lines of `make -n <target>`; fail if make fails or is silent."""
    result = subprocess.run(
        ["make", "-n", "-C", ROOT, target],
        env=_make_env(home, xdg_bin_home),
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    lines = [ln for ln in result.stdout.splitlines() if ln.strip()]
    assert lines, "make -n printed nothing, so it did not run the recipe"
    return lines


def index_starting(lines, prefix):
    """Index of the first line starting with prefix, failing with the output if absent."""
    for i, line in enumerate(lines):
        if line.startswith(prefix):
            return i
    raise AssertionError(f"no line starts with {prefix!r} in {lines}")


def test_build_stays_in_repo(tmp_path):
    lines = dry_run("build", tmp_path)
    index_starting(lines, "cargo build --release")
    assert not any("Downloads" in ln for ln in lines)


def test_build_linux_stays_in_repo(tmp_path):
    lines = dry_run("build-linux", tmp_path)
    index_starting(lines, "cargo zigbuild")
    assert not any("Downloads" in ln for ln in lines)


def test_install_uses_xdg_bin_home_after_build(tmp_path):
    lines = dry_run("install", tmp_path, xdg_bin_home="/x")
    build = index_starting(lines, "cargo build --release")
    mkdir = index_starting(lines, 'mkdir -p "/x"')
    inst = index_starting(lines, "install -m 755 target/release/etch")
    assert build < mkdir < inst
    assert "/x/etch" in lines[inst]


def test_install_defaults_to_home_local_bin(tmp_path):
    lines = dry_run("install", tmp_path)
    target = f"{tmp_path}/.local/bin"
    index_starting(lines, f'mkdir -p "{target}"')
    inst = index_starting(lines, "install -m 755 target/release/etch")
    assert f"{target}/etch" in lines[inst]


def test_empty_xdg_bin_home_falls_back_to_home(tmp_path):
    lines = dry_run("install", tmp_path, xdg_bin_home="")
    inst = index_starting(lines, "install -m 755 target/release/etch")
    assert f"{tmp_path}/.local/bin/etch" in lines[inst]
