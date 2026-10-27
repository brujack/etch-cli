"""Behavioural and static tests for .github/workflows/release.yml.

The `validate` and `range` steps' `run:` text is extracted from the workflow (no
PyYAML) and executed under `bash -e` inside a fixture git repo, so the tests
exercise the shipped scripts rather than a copy. To check against an older
workflow, call `static_problems(path)` or `extract_step_run(path, step_id)` with
that file; no environment variable selects the path.

The static checks are deliberately strict string checks that fail closed: any
occurrence of the text "CHANGELOG" (even in a comment) fails, the pinned
git-cliff action SHA must appear exactly once, and `steps.range.outputs.base`
must be referenced. Write release.yml to satisfy them: no comment mentioning the
old changelog, one git-cliff step using the range output, `checkout` first then
the `validate` step, an `id: notes_check` step running
`[[ -s /tmp/release-notes.md ]]` after git-cliff and before "Create and push
tag", and `git push origin "v${VERSION}"` as the only push.
"""

import os
import re
import subprocess
import textwrap

import pytest

WORKFLOW = ".github/workflows/release.yml"
CLIFF_SHA = "orhun/git-cliff-action@f50e11560dce63f7c33227798f90b924471a88b5"
ALLOWED_PUSH = 'git push origin "v${VERSION}"'
CARGO_FILES = ("app/Cargo.toml", "lib/Cargo.toml")
GIT_STRIPPED = ("GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_INDEX_FILE")


def _indent(line):
    return len(line) - len(line.lstrip())


def _key_indent(line):
    """Indent of the step's own keys, whether or not the line starts with `- `."""
    return _indent(line) + (2 if line.lstrip().startswith("- ") else 0)


def extract_step_run(path, step_id):
    """Return the dedented `run: |` text of the step whose id is `step_id`."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().splitlines()
    pattern = re.compile(r"^\s*(-\s+)?id:\s*" + re.escape(step_id) + r"\s*$")
    id_idx = next((i for i, ln in enumerate(lines) if pattern.match(ln)), None)
    assert id_idx is not None, f"no `id: {step_id}` step in {path}"
    key_indent = _key_indent(lines[id_idx])
    run_idx = None
    for i in range(id_idx + 1, len(lines)):
        ln = lines[i]
        if not ln.strip() or ln.lstrip().startswith("#"):
            continue
        if _indent(ln) < key_indent:
            break
        if _indent(ln) == key_indent and re.match(r"^\s*run:\s*\|\s*$", ln):
            run_idx = i
            break
    assert run_idx is not None, f"{step_id} step has no `run: |` block"
    body = []
    for ln in lines[run_idx + 1 :]:
        if ln.strip() and _indent(ln) <= key_indent:
            break
        body.append(ln)
    text = textwrap.dedent("\n".join(body)).strip("\n")
    assert text.strip(), f"{step_id} step run block is empty"
    return text + "\n"


def release_job_steps(text):
    """Return the `release` job's steps as text blocks, in order."""
    job = text.split("\n  sign:")[0]
    parts = re.split(r"(?m)^      - ", job)
    return ["- " + p for p in parts[1:]]


def reads_version(step):
    """True when a step's text refers to the version input or VERSION env."""
    return "VERSION" in step or "inputs.version" in step


def ordering_problems(steps):
    """Return problems with where the validate step sits among the steps."""
    ids = [i for i, s in enumerate(steps) if re.search(r"(?m)^\s+id: validate\s*$", s)]
    if not ids:
        return ["no validate step"]
    problems = []
    if ids[0] != 1 or "actions/checkout" not in steps[0]:
        problems.append("validate step is not directly after checkout")
    early = [i for i, s in enumerate(steps) if i < ids[0] and reads_version(s)]
    if early:
        problems.append(f"steps {early} read VERSION before validate")
    return problems


def push_problems(text):
    """Return every `git push` line other than the tag push."""
    return [
        ln.strip()
        for ln in text.splitlines()
        if "git push" in ln and ln.strip() != ALLOWED_PUSH
    ]


def notes_check_problems(steps):
    """Return problems with the empty-notes guard's presence and position."""

    def index_of(marker):
        return next((i for i, s in enumerate(steps) if marker in s), None)

    cliff = index_of(CLIFF_SHA)
    check = index_of("id: notes_check")
    tag = index_of("name: Create and push tag")
    if check is None:
        return ["no notes_check step"]
    problems = []
    if "-s /tmp/release-notes.md" not in steps[check]:
        problems.append("notes_check does not test -s /tmp/release-notes.md")
    if cliff is None or check < cliff:
        problems.append("notes_check is not after the git-cliff step")
    if tag is None or check > tag:
        problems.append("notes_check is not before the tag step")
    return problems


def static_problems(path):
    """Return a list of static-rule violations in the workflow at `path`."""
    with open(path, encoding="utf-8") as fh:
        text = fh.read()
    problems = []
    if "--latest" in text:
        problems.append("--latest present")
    if "CHANGELOG" in text:
        problems.append("CHANGELOG present")
    problems += [f"extra push: {p}" for p in push_problems(text)]
    problems += ordering_problems(release_job_steps(text))
    problems += notes_check_problems(release_job_steps(text))
    if text.count(CLIFF_SHA) != 1:
        problems.append("expected exactly one pinned git-cliff step")
    if "steps.range.outputs.base" not in text:
        problems.append("git-cliff args do not use the range output")
    return problems


def git_env(home):
    """Environment for fixture git: isolated HOME, identity, no GIT_* leakage."""
    env = {k: v for k, v in os.environ.items() if k not in GIT_STRIPPED}
    env.update(
        HOME=str(home),
        GIT_AUTHOR_NAME="t",
        GIT_AUTHOR_EMAIL="t@example.com",
        GIT_COMMITTER_NAME="t",
        GIT_COMMITTER_EMAIL="t@example.com",
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_CONFIG_SYSTEM=os.devnull,
    )
    return env


def git(repo, home, *args):
    """Run git in the fixture repo and return stripped stdout."""
    proc = subprocess.run(
        ["git", "-C", str(repo), *args],
        env=git_env(home),
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
        check=True,
    )
    return proc.stdout.strip()


def commit(repo, home, name):
    """Make an empty commit and return its SHA."""
    git(repo, home, "commit", "--allow-empty", "-m", name)
    return git(repo, home, "rev-parse", "HEAD")


def make_repo(tmp_path, cargo_version="1.2.3"):
    """Create a fixture repo on `main` with both crate manifests and one commit."""
    repo = tmp_path / "repo"
    for rel in CARGO_FILES:
        (repo / rel).parent.mkdir(parents=True)
        (repo / rel).write_text(f'[package]\nname = "x"\nversion = "{cargo_version}"\n')
    git(repo, tmp_path, "init", "-q", "-b", "main")
    git(repo, tmp_path, "add", "app/Cargo.toml", "lib/Cargo.toml")
    git(repo, tmp_path, "commit", "-q", "-m", "init")
    return repo


def run_step(tmp_path, repo, step_id, extra_env):
    """Run a workflow step's script under `bash -e` in `repo`."""
    env = git_env(tmp_path)
    env.update(extra_env)
    return subprocess.run(
        ["bash", "-e", "-c", extract_step_run(WORKFLOW, step_id)],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
        check=False,
    )


def validate(tmp_path, repo, version, ref="refs/heads/main"):
    """Run the validate step and return its exit code."""
    env = {"VERSION": version, "GITHUB_REF": ref}
    return run_step(tmp_path, repo, "validate", env).returncode


def run_range(tmp_path, repo):
    """Run the range step and return what it wrote to GITHUB_OUTPUT."""
    out = tmp_path / "github_output"
    out.write_text("")
    proc = run_step(tmp_path, repo, "range", {"GITHUB_OUTPUT": str(out)})
    assert proc.returncode == 0, proc.stderr
    return out.read_text()


def test_valid_version_on_main_passes(tmp_path):
    assert validate(tmp_path, make_repo(tmp_path), "1.2.3") == 0


@pytest.mark.parametrize(
    "version",
    ["v1.2.3", "1.2", "1.2.3;id", "$(id)", "1.2.3 ", "1.2.3-rc1", ""],
)
def test_bad_version_is_rejected(tmp_path, version):
    assert validate(tmp_path, make_repo(tmp_path), version) != 0


@pytest.mark.parametrize("payload", ["1.2.3;touch PWNED", "$(touch PWNED)"])
def test_metacharacters_are_not_executed(tmp_path, payload):
    repo = make_repo(tmp_path)
    assert validate(tmp_path, repo, payload) != 0
    assert not (repo / "PWNED").exists()


def test_existing_tag_is_rejected(tmp_path):
    repo = make_repo(tmp_path)
    git(repo, tmp_path, "tag", "v1.2.3")
    assert validate(tmp_path, repo, "1.2.3") != 0


def test_non_main_ref_is_rejected(tmp_path):
    repo = make_repo(tmp_path)
    assert validate(tmp_path, repo, "1.2.3", "refs/heads/feature") != 0


@pytest.mark.parametrize("which", range(len(CARGO_FILES)))
def test_either_cargo_mismatch_is_rejected(tmp_path, which):
    repo = make_repo(tmp_path)
    (repo / CARGO_FILES[which]).write_text('[package]\nversion = "9.9.9"\n')
    assert validate(tmp_path, repo, "1.2.3") != 0


def test_cargo_without_version_is_rejected(tmp_path):
    repo = make_repo(tmp_path)
    (repo / CARGO_FILES[0]).write_text("")
    assert validate(tmp_path, repo, "1.2.3") != 0


def test_range_base_is_merge_base_when_tag_is_off_main(tmp_path):
    repo = make_repo(tmp_path)
    fork = commit(repo, tmp_path, "fork point")
    git(repo, tmp_path, "checkout", "-q", "-b", "side")
    commit(repo, tmp_path, "side")
    git(repo, tmp_path, "tag", "v0.1.0")
    git(repo, tmp_path, "checkout", "-q", "main")
    commit(repo, tmp_path, "after")
    assert run_range(tmp_path, repo) == f"base={fork}\n"


def test_range_base_is_the_tagged_commit_when_tag_is_on_main(tmp_path):
    repo = make_repo(tmp_path)
    tagged = commit(repo, tmp_path, "tagged")
    git(repo, tmp_path, "tag", "v0.1.0")
    commit(repo, tmp_path, "after")
    assert run_range(tmp_path, repo) == f"base={tagged}\n"


def test_range_picks_highest_version_not_alphabetical(tmp_path):
    repo = make_repo(tmp_path)
    git(repo, tmp_path, "tag", "v0.9.0")
    newer = commit(repo, tmp_path, "newer")
    git(repo, tmp_path, "tag", "v0.10.0")
    commit(repo, tmp_path, "after")
    assert run_range(tmp_path, repo) == f"base={newer}\n"


def test_range_writes_nothing_without_tags(tmp_path):
    assert run_range(tmp_path, make_repo(tmp_path)) == ""


def test_real_workflow_has_no_static_problems():
    assert static_problems(WORKFLOW) == []


def _workflow_with(tmp_path, text):
    path = tmp_path / "release.yml"
    path.write_text(text)
    return str(path)


def test_static_rules_flag_the_old_shape(tmp_path):
    old = (
        "jobs:\n  release:\n    steps:\n"
        "      - uses: actions/checkout@v6\n"
        "      - name: Notes\n        run: git-cliff --latest\n"
        "      - name: Tag\n        run: |\n          echo $VERSION\n"
        "          git push\n          echo CHANGELOG.md\n"
    )
    problems = static_problems(_workflow_with(tmp_path, old))
    for expected in ("--latest present", "CHANGELOG present", "no validate step"):
        assert expected in problems
    assert any(p.startswith("extra push") for p in problems)


def test_static_rules_flag_version_read_before_validate(tmp_path):
    bad = (
        "jobs:\n  release:\n    steps:\n"
        "      - uses: actions/checkout@v6\n"
        "      - name: Early\n        run: echo ${VERSION}\n"
        "      - name: Validate\n        id: validate\n        run: true\n"
    )
    problems = ordering_problems(release_job_steps(bad))
    assert "validate step is not directly after checkout" in problems
    assert any("read VERSION before validate" in p for p in problems)


def _steps_with_check(order):
    blocks = {
        "cliff": f"- uses: {CLIFF_SHA}\n",
        "check": "- id: notes_check\n  run: |\n    [[ -s /tmp/release-notes.md ]]\n",
        "tag": "- name: Create and push tag\n",
        "weak": "- id: notes_check\n  run: |\n    true\n",
    }
    return [blocks[k] for k in order]


def test_notes_check_accepted_when_between_cliff_and_tag():
    assert notes_check_problems(_steps_with_check(["cliff", "check", "tag"])) == []


def test_notes_check_missing_is_flagged():
    assert notes_check_problems(_steps_with_check(["cliff", "tag"])) == [
        "no notes_check step"
    ]


def test_notes_check_must_test_the_notes_file():
    problems = notes_check_problems(_steps_with_check(["cliff", "weak", "tag"]))
    assert "notes_check does not test -s /tmp/release-notes.md" in problems


def test_notes_check_before_cliff_or_after_tag_is_flagged():
    early = notes_check_problems(_steps_with_check(["check", "cliff", "tag"]))
    late = notes_check_problems(_steps_with_check(["cliff", "tag", "check"]))
    assert "notes_check is not after the git-cliff step" in early
    assert "notes_check is not before the tag step" in late


def test_notes_check_on_no_steps():
    assert notes_check_problems([]) == ["no notes_check step"]


def test_static_rules_on_empty_workflow(tmp_path):
    assert "no validate step" in static_problems(_workflow_with(tmp_path, ""))
