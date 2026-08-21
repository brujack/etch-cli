"""Guards that every reference to the vendored dependency rendering names the
same file.

The rendering is committed here byte-identical to its source in dotfiles so that
a plain ``diff`` against that source IS the staleness check. That property has a
failure mode with no natural alarm: the repo names the artifact in four
independent places -- the CI install line, the diff command in ci.yml's comment,
the diff command in CLAUDE.md, and the developer install line in README.md -- and
nothing forces those strings to agree. When the install target and the diff
target drift apart, the staleness check goes on faithfully measuring a file that
nobody installs, and reports clean forever.

That is distinct from a check that cannot fail because it shares a derivation
with its subject. This one can fail; it would simply be failing about the wrong
artifact. The remedy is the opposite too: not a second, independently-derived
check (which would still be about the wrong file), but collapsing the references
to a single asserted name -- which is what this module does.

Every extraction below asserts it matched something before comparing. A regex
that silently matches nothing would make each assertEqual compare "" to "" and
pass, which is the same class of vacuous green this file exists to prevent.
"""

import re
import subprocess
import unittest
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent

# `pip install -r <file>` inside a workflow `run:` line -- what CI actually installs.
_INSTALL_RE = re.compile(r"pip install -r ([^\s\"'`]+)")
# The local (left-hand) operand of the documented staleness command. The remote
# operand is checked separately: it must point into dotfiles, or the diff would
# compare the file against something that is not its source.
_DIFF_RE = re.compile(r"diff ([^\s]+) (\S*dotfiles/[^\s`]+)")


def _hashes_per_package(text):
    """Map each pinned package to its own hash count.

    Requirement blocks are ``name==version`` followed by backslash-continued
    ``--hash=`` lines, so a package's hashes are the ones between its pin and the
    next pin.
    """
    counts = {}
    current = None
    for line in text.splitlines():
        stripped = line.strip()
        pin = re.match(r"^([A-Za-z0-9._-]+)==", stripped)
        if pin:
            current = pin.group(1)
            counts[current] = 0
        elif stripped.startswith("--hash=") and current is not None:
            counts[current] += 1
    return counts


def _read(relative_path):
    path = _REPO / relative_path
    if not path.is_file():
        raise AssertionError(f"expected {relative_path} to exist at {path}")
    return path.read_text()


class TestReferencesAgree(unittest.TestCase):
    """All four references must name one file."""

    def setUp(self):
        self.ci = _read(".github/workflows/ci.yml")
        self.claude_md = _read("CLAUDE.md")
        self.readme = _read("README.md")

    def test_ci_declares_exactly_one_install_target(self):
        found = _INSTALL_RE.findall(self.ci)
        self.assertEqual(
            len(found),
            1,
            f"expected exactly one 'pip install -r' line in ci.yml, found {found}. "
            "Two install lines mean two artifacts to keep in sync, which is the "
            "drift this module guards against.",
        )

    def test_install_target_and_ci_diff_target_agree(self):
        install = _INSTALL_RE.findall(self.ci)
        diff = _DIFF_RE.findall(self.ci)
        self.assertTrue(install, "no 'pip install -r' line found in ci.yml")
        self.assertTrue(diff, "no 'diff <local> <dotfiles/...>' command found in ci.yml")
        self.assertEqual(
            install[0],
            diff[0][0],
            "ci.yml installs one file and documents diffing another; the staleness "
            "check would measure a file CI never installs.",
        )

    def test_install_target_and_claude_md_diff_target_agree(self):
        install = _INSTALL_RE.findall(self.ci)
        diff = _DIFF_RE.findall(self.claude_md)
        self.assertTrue(install, "no 'pip install -r' line found in ci.yml")
        self.assertTrue(diff, "no staleness diff command found in CLAUDE.md")
        self.assertEqual(install[0], diff[0][0])

    def test_install_target_and_readme_agree(self):
        install = _INSTALL_RE.findall(self.ci)
        readme = _INSTALL_RE.findall(self.readme)
        self.assertTrue(install, "no 'pip install -r' line found in ci.yml")
        self.assertTrue(readme, "README.md documents no 'pip install -r' line")
        self.assertIn(
            install[0],
            readme,
            "README tells a developer to install a different file than CI does.",
        )

    def test_diff_remote_operand_points_at_dotfiles(self):
        for label, text in (("ci.yml", self.ci), ("CLAUDE.md", self.claude_md)):
            with self.subTest(source=label):
                diff = _DIFF_RE.findall(text)
                self.assertTrue(diff, f"no staleness diff command found in {label}")
                local, remote = diff[0]
                self.assertTrue(
                    remote.endswith(local),
                    f"{label} diffs {local} against {remote}; the remote operand must "
                    "be the same filename in dotfiles, or the check compares the "
                    "rendering against something that is not its source.",
                )


class TestTargetIsReal(unittest.TestCase):
    """The agreed-upon name must refer to a file this repo actually tracks."""

    def setUp(self):
        self.install = _INSTALL_RE.findall(_read(".github/workflows/ci.yml"))
        self.assertTrue(self.install, "no 'pip install -r' line found in ci.yml")
        self.target = self.install[0]

    def test_target_exists(self):
        self.assertTrue(
            (_REPO / self.target).is_file(),
            f"ci.yml installs {self.target}, which does not exist in the repo.",
        )

    def test_target_is_tracked_by_git(self):
        result = subprocess.run(
            ["git", "ls-files", "--error-unmatch", self.target],
            cwd=_REPO,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"{self.target} is not tracked by git, so CI would install a file that "
            "is not in the repository.",
        )

    def test_target_declares_no_loose_specifiers(self):
        text = (_REPO / self.target).read_text()
        pins = re.findall(r"^([A-Za-z0-9._-]+)==", text, re.MULTILINE)
        loose = re.findall(r"^[A-Za-z0-9._-]+(?:>=|<=|~=|>|<)", text, re.MULTILINE)
        self.assertTrue(pins, f"{self.target} declares no '==' pinned packages")
        self.assertEqual(loose, [], f"{self.target} contains unpinned specifiers: {loose}")

    def test_every_pinned_package_carries_its_own_hash(self):
        """Per package, not in aggregate.

        The first version of this compared total hashes against total packages.
        With 428 hashes against 65 packages that inequality holds however many
        unhashed packages are added, so it could not fail for the thing it
        existed to catch -- verified by mutation: appending an unhashed
        ``somepkg==1.0`` left it green. A single unhashed requirement drops pip
        out of --require-hashes mode, so the property is per-package or it is
        nothing.
        """
        per_package = _hashes_per_package((_REPO / self.target).read_text())
        self.assertTrue(per_package, f"{self.target} declares no pinned packages")
        unhashed = sorted(name for name, count in per_package.items() if count == 0)
        self.assertEqual(
            unhashed,
            [],
            f"{self.target} pins these packages with no hash: {unhashed}. One "
            "unhashed requirement disables hash checking for the whole file.",
        )


class TestExtractionIsNotVacuous(unittest.TestCase):
    """The regexes must actually discriminate.

    Without these, a regex that stopped matching would make every assertion above
    compare empty against empty and pass -- the exact silent-green this guards.
    """

    def test_install_regex_rejects_text_without_an_install_line(self):
        self.assertEqual(_INSTALL_RE.findall("run: echo hello\n"), [])

    def test_install_regex_captures_the_filename(self):
        self.assertEqual(
            _INSTALL_RE.findall("run: pip install -r some-file.txt\n"), ["some-file.txt"]
        )

    def test_diff_regex_rejects_a_diff_not_against_dotfiles(self):
        self.assertEqual(_DIFF_RE.findall("diff a.txt ~/elsewhere/a.txt"), [])

    def test_diff_regex_captures_both_operands(self):
        self.assertEqual(
            _DIFF_RE.findall("diff a.txt ~/git-repos/personal/dotfiles/a.txt"),
            [("a.txt", "~/git-repos/personal/dotfiles/a.txt")],
        )


if __name__ == "__main__":
    unittest.main()
