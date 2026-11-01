# Shared Push-Content Gate — PR4 (etch-cli) Implementation Plan

> **Status: DONE** — merged as #160 (`3a01183`). V1 on 2026-10-31 used the installed pre-push hook against a local bare remote: code to main was refused `[file rule R2]`, docs to main was permitted, a feature push was permitted.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** ai-config `docs/superpowers/specs/2026-10-30-shared-push-content-gate-design.md`. The spec is cross-repo, so this plan has no `spec:` line. R13, as amended on 2026-10-31, is copied below. Spec alignment records SKIP for this PR (operator decision, 2026-10-31).

**Goal:** `scripts/pre-push` loads ai-config's shared push-content gate for every push to master or main. A new bats suite proves the wiring. It runs in `make test` and in CI's existing `test` job.

**Architecture:**

- The canonical wiring block goes into `scripts/pre-push` byte for byte, from ai-config `origin/master:docs/templates/pre-push-wiring`.
- The hook's first line becomes `#!/usr/bin/env -S bash -p`.
- The hook's own ref loop reads `${_pcg_refs}` instead of stdin.
- A new `tests/pre_push.bats` runs the hook against a throwaway git repository, with `gh` and `make` stubs first on `PATH`.
- `make test` gains `test-hooks`, which runs shellcheck on the bats file and then bats. `make lint` already shellchecks `scripts/pre-push`.
- CI's `test` job already installs a pinned, checksum-verified shellcheck and is already in `auto-merge`'s `needs:`. It gains an apt `bats` install and a `make test-hooks` step. No new job is added.

**Tech Stack:** bash, bats (apt), shellcheck 0.11.0 (already pinned in CI), GitHub Actions.

## Global Constraints

- **R13 (amended 2026-10-31).** The repo's tracked pre-push script contains the wiring block, byte-identical to ai-config's `docs/templates/pre-push-wiring` between its markers.
    - The block runs only when the script is executed, not when it is sourced.
    - It sits after any setup that must come before reading stdin, and before the script's first stdin read.
    - Every later stdin read reads `${_pcg_refs}` behind a non-empty check.
    - The script's first line is `#!/usr/bin/env -S bash -p`.
- **Block hash.** The sha256 of the text from `# >>> push-content-gate wiring` through `# <<< push-content-gate wiring` is `d556979bd64b72ca237e4bd3a8d4cf709e0db164ae155c18ab0eea86e39e221f` (ai-config `34cf3bee`). A test pins it, so any drift from the template fails.
- **No network.** No test makes a network call. Every test that runs the hook puts a `gh` stub that exits 1 first on `PATH`, and a `make` stub that records its arguments.
- **Trigger.** The only trigger change is adding `^scripts/`, so a change to the hook runs `make test`. That closes the `scripts/*.py` part of the trigger gap that CLAUDE.md and ai-config `etch-cli-ci-python-tooling.md` describe. `ruff.toml` and `Makefile` stay uncovered.
- **Rust and auto-merge.** No `.rs` file changes. A peer PR reformats every `.rs` file. `auto-merge`'s `needs:` does not change.
- **Missing tools.** In `test-hooks`, a missing `bats` or `shellcheck` prints its remedy and skips locally (core.md shell-004). It fails when `CI` is set.

## Verification Planning

After merge, run V1 on a throwaway clone. Copy only `scripts/pre-push` into `.git/hooks/pre-push`, because the pre-commit hook needs the full toolchain. Push to a local bare remote. Check that every commit succeeds before its push. Three results are expected:

- A `.sh` file pushed to main is refused with `[file rule R2]`, and the remote's main does not move.
- A `.md`-only commit pushed to main is permitted.
- A code change pushed to a feature branch is permitted.

The bats suite covers these cases:

- each trigger alternative, plus a negative (`sub/Cargo.toml`, `docs/x.md`);
- a new-branch push;
- a deletion push;
- an unresolvable range, which fails closed by running make;
- a main push whose gate cannot load, which is refused before make;
- main on a later ref line;
- a PATH `awk` or `cat`;
- an unreadable stdin;
- the shebang;
- the pinned hash.

---

### Task 1: Wire the hook and add its bats suite

```yaml-task
id: 1
description: scripts/pre-push gets the wiring block, the bash -p shebang and a ${_pcg_refs} loop; tests/pre_push.bats proves it; make test runs it via test-hooks
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: bats tests/pre_push.bats
    exit_code: 0
  - cmd: make test-hooks
    exit_code: 0
  - cmd: make lint
    exit_code: 0
  - cmd: 'test "$(sed -n "/^# >>> push-content-gate wiring$/,/^# <<< push-content-gate wiring$/p" scripts/pre-push | sha256sum | cut -d" " -f1)" = d556979bd64b72ca237e4bd3a8d4cf709e0db164ae155c18ab0eea86e39e221f'
    exit_code: 0
max_retries: 3
files_touched:
  - scripts/pre-push
  - tests/pre_push.bats
  - Makefile
depends_on: []
```

- [ ] Write `tests/pre_push.bats` first.
    - Model it on brucejacksonconsulting-site `tests/pre_push.bats` (`git -C ~/git-repos/personal/brucejacksonconsulting-site show origin/main:tests/pre_push.bats`): its `setup()` (`mktemp -d` under `BATS_TEST_TMPDIR`, redirected `HOME` and `XDG_CONFIG_HOME`, `GIT_*` unset, `GIT_CONFIG_GLOBAL=/dev/null`, `gh` and `make` stubs), its helpers, and its `set -m` stdin watchdog.
    - Adapt the trigger table to this hook's regex: `x.rs`, `Cargo.toml`, `Cargo.lock`, `tests/x.py`, `scripts/x.py`. The negatives are `sub/Cargo.toml` and `docs/x.md`.
    - Add an etch-cli-specific test: an unresolvable `remote_sha`, a real-looking sha the fixture lacks, runs make and prints `cannot resolve`.
    - Run it and watch the wiring tests fail on the unchanged hook.
- [ ] In `scripts/pre-push`:
    - Change line 1 to `#!/usr/bin/env -S bash -p`.
    - Paste the block after `set -e` and before the `REPO_ROOT` lines, marker to marker.
    - Wrap the loop in `if [[ -n "${_pcg_refs}" ]]; then ... fi` and end it with `done <<< "${_pcg_refs}"`.
    - Add `|^scripts/` to the trigger regex.
    - Update the header comment to say it also judges pushes to master or main with the shared gate.
- [ ] In the `Makefile`:
    - Add `test-hooks` to `.PHONY`.
    - Make `test: lint test-hooks`.
    - Add a `test-hooks` target. If `shellcheck` or `bats` is missing, print the remedy (`CI pins shellcheck 0.11.0; apt may differ`) and skip with exit 0, unless `CI` is set, in which case exit 1. Otherwise run `shellcheck --severity=warning tests/pre_push.bats`, then `bats tests/pre_push.bats < /dev/null`.
- [ ] Gates, then commit with `feat(pre-push): load the shared push-content gate`.

### Task 2: CI steps and docs

```yaml-task
id: 2
description: CI's existing test job installs bats and runs make test-hooks; CLAUDE.md and the trigger-gap note reflect the gate (CI config and docs, verified by the Task 1 suite in CI)
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: 'grep -q "make test-hooks" .github/workflows/ci.yml'
    exit_code: 0
  - cmd: 'grep -q "push-content gate" CLAUDE.md'
    exit_code: 0
  - cmd: python3 -m pytest tests/ -q
    exit_code: 0
max_retries: 2
files_touched:
  - .github/workflows/ci.yml
  - CLAUDE.md
depends_on: [1]
```

- [ ] In `.github/workflows/ci.yml`'s `test` job, after "Install shellcheck (pinned)", add two steps: `Install bats` (`sudo apt-get install -y bats`) and `Run hook tests` (`make test-hooks`). Leave `auto-merge`'s `needs:` unchanged. Add no `continue-on-error` (`tests/test_continue_on_error.py`).
- [ ] In `CLAUDE.md`:
    - The trigger-gap bullet now reads: `scripts/pre-push` does not trigger on `ruff.toml` or `Makefile`.
    - The `Pre-push hook:` line says it runs `make test`, which includes the hook's bats suite. Pushes to main are judged by ai-config's shared push-content gate (ADR-0091), loaded from GitHub and refused when it cannot load.
- [ ] Commit with `ci: run the pre-push hook tests`.

The orchestrator updates ai-config `docs/knowledge/etch-cli-ci-python-tooling.md` "The pre-push trigger gap" after merge, as a docs-only ai-config commit. It runs `make test` once after Task 2.

## Non-goal check

Run 2026-10-31 against the spec's six effective non-goals (one reviewer-role subagent, `nongoal-check.md`): N1–N6 all CLEAR, 0 CONFLICT, 0 UNCLEAR, no plan change. The hook keeps only its `make test` trigger loop; content judgement comes from the shared gate (N2). The block is copied from the template at authoring time and pinned by hash (N5).
