"""Cargo tools in workflows must come from a pinned, SHA-referenced install-action.

`cargo install --locked` pins the tool's dependency tree, not the tool, and compiles
it from source on every run. The replacement is `taiki-e/install-action`, referenced
by full commit SHA with the release tag in a trailing comment, installing exact
`name@X.Y.Z` versions with `fallback: none` so an unlisted version fails instead of
building or downloading unverified. Rules, scanned as text (no PyYAML):

* `cargo install` / `cargo binstall` outside a comment fails;
* each `uses: taiki-e/install-action@<40 hex>  # v2.87.28` step needs a `with:` block
  holding `tool:` (comma-separated `name@X.Y.Z`) and `fallback: none`, and never
  `checksum: false`;
* one tool is pinned to one version across every workflow.

Scan functions take the directory so a test can point them at an older tree.
"""

import glob
import os
import re

WORKFLOW_DIR = ".github/workflows"
ACTION_TAG = "v2.87.28"

USES_RE = re.compile(
    r"^(?P<indent>\s*(?:-\s+)?)uses:\s*taiki-e/install-action(?P<rest>.*)$"
)
REF_RE = re.compile(r"^@(?P<sha>[0-9a-f]{40})\s+#\s*(?P<tag>\S+)\s*$")
TOOL_RE = re.compile(r"^[a-z0-9-]+@\d+\.\d+\.\d+$")
CARGO_RE = re.compile(r"\bcargo\s+(?:install|binstall)\b")


def workflow_files(workflow_dir=WORKFLOW_DIR):
    """Return every workflow and composite-action file, .yml and .yaml."""
    found = []
    for pattern in ("*.yml", "*.yaml"):
        found += glob.glob(os.path.join(workflow_dir, pattern))
    if workflow_dir == WORKFLOW_DIR:
        for action in ("action.yml", "action.yaml"):
            found += glob.glob(
                os.path.join(".github", "actions", "**", action), recursive=True
            )
    return sorted(found)


def code_part(line):
    """Return the line without a whole-line or trailing ` #` comment."""
    if line.lstrip().startswith("#"):
        return ""
    return re.split(r"\s#", line, maxsplit=1)[0]


def cargo_install_problems(text):
    """Return one problem per non-comment `cargo install`/`cargo binstall` line."""
    return [
        f"cargo install/binstall: {line.strip()}"
        for line in text.splitlines()
        if CARGO_RE.search(code_part(line))
    ]


def indent_of(line):
    """Return the number of leading spaces."""
    return len(line) - len(line.lstrip())


def step_lines(lines, start, key_indent):
    """Return the lines after `start` that belong to the same step."""
    body = []
    for line in lines[start + 1 :]:
        if line.strip() and indent_of(line) < key_indent:
            break
        body.append(line)
    return body


def with_block(body, key_indent):
    """Return the key/value pairs of the step's `with:` block, or None."""
    for n, line in enumerate(body):
        if indent_of(line) == key_indent and line.strip() == "with:":
            pairs = {}
            for child in body[n + 1 :]:
                if child.strip() and indent_of(child) <= key_indent:
                    break
                key, sep, value = code_part(child).partition(":")
                if sep:
                    pairs[key.strip()] = value.strip().strip("\"'")
            return pairs
    return None


def check_with(pairs):
    """Return problems for a parsed `with:` block, and its tool list."""
    if not pairs:
        return ["no parseable with: block"], []
    problems = []
    if pairs.get("fallback") != "none":
        problems.append("fallback is not none")
    if pairs.get("checksum", "").lower() == "false":
        problems.append("checksum: false")
    tools = [t.strip() for t in pairs.get("tool", "").split(",") if t.strip()]
    if not tools:
        problems.append("no tool: list")
    problems += [f"tool not name@X.Y.Z: {t}" for t in tools if not TOOL_RE.match(t)]
    return problems, tools


def check_ref(rest):
    """Return problems for the text after `taiki-e/install-action`."""
    match = REF_RE.match(rest)
    if not match:
        return [f"ref is not a 40-hex SHA with a tag comment: {rest.strip()}"]
    if match.group("tag") != ACTION_TAG:
        return [f"tag comment {match.group('tag')} is not {ACTION_TAG}"]
    return []


def install_steps(text):
    """Return (line number, problems, tools) for each install-action step."""
    lines = text.splitlines()
    steps = []
    for n, line in enumerate(lines):
        match = USES_RE.match(line)
        if not match:
            continue
        key_indent = len(match.group("indent"))
        body = step_lines(lines, n, key_indent)
        problems, tools = check_with(with_block(body, key_indent))
        steps.append((n + 1, check_ref(match.group("rest")) + problems, tools))
    return steps


def scan_text(text, name):
    """Return (step count, problems, {tool: versions}) for one file."""
    problems = [f"{name}: {p}" for p in cargo_install_problems(text)]
    versions = {}
    steps = install_steps(text)
    for line_no, found, tools in steps:
        problems += [f"{name}:{line_no}: {p}" for p in found]
        for tool in tools:
            base, _, version = tool.partition("@")
            versions.setdefault(base, set()).add(version)
    return len(steps), problems, versions


def conflicts(versions):
    """Return a problem per tool pinned to more than one version."""
    return [
        f"{tool} pinned to several versions: {sorted(seen)}"
        for tool, seen in sorted(versions.items())
        if len(seen) > 1
    ]


def scan_workflows(workflow_dir=WORKFLOW_DIR):
    """Return (install-action step count, problems) across the workflow files."""
    total = 0
    problems = []
    versions = {}
    for path in workflow_files(workflow_dir):
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        count, found, seen = scan_text(text, os.path.basename(path))
        total += count
        problems += found
        for tool, vers in seen.items():
            versions.setdefault(tool, set()).update(vers)
    return total, problems + conflicts(versions)


def test_real_workflows_have_install_action_steps():
    total, _ = scan_workflows()
    assert total >= 5, f"found {total} install-action steps; scanner is broken"


def test_every_cargo_tool_is_pinned_through_install_action():
    _, problems = scan_workflows()
    assert not problems, "\n".join(problems)


SHA = "1168f6810a3cb30a3514bf5756a3b7f0422c6df8"
GOOD = (
    "jobs:\n"
    "  t:\n"
    "    steps:\n"
    "      - name: Install tools\n"
    f"        uses: taiki-e/install-action@{SHA}  # {ACTION_TAG}\n"
    "        with:\n"
    "          tool: cargo-nextest@0.9.148,cargo-machete@0.9.2\n"
    "          fallback: none\n"
    "      - name: Next\n"
    "        run: make test\n"
)


def problems_of(text):
    """Return the problems the scanner reports for one workflow text."""
    return scan_text(text, "w.yml")[1]


def test_compliant_sample_passes():
    count, problems, versions = scan_text(GOOD, "w.yml")
    assert (count, problems) == (1, [])
    assert versions == {"cargo-nextest": {"0.9.148"}, "cargo-machete": {"0.9.2"}}


def test_dash_uses_form_passes():
    text = GOOD.replace("      - name: Install tools\n        uses", "      - uses")
    text = text.replace("        with:", "        with:")
    assert problems_of(text) == []


def test_tag_ref_is_flagged():
    text = GOOD.replace(SHA, "v2")
    assert any("40-hex" in p for p in problems_of(text))


def test_wrong_tag_comment_is_flagged():
    text = GOOD.replace(f"# {ACTION_TAG}", "# v2.0.0")
    assert any("is not" in p for p in problems_of(text))


def test_missing_tag_comment_is_flagged():
    text = GOOD.replace(f"  # {ACTION_TAG}", "")
    assert any("40-hex" in p for p in problems_of(text))


def test_bare_tool_name_is_flagged():
    text = GOOD.replace("cargo-nextest@0.9.148", "cargo-nextest")
    assert any("name@X.Y.Z" in p for p in problems_of(text))


def test_missing_fallback_is_flagged():
    text = GOOD.replace("          fallback: none\n", "")
    assert any("fallback" in p for p in problems_of(text))


def test_fallback_cargo_install_is_flagged():
    text = GOOD.replace("fallback: none", "fallback: cargo-install")
    assert any("fallback" in p for p in problems_of(text))


def test_checksum_false_is_flagged():
    text = GOOD.replace(
        "fallback: none\n", "fallback: none\n          checksum: false\n"
    )
    assert any("checksum" in p for p in problems_of(text))


def test_missing_with_is_flagged():
    text = GOOD.split("        with:")[0] + "      - name: Next\n        run: x\n"
    assert any("with:" in p for p in problems_of(text))


def test_with_of_the_next_step_does_not_leak():
    text = (
        f"      - uses: taiki-e/install-action@{SHA}  # {ACTION_TAG}\n"
        "      - uses: other/action@v1\n"
        "        with:\n"
        "          tool: cargo-nextest@0.9.148\n"
        "          fallback: none\n"
    )
    assert any("with:" in p for p in problems_of(text))


def test_cargo_install_line_is_flagged():
    text = "steps:\n  - run: cargo install cargo-deny --locked\n"
    assert any("cargo install" in p for p in problems_of(text))


def test_cargo_binstall_is_flagged():
    assert problems_of("      run: cargo binstall foo\n")


def test_cargo_install_in_a_comment_is_ignored():
    text = "      # was: cargo install cargo-deny\n      run: make test  # not cargo install\n"
    assert problems_of(text) == []


def test_two_versions_of_one_tool_are_flagged(tmp_path):
    (tmp_path / "a.yml").write_text(GOOD)
    (tmp_path / "b.yml").write_text(GOOD.replace("0.9.148", "0.9.147"))
    _, problems = scan_workflows(str(tmp_path))
    assert any("cargo-nextest pinned to several versions" in p for p in problems)


def test_yaml_extension_is_scanned(tmp_path):
    (tmp_path / "w.yaml").write_text("      run: cargo install x\n")
    _, problems = scan_workflows(str(tmp_path))
    assert problems


def test_empty_text_has_nothing_to_report():
    assert scan_text("", "w.yml") == (0, [], {})


def test_empty_dir_has_nothing_to_report(tmp_path):
    assert scan_workflows(str(tmp_path)) == (0, [])
