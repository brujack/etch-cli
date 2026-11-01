# Rust Standards and Edition 2024 Implementation Plan

spec: docs/cursor/specs/2026-10-31-rust-standards-edition-2024-spec.md

> **Status: DONE** — PR1 etch-cli#159 (ce5cbf6), PR2 etch-cli#161 (1a1ca4b), merged 2026-10-31/11-01. V1-V10 verified (V4, V9 on the Mac Studio after merge).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce the whole `deny.toml` policy, retire cargo-audit without losing unsound coverage, guard `unsafe`, fix `command.run` env handling, and move the workspace to edition 2024.

**Architecture:** PR1 runs Tasks 1-12 on one feature branch in one worktree, sequentially, one commit per task. Process-env mutation leaves production code (Task 6) and test code (Tasks 7-10, through `EnvGuard`), so the edition bump (Task 11) finds no `set_var` sites. PR2 (Task 13) is the style-edition reformat alone, opened after PR1 merges.

**Tech Stack:** Rust (cargo, clippy, rustfmt, cargo-nextest, cargo-deny 0.20.2, `which` 8.0.2, `serial_test`), pytest, GitHub Actions.

## Global Constraints

- cargo-deny version is `0.20.2` in every workflow (`tests/test_tool_installs.py` enforces one version per tool). Install only through `taiki-e/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8  # v2.87.28` with `fallback: none`.
- `cargo deny --config <file>` puts `--config` before `check`; after it, 0.20.2 exits 2.
- No `rust-version` anywhere and no MSRV job (N1). Workspace `resolver` stays `"2"` (N5). No lint change in `fuzz/` (N6). Job id `cargo-audit` is not renamed (N7).
- No `#[allow(unsafe_code)]` anywhere; every `unsafe` block carries `#[expect(unsafe_code, reason = "...")]` (N4).
- No change to atom or action public fields or signatures (N3). No `.git-blame-ignore-revs` (N9). Do not fix `exec.rs:157`'s dead `elevate()` check (N10).
- Tests that run `etch apply` set `ETCH_STATE_DIR` to a tempdir (use `apply_with_state` in `app/tests/integration.rs`).
- Cargo needs `export PATH="$HOME/.cargo/bin:$PATH"` in agent shells.
- Every commit message comes from `caveman:caveman-commit`, ends with the session's Co-Authored-By and Claude-Session trailers.
- **Aggregate gate** for every code task: `make test` (lint + nextest + pytest) **and** CI's Python coverage command `pytest tests/ -q --cov=scripts --cov-fail-under=90`, which no local hook runs.

## Verification Planning

- **Whole change:** `make test` green, `pytest tests/ -q --cov=scripts --cov-fail-under=90` green, `cargo deny check advisories licenses bans sources` prints `advisories ok, bans ok, licenses ok, sources ok`, and `git grep -nE 'env::(set_var|remove_var)' -- '*.rs'` lists only `lib/src/test_helpers.rs`. CI on PR1: every job in `auto-merge`'s `needs:` green, the `cargo-audit` log shows `licenses ok` and `sources ok` (V1).
- **Behaviour:** R13 integration cases (a), (b), (c) fail on `main` and pass after Task 6; (d) passes on both and fails with `which_in` reverted (V2, V6).
- **Edge cases:** threaded runner (`cargo test -p etch-lib`) passes, not only nextest; macOS build and tests on the Mac Studio (V4, V9 run by the operator); fuzz build on nightly (V5).
- **Orchestrator-run evidence (not task gates):** V8 per-file counts from Task 11; V3 drop-order table from Task 11; V10 two-lockfile comparison (step O1 below); V7 coverage from CI.

## Orchestrator steps

- [ ] **O0.** Create worktree `../etch-cli-wt-rust-standards` on branch `feat/rust-standards-edition-2024` from `origin/main`. Every subagent prompt starts with `**IMPORTANT: All work must happen in /home/bruce/git-repos/personal/etch-cli-wt-rust-standards/. Do not cd anywhere else.**` Run `make test` once in the new worktree before Task 1 to warm `target/` (warm suite measured at 76 s on `claude`; a cold build is longer), so task gates stay under the 600 s Bash cap.
- [ ] **O1 (V10, before Task 2).** On the untouched `main` lockfile, in a scratch copy: `cargo audit` lists `RUSTSEC-2026-0306` and `RUSTSEC-2026-0205` as `unsound`; `cargo deny --config <copy with unsound = "all">` check advisories reports exactly those two `error[unsound]`. Save both outputs for the PR body. Re-run both on the PR1 lockfile after Task 2: neither ID appears.
- [ ] **O2.** After Task 6, push the branch and open the PR as a draft: `gh pr create --repo brujack/etch-cli --base main --draft`. Push after each later task.
- [ ] **O3.** After Task 12: Phase 3 (`finishing-a-development-branch`), PR body carries V2/V3/V6/V8/V10 evidence, then `gh pr ready <n> --repo brujack/etch-cli` and `gh pr checks <n> --repo brujack/etch-cli --watch`.
- [ ] **O4.** Ask the operator to run V4 and V9 on the Mac Studio (no ssh alias resolves from `claude`).
- [ ] **O5.** After PR1 merges: Task 13 on a fresh branch `chore/style-edition-2024`, PR2.

---

### Task 1: Enforce licenses, bans and sources in CI

```yaml-task
id: 1
description: The cargo-audit job runs the full cargo-deny check; CI-config change with no unit under test, proven by the existing workflow tests and a grep.
role: executor
model: sonnet
tdd: not-applicable
requirements: [R1]
acceptance:
  - cmd: 'grep -q "run: cargo deny check advisories licenses bans sources" .github/workflows/ci.yml'
    exit_code: 0
  - cmd: 'grep -qE "^  cargo-audit:$" .github/workflows/ci.yml'
    exit_code: 0
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo deny check advisories licenses bans sources'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 2
files_touched: [.github/workflows/ci.yml]
depends_on: []
```

**Files:** `.github/workflows/ci.yml` lines 158-173 (`cargo-audit` job).

- [ ] Change the step `run:` to `cargo deny check advisories licenses bans sources`. Rename the step to `Run cargo deny (advisories, licenses, bans, sources)`. Replace the comment with: `# Reads all of deny.toml. The job id stays cargo-audit: it is the required-check name.`
- [ ] Keep the job id `cargo-audit` and its entry in `auto-merge`'s `needs:`.
- [ ] Run the gates. The base tree fails gate 1 (current text is `check advisories` only).
- [ ] Commit.

**Interfaces:** Produces: CI job `cargo-audit` running all four checks.

### Task 2: Unsound coverage, dependency bumps, policy tests

```yaml-task
id: 2
description: deny.toml gains unmaintained/unsound = all; serial_test 4 and lockfile bumps clear both unsound advisories; tests/test_deny_policy.py proves the policy loads and rejects; test job installs cargo-deny.
role: executor
model: sonnet
tdd: required
requirements: [R2, R5, R6, R7]
acceptance:
  - cmd: 'python3 -m pytest tests/test_deny_policy.py -q'
    exit_code: 0
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo deny check advisories licenses bans sources'
    exit_code: 0
  - cmd: '! command grep -qE "^name = \"scc\"" Cargo.lock'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_tool_installs.py -q'
    exit_code: 0
  - cmd: make test
    exit_code: 0
  - cmd: 'python3 -m pytest tests/ -q --cov=scripts --cov-fail-under=90'
    exit_code: 0
max_retries: 3
files_touched: [deny.toml, lib/Cargo.toml, Cargo.lock, tests/test_deny_policy.py, tests/fixtures/deny/gpl-fixture/Cargo.toml, tests/fixtures/deny/gpl-fixture/src/lib.rs, tests/fixtures/deny/mit-fixture/Cargo.toml, tests/fixtures/deny/mit-fixture/src/lib.rs, .github/workflows/ci.yml, docs/superpowers/README.md]
depends_on: [1]
```

**RED first:** write `tests/test_deny_policy.py` and the fixtures before the `deny.toml` and lockfile edits. RED evidence: the unit tests for `_cargo_deny()` fail until it exists; run the fixture cases against the base `deny.toml` (they pass there; the mutation controls below are what prove they can fail). The one-list case belongs to Task 4.

**Test file contents (`tests/test_deny_policy.py`):**

- `REPO = Path(__file__).resolve().parent.parent`; `FIXTURES = REPO / "tests/fixtures/deny"`.
- `_cargo_deny()`: `shutil.which("cargo-deny")`; if `None`, `pytest.fail("cargo-deny missing under CI")` when `os.environ.get("CI")`, else `pytest.skip("cargo-deny not installed")` (R6).
- `_run(fixture, tmp_path)`: `shutil.copytree(FIXTURES / fixture, tmp_path / fixture)`; run `["cargo", "deny", "--config", str(REPO / "deny.toml"), "--manifest-path", str(tmp_path / fixture / "Cargo.toml"), "check", "licenses"]`, `capture_output=True, text=True`.
- `test_gpl_fixture_is_rejected`: returncode != 0 and `"error[rejected]"` in stdout+stderr.
- `test_mit_fixture_is_accepted`: returncode == 0. Docstring: this case proves `deny.toml` loaded, since cargo-deny's built-in fallback config has an empty allow list.
- Two unit tests for `_cargo_deny()` with `monkeypatch` (`shutil.which` returning `None`, `CI` set / unset) asserting fail / skip (`pytest.raises(pytest.fail.Exception)` / `pytest.skip.Exception`).

**Fixtures:** `Cargo.toml` with `[package] name = "gpl-fixture"` (resp. `mit-fixture`), `version = "0.1.0"`, `edition = "2021"`, `publish = false`, `license = "GPL-3.0-only"` (resp. `"MIT"`), and an empty `[workspace]` table so cargo never treats it as a member of the repo workspace; `src/lib.rs` empty.

**Then:**

- [ ] `deny.toml` `[advisories]`: add `unmaintained = "all"` and `unsound = "all"` after `db-urls`, with a comment: `# "workspace" (the unsound default) drops unsound advisories in transitive crates`.
- [ ] `lib/Cargo.toml`: dev-dependency `serial_test = "4"`. Run `cargo update -p serial_test -p faster-hex -p autocfg`. Expect 5 packages locked, `scc` and `sdd` removed. If either advisory still shows, STOP and report a blocker (the spec permits an ignore only by amendment).
- [ ] `ci.yml` `test` job: add `cargo-deny@0.20.2` to the existing install step's `tool:` list (`cargo-machete@0.9.2,cargo-nextest@0.9.148,cargo-deny@0.20.2`) (R7).
- [ ] `docs/superpowers/README.md`: delete the backlog row starting `| P3 | Raw \`cargo audit\` advisories \`cargo deny\` skips |`.
- [ ] Mutation controls for the PR body (V2): relabel `gpl-fixture` MIT in a scratch copy and see `test_gpl_fixture_is_rejected` fail; point `--config` at an empty file in a scratch run and see the MIT case fail. Report both outputs.
- [ ] Gates, commit.

### Task 3: `unsafe_code = "deny"` with reasoned expects

```yaml-task
id: 3
description: Deny unsafe_code in lib, app, jsonschemagen; tag each existing unsafe block with #[expect(unsafe_code, reason)].
role: executor
model: sonnet
tdd: required
requirements: [R8, R9]
acceptance:
  - cmd: 'for f in lib/Cargo.toml app/Cargo.toml jsonschemagen/Cargo.toml; do grep -qx "unsafe_code = \"deny\"" "$f" || exit 1; done'
    exit_code: 0
  - cmd: '! command grep -rn "allow(unsafe_code" lib/src app/src jsonschemagen/src'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
files_touched: [lib/Cargo.toml, app/Cargo.toml, jsonschemagen/Cargo.toml, lib/src/actions/file/flags.rs, lib/src/actions/plugin/mod.rs, lib/src/atoms/file/chflags.rs, lib/src/atoms/git/clone.rs, lib/src/manifests/providers/git.rs, lib/src/atoms/git/config_unset.rs]
depends_on: [2]
```

**RED:** add `unsafe_code = "deny"` to each `[lints.rust]` table first and run `cargo clippy --all-targets -- -D warnings`; it fails on every site below. That failure is the test.

- [ ] Sites (8 production blocks): `actions/file/flags.rs:59` and `:60` (`std::mem::zeroed`, `libc::stat`), `actions/plugin/mod.rs:23` (`Lua::unsafe_new`), `atoms/file/chflags.rs:32`, `:33`, `:70` (`zeroed`, `libc::stat`, `libc::chflags`), `atoms/git/clone.rs:73` and `manifests/providers/git.rs:73` (`gix::interrupt::init_handler`). Put `#[expect(unsafe_code, reason = "<FFI call>: <why sound>")]` on the smallest enclosing statement or item. Reasons name the call and the invariant (e.g. `"libc::stat on a NUL-terminated CString from a valid path; sb is a zeroed out-param"`).
- [ ] `atoms/git/config_unset.rs` test blocks (~line 216 and 220): tag each with `#[expect(unsafe_code, reason = "temporary; replaced by EnvGuard in Task 8")]`.
- [ ] macOS-only sites (`flags.rs`, `chflags.rs`) are `#[cfg(target_os = "macos")]`; Linux clippy cannot check them, so report this for V4.
- [ ] Mutation control (V2): delete one `#[expect]`, run `make lint`, report the failing line, restore.
- [ ] Gates, commit.

### Task 4: Scheduled cargo-deny replaces scheduled cargo-audit

```yaml-task
id: 4
description: Delete cargo-audit-scheduled.yml, add cargo-deny-scheduled.yml with the same cron, and add the one-ignore-list test.
role: executor
model: sonnet
tdd: required
requirements: [R4, R10]
acceptance:
  - cmd: 'test ! -e .github/workflows/cargo-audit-scheduled.yml'
    exit_code: 0
  - cmd: 'grep -q "cron: \"0 6 \* \* 1\"" .github/workflows/cargo-deny-scheduled.yml && grep -q "run: cargo deny check advisories" .github/workflows/cargo-deny-scheduled.yml'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_deny_policy.py -q -k one_ignore_list'
    exit_code: 0
  - cmd: make test
    exit_code: 0
  - cmd: 'python3 -m pytest tests/ -q --cov=scripts --cov-fail-under=90'
    exit_code: 0
max_retries: 2
files_touched: [.github/workflows/cargo-audit-scheduled.yml, .github/workflows/cargo-deny-scheduled.yml, tests/test_deny_policy.py]
depends_on: [3]
```

- [ ] `git rm .github/workflows/cargo-audit-scheduled.yml`.
- [ ] New `.github/workflows/cargo-deny-scheduled.yml`: `name: cargo-deny (scheduled)`, `on: schedule: - cron: "0 6 * * 1"` with comment `# Weekly: catches advisories published against an unchanged lockfile`, the same `env:` block, one job `deny` on `ubuntu-latest` with steps `actions/checkout@v6`, the pinned install-action (`tool: cargo-deny@0.20.2`, `fallback: none`), and `run: cargo deny check advisories`. No Rust toolchain or cache step is needed for advisories; if cargo-deny needs `cargo metadata`, add `dtolnay/rust-toolchain@stable` as the old file did.
- [ ] **RED first:** add `test_no_workflow_carries_its_own_ignore_list` (name contains `one_ignore_list`: call it `test_one_ignore_list_no_workflow_has_its_own`) to `tests/test_deny_policy.py`: `files = sorted((REPO / ".github/workflows").glob("*.y*ml"))`; `assert files`; no file text contains `--ignore RUSTSEC`. Run it: fails on `cargo-audit-scheduled.yml`. Then do the workflow swap; it passes.
- [ ] Mutation control (V2): add `--ignore RUSTSEC-0000-0000` to a scratch copy of a workflow under a tmp `REPO` and see it fail; report.
- [ ] Gates, commit.

### Task 5: `Exec` resolves on the step's own `env:` PATH

```yaml-task
id: 5
description: When not elevated and environment carries PATH, Exec resolves the program with which::which_in against that PATH and the etch cwd.
role: executor
model: sonnet
tdd: required
requirements: [R3]
acceptance:
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo nextest run -p etch-lib atoms::command::exec'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
files_touched: [lib/src/atoms/command/exec.rs, lib/src/utilities/mod.rs]
depends_on: [4]
```

**Interfaces:** Produces `pub fn get_binary_path_in(binary: &str, path: &std::ffi::OsStr) -> Result<String, anyhow::Error>` in `lib/src/utilities/mod.rs` (calls `which::which_in(binary, Some(path), std::env::current_dir()?)`), and private `fn is_elevated(&self) -> bool` on `Exec` (`self.privileged && whoami::username() != "root"`, the `(true, _)` arm of `elevate_if_required`, which must then use it).

**RED (in `exec.rs` tests):**

- `execute_resolves_program_on_environment_path`: tempdir with executable script `etch-test-only-prog` (`#!/bin/sh\nexit 0`, mode 0o755); `Exec { command: "etch-test-only-prog", environment: vec![("PATH", "<tmp>:<std::env::var(PATH)>")], privileged: false, .. }`; `execute()` is `Ok`. Fails on base with "not found in path".
- `execute_ignores_environment_path_when_unset`: `environment` empty, `command: "sh"`, `arguments: ["-c", "exit 0"]`: `Ok`.
- `is_elevated` table test: (false, any) false; privileged true is true unless running as root (assert `== (whoami::username().unwrap() != "root")`).

**N10 boundary:** `elevate_if_required` (`exec.rs:61-89`) only changes its match guard to call `is_elevated()`. The dead provider comparison `command.eq("doas") || command.eq("sudo") || command.eq("run0")` sits in `execute()` (`exec.rs:157`), after resolution; keep that line byte-for-byte.

**GREEN:** in `execute()`, replace `utilities::get_binary_path(&command)` with: if `!self.is_elevated()` and `self.environment` has key `"PATH"`, `utilities::get_binary_path_in(&command, value)`, else `get_binary_path(&command)`. Keep the error text `Command \`{command}\` not found in path`.

- [ ] Mutation control (V2, for the PR body): revert to `get_binary_path` and see `execute_resolves_program_on_environment_path` fail; restore.
- [ ] Gates, commit.

### Task 6: `command.run` env goes to `Exec.environment`

```yaml-task
id: 6
description: RunCommand::plan fills Exec.environment (sorted) and drops SetEnvVars/RemoveEnvVars; integration cases (a)-(d) and docs.
role: executor
model: sonnet
tdd: required
requirements: [R11, R12, R13, R19]
acceptance:
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo nextest run -p etch-cli --test integration command_run_env'
    exit_code: 0
  - cmd: '! command grep -rnE "SetEnvVars|RemoveEnvVars" lib/src app/src'
    exit_code: 0
  - cmd: 'test ! -e lib/src/steps/initializers/env_vars_set.rs && test ! -e lib/src/steps/finalizers/env_vars_remove.rs'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
files_touched: [lib/src/actions/command/run.rs, lib/src/steps/initializers/mod.rs, lib/src/steps/initializers/env_vars_set.rs, lib/src/steps/finalizers/mod.rs, lib/src/steps/finalizers/env_vars_remove.rs, app/tests/integration.rs, docs/src/command.md]
depends_on: [5]
```

**RED first, on the unmodified code** (write all tests, run, record output for V6):

- Unit (`run.rs` tests): `plan` with `env: {B: etch-secret-b, A: etch-secret-a}` yields one step; `format!("{:?}", step.atom)` contains `"A"` before `"B"` and contains neither `etch-secret-a` nor `etch-secret-b` (proves sorted, routed to `Exec.environment`, redacted, without a new accessor, per N3); `step.initializers.len()` is 0 without `skip_if_exists` and 1 with it; `step.finalizers` is empty. Update the existing count test at `run.rs:~269` (comment `SetEnvVars (Ensure) + FileExists (SkipIf) = 2 initializers`) to expect 1.
- Integration (`app/tests/integration.rs`, new section `// ─── command.run env ───`, names prefixed `command_run_env_`). Each runs `etch` via a new helper `apply_with_state_env(dir, state, &[("ETCH_SPEC_PROBE", "orig")])` (copy of `apply_with_state` adding `.envs`). Probe action: `command: sh`, `args: ["-c", "printf %s \"${ETCH_SPEC_PROBE-unset}\" > <tmp>/probe"]`; the test reads `<tmp>/probe` (missing file fails).
    - `command_run_env_restores_inherited_value` (a): action 1 `env: {ETCH_SPEC_PROBE: x}` runs `true`; probe reads `orig`. Base: `unset`.
    - `command_run_env_does_not_leak_after_failure` (b): action 1 same env, `command: false`; probe reads `orig`; run exits non-zero. Base: `x`.
    - `command_run_env_path_does_not_break_later_actions` (c): write executable `<tmp>/ok` (`#!/bin/sh\nexit 0`); action 1 `env: {PATH: <tmp>}`, `command: <tmp>/ok` (absolute); action 2 is the probe, resolving `sh` by name; probe file reads `orig`. Base: action 2 fails with "Command `sh` not found in path" (the finalizer removed `PATH`).
    - `command_run_env_path_resolves_same_step_program` (d): action 1 `env: {PATH: "<tmp>:<real PATH>"}`, `command: etch-spec-mock` (script only in `<tmp>`, writes `<tmp>/ran`); `<tmp>/ran` exists. Passes on base.

**GREEN:** `plan` sets `environment: { let mut v: Vec<_> = self.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect(); v.sort(); v }` and builds `initializers` without `SetEnvVars`, `finalizers: vec![]`. Delete `env_vars_set.rs`, `env_vars_remove.rs` and their `mod`/`pub use` lines.

- [ ] `docs/src/command.md`: under `env:` state the three R19 facts.
- [ ] Report the base-tree failures of (a), (b), (c) verbatim for V6.
- [ ] Gates, commit.

### Task 7: `EnvGuard` and the source check

```yaml-task
id: 7
description: Add test-only EnvGuard (one unsafe block) and tests/test_env_guard_serial.py with must-flag and must-not-flag fixtures.
role: executor
model: sonnet
tdd: required
requirements: [R14, R16]
acceptance:
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo nextest run -p etch-lib test_helpers'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_env_guard_serial.py -q'
    exit_code: 0
  - cmd: make test
    exit_code: 0
  - cmd: 'python3 -m pytest tests/ -q --cov=scripts --cov-fail-under=90'
    exit_code: 0
max_retries: 3
files_touched: [lib/src/test_helpers.rs, tests/test_env_guard_serial.py]
depends_on: [6]
```

**`EnvGuard`** (in `lib/src/test_helpers.rs`): exact API and doc text from spec Design 6: `#[must_use = "bind to a named variable (let _g = ...); let _ = drops and restores at once"]`, `#[derive(Debug)]`, fields `key: OsString`, `previous: Option<OsString>`; `set`, `remove`, `prepend_path(dir: &Path)` (joins `dir` before current `PATH` with `std::env::join_paths`); `Drop` restores. One private `fn apply(key: &OsStr, value: Option<&OsStr>)` holds the only `unsafe` block under `#[expect(unsafe_code, reason = "...")]` with the SAFETY note from spec Design 6 (nextest process-per-test sound; threaded runners: `#[serial]` orders guard holders, non-serial env readers remain a residual race, unchanged from bare `set_var`, out of scope).

**Rust RED tests** (each `#[test] #[serial]`, unique key names `ETCH_ENVGUARD_T_*`): set restores a previous value; set restores absence; remove restores value; prepend_path puts dir first and restores; restore happens on panic (`std::panic::catch_unwind` around a closure holding a guard).

**Python check** `tests/test_env_guard_serial.py`: function `scan(source: str, path: str) -> list[str]` returning violations, plus `test_real_tree_is_clean` over `lib/src/**/*.rs`. Body extraction uses a brace counter skipping `"..."` (with escapes), `r#"..."#` raw strings, `'c'` char literals (not lifetimes `'a`), `//` and `/* */` comments. Rules exactly as spec Design 6 bullets. Fixtures (inline strings): must-flag — alias import, non-test helper naming `EnvGuard`, `#[test]` without serial, `let _ = EnvGuard::set(..)`, `let _ = crate::test_helpers::EnvGuard::set(..)`, `drop(EnvGuard::set(..))`; must-not-flag — `#[serial]` before `#[test]`, `#[serial(key)]`, `#[serial_test::serial]`, module-level `use crate::test_helpers::EnvGuard;`, closure inside a test body, a string literal `"{{{"` before a serial test, the `impl EnvGuard`/`impl Drop for EnvGuard` blocks under path `lib/src/test_helpers.rs`.

- [ ] Gates, commit.

### Task 8: Migrate atoms to `EnvGuard`

```yaml-task
id: 8
description: Replace set_var/remove_var and manual restores with EnvGuard in atom tests; drop Task 3's temporary expects.
role: executor
model: sonnet
tdd: not-applicable
requirements: [R15]
acceptance:
  - cmd: '! command grep -nE "env::(set_var|remove_var)" lib/src/atoms/systemd/service.rs lib/src/atoms/macos/service.rs lib/src/atoms/git/pull.rs lib/src/atoms/git/clone.rs lib/src/atoms/git/config_unset.rs lib/src/atoms/file/stash.rs'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_env_guard_serial.py -q'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
files_touched: [lib/src/atoms/systemd/service.rs, lib/src/atoms/macos/service.rs, lib/src/atoms/git/pull.rs, lib/src/atoms/git/clone.rs, lib/src/atoms/git/config_unset.rs, lib/src/atoms/file/stash.rs]
depends_on: [7]
```

Test-only refactor: behaviour preserved, so the existing tests are the oracle (TDD waived; the tests themselves are the edited code).

- [ ] Pattern: `let original_path = std::env::var("PATH")...; std::env::set_var("PATH", format!("{}:{}", dir, original_path)); ...; std::env::set_var("PATH", &original_path);` becomes `let _path = EnvGuard::prepend_path(dir);` at the top of the scope, restore lines deleted. A whole-value replacement uses `EnvGuard::set`. Import `use crate::test_helpers::EnvGuard;` in each `mod tests`.
- [ ] `config_unset.rs`: replace both `unsafe` blocks with `let _git_dir = EnvGuard::set("GIT_DIR", "/nonexistent/.git");` scoped around `atom.execute()`; remove their temporary `#[expect]`s.
- [ ] `macos/service.rs` is macOS-gated: edit by reading, confirm `cargo check` on Linux is unaffected, and list it for V4.
- [ ] Every test touched keeps or gains `#[serial]`.
- [ ] Gates, commit.

### Task 9: Migrate actions to `EnvGuard`

```yaml-task
id: 9
description: Replace set_var/remove_var and manual restores with EnvGuard in action tests (pyenv, claude, npm, pip, gem, powershell).
role: executor
model: sonnet
tdd: not-applicable
requirements: [R15]
acceptance:
  - cmd: '! command grep -nE "env::(set_var|remove_var)" lib/src/actions/pyenv/virtualenv.rs lib/src/actions/pyenv/install.rs lib/src/actions/claude/install.rs lib/src/actions/claude/upgrade.rs lib/src/actions/claude/marketplace.rs lib/src/actions/npm/install.rs lib/src/actions/pip/install.rs lib/src/actions/gem/install.rs lib/src/actions/powershell/module.rs'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_env_guard_serial.py -q'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
files_touched: [lib/src/actions/pyenv/virtualenv.rs, lib/src/actions/pyenv/install.rs, lib/src/actions/claude/install.rs, lib/src/actions/claude/upgrade.rs, lib/src/actions/claude/marketplace.rs, lib/src/actions/npm/install.rs, lib/src/actions/pip/install.rs, lib/src/actions/gem/install.rs, lib/src/actions/powershell/module.rs]
depends_on: [8]
```

Test-only refactor; existing tests are the oracle. Same pattern as Task 8. `pyenv/virtualenv.rs:~327` (`PATH=/nonexistent`) uses `EnvGuard::set("PATH", "/nonexistent")`.

- [ ] Gates, commit.

### Task 10: Migrate the rest; inline closure helpers

```yaml-task
id: 10
description: Migrate user/group providers, doctor, state, rollback, contexts to EnvGuard; inline four closure helpers; add #[serial] to two unserialised env tests.
role: executor
model: sonnet
tdd: not-applicable
requirements: [R15, R20]
acceptance:
  - cmd: 'git grep -lE "env::(set_var|remove_var)" -- "*.rs"'
    exit_code: 0
    stdout_match: '^lib/src/test_helpers\.rs$'
  - cmd: '! command grep -rnE "fn (with_mock_bins|with_mock_id|with_failing_id)" lib/src'
    exit_code: 0
  - cmd: 'python3 -m pytest tests/test_env_guard_serial.py -q'
    exit_code: 0
  - cmd: make test
    exit_code: 0
  - cmd: 'export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p etch-lib'
    exit_code: 0
max_retries: 3
files_touched: [lib/src/actions/user/providers/linux.rs, lib/src/actions/user/providers/macos.rs, lib/src/actions/group/providers/linux.rs, lib/src/doctor/versions.rs, lib/src/doctor/tools.rs, lib/src/state/mod.rs, lib/src/rollback/mod.rs, lib/src/contexts/mod.rs]
depends_on: [9]
```

Test-only refactor; existing tests are the oracle. Same pattern as Task 8.

- [ ] Inline `with_mock_bins` (`group/providers/linux.rs:58`, `user/providers/linux.rs:143`), `with_mock_id` and `with_failing_id` (`user/providers/macos.rs:143`, `:158`) into their 9 callers: each caller builds its own guard.
- [ ] Add `#[serial]` to the env-mutating tests at `contexts/mod.rs:~170` and `state/mod.rs:~254`.
- [ ] The last gate runs the threaded runner (`cargo test`), proving the guards hold without nextest's process isolation.
- [ ] Gates, commit.

### Task 11: Edition 2024

```yaml-task
id: 11
description: Record the 2021-tree pre-check, bump all four crates to edition 2024 with style_edition 2021 pinned, apply cargo fix, classify drop-order sites, fix new clippy lints.
role: executor
model: sonnet
tdd: not-applicable
requirements: [R17]
acceptance:
  - cmd: 'for f in app/Cargo.toml lib/Cargo.toml jsonschemagen/Cargo.toml fuzz/Cargo.toml; do grep -qx "edition = \"2024\"" "$f" || exit 1; done'
    exit_code: 0
  - cmd: 'grep -qx "style_edition = \"2021\"" rustfmt.toml'
    exit_code: 0
  - cmd: 'grep -q "resolver = \"2\"" Cargo.toml'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 3
depends_on: [10]
```

Edition migration; compilation and the existing suite are the gate (no new behaviour to test first). `files_touched` unconstrained: `cargo fix` and clippy fixes reach files not known in advance.

- [ ] **Pre-check (V8), before any edit:** `RUSTFLAGS="-W rust-2024-compatibility" CARGO_TARGET_DIR=<scratch> cargo check --workspace --all-targets`; report per-file counts of each warning kind. Expected: 0 `set_var`/`remove_var`; 4 `gen` (`app/src/commands/gen_completions.rs`, `lib/src/utilities/mod.rs`); 1 `if let` (`lib/src/manifests/providers/git.rs`); drop-order only in the files listed in spec Problem 4. A nonzero `set_var` count is a blocker.
- [ ] Root `rustfmt.toml`: `style_edition = "2021"` (one line, comment above: `# Flipped to 2024 in a separate PR: a reformat here would not survive squash-merge as its own commit`).
- [ ] `edition = "2024"` in the four `Cargo.toml`. `cargo fix --edition --allow-dirty --all-targets --workspace`, then in `fuzz/` (nightly: `PATH` from the Makefile's `NIGHTLY_BIN`).
- [ ] Drop-order (V3): for each Problem 4 site, report temporary type, behaviour change yes/no, action; a yes gets a `let` binding keeping the old order.
- [ ] Fix every new clippy lint (no `allow`).
- [ ] `make fuzz-manifest FUZZ_TIMEOUT=10` (V5); report output.
- [ ] Gates, commit.

### Task 12: Repo docs

```yaml-task
id: 12
description: Update CLAUDE.md CI table, Advisory Triage and Testing sections for cargo-deny-only triage, unsound/unmaintained, EnvGuard and edition 2024 (docs-only).
role: executor
model: sonnet
tdd: not-applicable
requirements: []
acceptance:
  - cmd: '! command grep -n "cargo-audit-scheduled" CLAUDE.md'
    exit_code: 0
  - cmd: 'grep -q "EnvGuard" CLAUDE.md'
    exit_code: 0
  - cmd: make lint
    exit_code: 0
max_retries: 2
files_touched: [CLAUDE.md]
depends_on: [11]
```

- [ ] CI table `cargo-audit` row: runs `cargo deny check advisories licenses bans sources`. Add a row for scheduled `cargo-deny-scheduled.yml` (weekly advisories).
- [ ] Rewrite "Advisory Triage": one ignore list (`deny.toml`), `unsound`/`unmaintained = "all"` and why, `tests/test_deny_policy.py` forbids a workflow `--ignore RUSTSEC`.
- [ ] Testing: tests mutate env only via `crate::test_helpers::EnvGuard`, inside `#[serial]` tests; `tests/test_env_guard_serial.py` enforces it.
- [ ] Workspace note: edition 2024, `style_edition` pinned in `rustfmt.toml`.
- [ ] The ai-config knowledge file `etch-cli-cargo-deny-vs-audit.md` is stale after this; the orchestrator raises it with the ai-config session at Phase 3 `docs` (not this task: another repo).
- [ ] Gates, commit.

### Task 13 (PR2): Style edition 2024

```yaml-task
id: 13
description: Flip rustfmt style_edition to 2024 and reformat; fmt-only PR opened after PR1 merges.
role: executor
model: sonnet
tdd: not-applicable
requirements: [R18]
acceptance:
  - cmd: 'grep -qx "style_edition = \"2024\"" rustfmt.toml'
    exit_code: 0
  - cmd: make test
    exit_code: 0
max_retries: 2
depends_on: [12]
```

Formatting only. `files_touched` is unconstrained: the task edits `rustfmt.toml`, and `cargo fmt --all` rewrites about 63 `.rs` files (measured on a scratch copy).

- [ ] `rustfmt.toml`: `style_edition = "2024"`; update the comment.
- [ ] `cargo fmt --all`. `git diff --stat` must show only `.rs` files and `rustfmt.toml`.
- [ ] Gates, commit, PR2 (`gh pr create --repo brujack/etch-cli --base main`).


## Non-goal check

Reviewer: one fresh subagent, `~/.claude/skills/writing-plans/nongoal-check.md`, 2026-10-31.

- N1-N9: CLEAR.
- N10: UNCLEAR. Quote: "Task 5 Interfaces: private `fn is_elevated(&self) -> bool` on `Exec` (... the `(true, _)` arm of `elevate_if_required`, which must then use it)." Why: the rewrite might touch the dead check at `exec.rs:157`.
  Resolution (plan revision): the check is in `execute()` at line 157, outside `elevate_if_required` (lines 61-89). Task 5 now states the boundary. Before: no statement. After: "**N10 boundary:** `elevate_if_required` (`exec.rs:61-89`) only changes its match guard to call `is_elevated()`. The dead provider comparison ... sits in `execute()` (`exec.rs:157`), after resolution; keep that line byte-for-byte."
