"""Every direct download in a workflow must be pinned and checksum-verified.

A `curl`/`wget` of a release binary runs whatever the server returns. Core rule
ci-002 requires an exact version and `sha256sum -c` against the downloaded file
before anything uses it. Rules, per `run:` block containing a download:

* the URL has no `/latest/` segment;
* the download writes to a file (`-o`, `-O`, `-qO`), never into a pipe;
* a later line runs `sha256sum -c` naming that same path, and no line between
  the download and that check mentions the path.

The scanner is deliberately strict and fails closed on unusual shapes: write a
download as `curl -fsSL -o <path> <https url>` then
`echo "<sha>  <path>" | sha256sum -c -`.

Workflows are read as text (no PyYAML). To check against an older tree, call
scan_workflows with a directory holding those workflows.
"""

import glob
import os
import re

WORKFLOW_DIR = ".github/workflows"

RUN_RE = re.compile(r"^(\s*)(-\s+)?run:\s*(.*?)\s*$")
FETCH_RE = re.compile(r"\b(curl|wget)\b")
OUT_RE = re.compile(r"(?:^|\s)-[A-Za-z]*[oO]\s+(\"[^\"]+\"|'[^']+'|\S+)")


def run_blocks(text):
    """Return the body of each `run:` block as a list of lines."""
    return [body for _, body in run_block_spans(text)]


def run_block_spans(text):
    """Return (set of 0-based line numbers covered, body lines) per `run:` block.

    A block is the lines after a `run: |` key that are blank or indented deeper
    than the key; it ends at the first non-blank line at the key's indent or
    shallower, so nothing leaks into the next step. A one-line `run: cmd` is a
    one-line block.
    """
    blocks = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        match = RUN_RE.match(lines[i])
        i += 1
        if not match:
            continue
        key_indent = len(match.group(1)) + (
            len(match.group(2)) if match.group(2) else 0
        )
        rest = match.group(3)
        if rest and rest[0] not in "|>":
            blocks.append(({i - 1}, [rest]))
            continue
        body = []
        start = i - 1
        while i < len(lines):
            line = lines[i]
            indent = len(line) - len(line.lstrip())
            if line.strip() and indent <= key_indent:
                break
            body.append(line)
            i += 1
        blocks.append((set(range(start, i)), body))
    return blocks


def logical_lines(body):
    """Join backslash-continued lines into single command lines."""
    joined = []
    pending = ""
    for line in body:
        stripped = line.strip()
        if stripped.endswith("\\"):
            pending += stripped[:-1] + " "
        else:
            joined.append(pending + stripped)
            pending = ""
    if pending:
        joined.append(pending)
    return joined


VERIFY_RE = re.compile(r"(?:^|\|)\s*sha256sum -c(?:\s+-)?\s*$")
PIPE_RE = re.compile(r"(?<!\|)\|(?!\|)")
GUARD_RE = re.compile(r"^\s*(?:if|while|until)\b|(?:^|\s)!\s|\|\||&|;")


def mentions(path, line):
    """Return True if line names path as a whole token, not as a substring."""
    path = path.removeprefix("./")
    pattern = rf"(?<![\w./-])(?:\./)?{re.escape(path)}(?![\w./-])"
    return re.search(pattern, line) is not None


def is_verifying(line, path):
    """Return True if line is a plain pipeline that fails the step on mismatch.

    A `sha256sum -c` inside `if`/`!`/`||`/`&&`/`;` cannot fail the step, so it
    does not count.
    """
    return (
        mentions(path, line)
        and VERIFY_RE.search(line) is not None
        and GUARD_RE.search(line) is None
    )


def check_block(body):
    """Return (number of downloads, list of rule violations) for one block.

    Any curl/wget line is a download, so a variable or http:// URL is flagged
    rather than skipped.
    """
    lines = [x for x in logical_lines(body) if not x.startswith("#")]
    downloads = 0
    problems = []
    for idx, line in enumerate(lines):
        if not FETCH_RE.search(line):
            continue
        downloads += 1
        if "https://" not in line:
            problems.append(f"no pinned https URL visible: {line}")
        if "/latest/" in line:
            problems.append(f"unpinned /latest/ URL: {line}")
        if PIPE_RE.search(line):
            problems.append(f"download piped instead of saved: {line}")
        out = OUT_RE.search(line)
        if not out:
            problems.append(f"download has no -o/-O output file: {line}")
            continue
        path = out.group(1).strip("\"'")
        verified = False
        used_early = False
        for later in lines[idx + 1 :]:
            if is_verifying(later, path):
                verified = True
                break
            if mentions(path, later):
                problems.append(f"{path} used before checksum: {later}")
                used_early = True
                break
        if not verified and not used_early:
            problems.append(f"no sha256sum -c naming {path}: {line}")
    return downloads, problems


def unclassified_fetches(text):
    """Return curl/wget lines that sit outside every parsed `run:` block.

    Fail closed: a download hidden in a shape the block parser does not
    understand (a `with: script:` value, a comment, an odd key) must be looked at
    by a person, not skipped.
    """
    covered = set()
    for lines, _ in run_block_spans(text):
        covered |= lines
    return [
        line.strip()
        for n, line in enumerate(text.splitlines())
        if n not in covered and FETCH_RE.search(line)
    ]


def workflow_files(workflow_dir=WORKFLOW_DIR):
    """Return every workflow and composite-action file, .yml and .yaml."""
    patterns = ["*.yml", "*.yaml"]
    found = []
    for pattern in patterns:
        found += glob.glob(os.path.join(workflow_dir, pattern))
    if workflow_dir == ".github/workflows":
        for action in ("action.yml", "action.yaml"):
            found += glob.glob(
                os.path.join(".github", "actions", "**", action), recursive=True
            )
    return sorted(found)


def scan_workflows(workflow_dir=WORKFLOW_DIR):
    """Return (download count, problems) across the workflow files."""
    total = 0
    problems = []
    for path in workflow_files(workflow_dir):
        name = os.path.relpath(path, os.path.dirname(workflow_dir) or ".")
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        for body in run_blocks(text):
            count, found = check_block(body)
            total += count
            problems.extend(f"{name}: {p}" for p in found)
        problems.extend(
            f"{name}: unclassified fetch: {x}" for x in unclassified_fetches(text)
        )
    return total, problems


def test_real_workflows_have_at_least_two_downloads():
    total, _ = scan_workflows()
    assert total >= 2, f"found {total} download blocks; scanner is broken"


def test_every_workflow_download_is_pinned_and_checksummed():
    _, problems = scan_workflows()
    assert not problems, "\n".join(problems)


GOOD = [
    "curl -fsSL -o /tmp/x.tgz \\",
    '  "https://example.com/v1.2.3/x.tgz"',
    'echo "${SHA}  /tmp/x.tgz" | sha256sum -c -',
    "tar -xzf /tmp/x.tgz -C /tmp",
]


def test_compliant_sample_passes():
    assert check_block(GOOD) == (1, [])


def test_missing_checksum_is_flagged():
    body = [GOOD[0], GOOD[1], GOOD[3]]
    count, problems = check_block(body)
    assert count == 1 and problems


def test_checksum_of_a_different_path_is_flagged():
    body = [GOOD[0], GOOD[1], 'echo "${SHA}  /tmp/other" | sha256sum -c -', GOOD[3]]
    count, problems = check_block(body)
    assert count == 1 and problems


def test_use_before_checksum_is_flagged():
    body = [GOOD[0], GOOD[1], GOOD[3], GOOD[2]]
    count, problems = check_block(body)
    assert count == 1 and problems


def test_pipe_into_tar_is_flagged():
    body = [
        "curl -sSL https://example.com/v1/x.tgz \\",
        "  | tar -xz -C /usr/local/bin",
    ]
    count, problems = check_block(body)
    assert count == 1 and problems


def test_latest_url_is_flagged():
    body = [
        "curl -fsSL -o /tmp/s https://example.com/cli/latest/s",
        'echo "${SHA}  /tmp/s" | sha256sum -c -',
    ]
    count, problems = check_block(body)
    assert count == 1 and any("latest" in p for p in problems)


def test_wget_short_flags_are_understood():
    body = [
        "wget -qO /tmp/g.tgz https://example.com/v1/g.tgz",
        'echo "${SHA}  /tmp/g.tgz" | sha256sum -c -',
    ]
    assert check_block(body) == (1, [])


def test_block_ends_at_next_step_so_checksum_cannot_leak():
    text = (
        "steps:\n"
        "  - name: Download\n"
        "    run: |\n"
        "      curl -fsSL -o /tmp/x https://example.com/v1/x\n"
        "  - name: Verify elsewhere\n"
        "    run: |\n"
        '      echo "${SHA}  /tmp/x" | sha256sum -c -\n'
    )
    blocks = run_blocks(text)
    assert len(blocks) == 2
    count, problems = check_block(blocks[0])
    assert count == 1 and problems


def test_one_line_run_is_a_block():
    text = "- run: curl -fsSL -o /tmp/x https://example.com/v1/x\n- name: next\n"
    assert run_blocks(text) == [["curl -fsSL -o /tmp/x https://example.com/v1/x"]]


def test_fetch_outside_a_run_block_fails_closed():
    text = (
        "steps:\n  - uses: x/y@v1\n    with:\n      script: curl https://e.com/x | sh\n"
    )
    assert unclassified_fetches(text)


def test_fetch_inside_a_run_block_is_classified():
    text = "steps:\n  - run: |\n      curl -fsSL -o /tmp/x https://e.com/x\n"
    assert unclassified_fetches(text) == []


def test_yaml_extension_is_scanned(tmp_path):
    bad = (
        "jobs:\n  j:\n    steps:\n      - run: curl -fsSL https://e.com/latest/x | sh\n"
    )
    (tmp_path / "evil.yaml").write_text(bad, encoding="utf-8")
    total, problems = scan_workflows(str(tmp_path))
    assert total == 1 and problems


def test_variable_url_download_is_flagged():
    body = ["URL=https://e.com/latest/x", 'curl -o /tmp/x "$URL"']
    count, problems = check_block(body)
    assert count == 1 and any("https" in p for p in problems)


def test_http_url_download_is_flagged():
    count, problems = check_block(["wget -O /tmp/y http://e.com/y"])
    assert count == 1 and any("https" in p for p in problems)


def test_checksum_of_a_longer_path_is_flagged():
    for other in ("/tmp/x.tgz.sha", "/tmp/x.tgz.bak"):
        body = [GOOD[0], GOOD[1], f'echo "${{SHA}}  {other}" | sha256sum -c -', GOOD[3]]
        _, problems = check_block(body)
        assert problems, other


def test_checksum_that_cannot_fail_is_flagged():
    forms = [
        'if ! echo "${SHA}  /tmp/x.tgz" | sha256sum -c -; then echo hi; fi',
        'echo "${SHA}  /tmp/x.tgz" | sha256sum -c - || true',
        'echo "${SHA}  /tmp/x.tgz" | sha256sum -c - && echo ok',
        'echo "${SHA}  /tmp/x.tgz" | sha256sum -c - ; true',
    ]
    for form in forms:
        _, problems = check_block([GOOD[0], GOOD[1], form, GOOD[3]])
        assert problems, form


def test_earlier_problem_does_not_hide_later_missing_checksum():
    bad_first = [
        "curl -fsSL -o /tmp/a https://e.com/a",
        "chmod +x /tmp/a",
        "curl -fsSL -o /tmp/b https://e.com/b",
    ]
    _, problems = check_block(bad_first)
    assert any("used before" in p for p in problems)
    assert any("/tmp/b" in p and "no sha256sum" in p for p in problems)


D = "curl -fsSL -o /tmp/x https://e.com/v1/x"


def test_comment_or_echoed_text_is_not_verification():
    for fake in ("# /tmp/x sha256sum -c -", "echo /tmp/x sha256sum -c -"):
        _, problems = check_block([D, fake])
        assert problems, fake


def test_relative_output_path_is_matched_with_or_without_dot_slash():
    body = [
        "curl -fsSL -o snyk https://e.com/v1/x",
        "./snyk auth",
        'echo "$S  snyk" | sha256sum -c -',
    ]
    assert any("used before" in p for p in check_block(body)[1])
    good = [
        "curl -fsSL -o ./snyk https://e.com/v1/x",
        'echo "$S  snyk" | sha256sum -c -',
        "./snyk auth",
    ]
    assert check_block(good) == (1, [])


def test_or_exit_is_not_a_pipe():
    body = [D + " || exit 1", 'echo "$S  /tmp/x" | sha256sum -c -']
    assert check_block(body) == (1, [])


def test_comment_mentioning_curl_is_not_a_download():
    assert check_block(["# curl is used below to fetch the binary"]) == (0, [])


def test_empty_inputs_yield_no_downloads():
    assert check_block([]) == (0, [])
    assert run_blocks("") == []


def test_empty_workflow_dir_yields_no_downloads(tmp_path):
    (tmp_path / "workflows").mkdir()
    assert scan_workflows(str(tmp_path / "workflows")) == (0, [])
