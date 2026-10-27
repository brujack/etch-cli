"""Every direct download in a workflow must be pinned and checksum-verified.

A `curl`/`wget` of a release binary runs whatever the server returns. Core rule
ci-002 requires an exact version and `sha256sum -c` against the downloaded file
before anything uses it. Rules, per `run:` block containing a download:

* the URL has no `/latest/` segment;
* the download writes to a file (`-o`, `-O`, `-qO`), never into a pipe;
* a later line runs `sha256sum -c` naming that same path, and no line between
  the download and that check mentions the path.

Workflows are read as text (no PyYAML). Point DL_WORKFLOW_DIR at a directory of
older workflows to confirm the test fails against a pre-change tree.
"""

import glob
import os
import re

WORKFLOW_DIR = os.environ.get("DL_WORKFLOW_DIR", ".github/workflows")

RUN_RE = re.compile(r"^(\s*)(-\s+)?run:\s*(.*?)\s*$")
FETCH_RE = re.compile(r"\b(curl|wget)\b")
OUT_RE = re.compile(r"(?:^|\s)-[A-Za-z]*[oO]\s+(\"[^\"]+\"|'[^']+'|\S+)")


def run_blocks(text):
    """Return the body of each `run:` block as a list of lines.

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
            blocks.append([rest])
            continue
        body = []
        while i < len(lines):
            line = lines[i]
            indent = len(line) - len(line.lstrip())
            if line.strip() and indent <= key_indent:
                break
            body.append(line)
            i += 1
        blocks.append(body)
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


def check_block(body):
    """Return (number of downloads, list of rule violations) for one block."""
    lines = logical_lines(body)
    downloads = 0
    problems = []
    for idx, line in enumerate(lines):
        if not (FETCH_RE.search(line) and "https://" in line):
            continue
        downloads += 1
        if "/latest/" in line:
            problems.append(f"unpinned /latest/ URL: {line}")
        if "|" in line:
            problems.append(f"download piped instead of saved: {line}")
        out = OUT_RE.search(line)
        if not out:
            problems.append(f"download has no -o/-O output file: {line}")
            continue
        path = out.group(1).strip("\"'")
        verified = False
        for later in lines[idx + 1 :]:
            if "sha256sum -c" in later and path in later:
                verified = True
                break
            if path in later:
                problems.append(f"{path} used before checksum: {later}")
                break
        if not verified and not any("used before" in p for p in problems):
            problems.append(f"no sha256sum -c naming {path}: {line}")
    return downloads, problems


def scan_workflows():
    """Return (download count, problems) across the workflow directory."""
    total = 0
    problems = []
    for path in sorted(glob.glob(os.path.join(WORKFLOW_DIR, "*.yml"))):
        name = os.path.basename(path)
        with open(path, encoding="utf-8") as fh:
            for body in run_blocks(fh.read()):
                count, found = check_block(body)
                total += count
                problems.extend(f"{name}: {p}" for p in found)
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
