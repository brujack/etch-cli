# Rust standards: supply-chain policy, `unsafe_code`, edition 2024

**Status:** design approved in conversation 2026-10-31 (one combined spec, env-guard
approach A, `command.run` env fix folded in); spec under review.

**Origin:** the math session passed on Rust learnings from math#187/#188 (spec
`math docs/superpowers/specs/2026-10-31-rust-python-standards-tier1-design.md`) at the
operator's request. Each item was re-measured against etch-cli before design; three of the
math findings do not transfer as stated, and one production bug surfaced in the process.

## Problem

Four independent gaps, all measured on `origin/main` at `3496cf3` on `claude`
(cargo-deny 0.20.2, stable toolchain):

1. **The license, ban and source policy in `deny.toml` is not enforced anywhere.** The only
   cargo-deny invocation in the repo is `cargo deny check advisories` (`ci.yml`, job
   `cargo-audit`). No Makefile target, hook or workflow runs `check licenses`,
   `check bans` or `check sources`. The `[licenses].allow` list, the `ring`
   `[[licenses.clarify]]` and `[sources]` are policy that nothing reads. A GPL crate
   arriving through a Renovate bump would ship in the `etch` binary unflagged. Population:
   every workflow file and the Makefile, searched with
   `grep -rn 'cargo deny' .github/ Makefile scripts/`.
   Running the missing checks today passes: `bans ok, licenses ok, sources ok`, with
   `warning[duplicate]` lines only (`multiple-versions = "warn"`).

2. **Two advisory ignore lists exist and have already drifted.** `deny.toml` ignores 6
   advisories; `.github/workflows/cargo-audit-scheduled.yml` ignores 8. The two extra,
   `RUSTSEC-2026-0118` and `RUSTSEC-2026-0119`, are absent from `deny.toml` while the
   blocking `cargo deny check advisories` stays green, so they no longer match any crate in
   the lockfile and are stale. cargo-audit and cargo-deny read the same RustSec database;
   the second tool adds a second list to keep in sync and nothing else.

3. **No guard against new `unsafe`.** No crate sets `unsafe_code`. `lib/` holds 8 `unsafe`
   blocks of legitimate FFI in production code (`libc::stat`, `libc::chflags`,
   `Lua::unsafe_new`, `gix::interrupt::init_handler`) plus 2 in a test
   (`atoms/git/config_unset.rs`, around `set_var`/`remove_var`). `app/`,
   `jsonschemagen/` and `fuzz/` hold none.

4. **Edition 2021, and the move to 2024 is blocked by process-global env mutation, not by
   syntax.** `cargo check --workspace --all-targets` with
   `RUSTFLAGS=-W rust-2024-compatibility` on the 2021 tree reports:

    | Warning                                                | Count | Sites                                                                         |
    | ------------------------------------------------------ | ----: | ----------------------------------------------------------------------------- |
    | `set_var` is unsafe and requires unsafe block          |   154 | 23 files under `lib/src/`                                                     |
    | `remove_var` is unsafe and requires unsafe block       |     5 | same population                                                               |
    | relative drop order changing in Rust 2024              |    13 | 12 sites (list below)                                                         |
    | `gen` is a keyword in the 2024 edition                 |     4 | `app/src/commands/gen_completions.rs:19-20`, `lib/src/utilities/mod.rs:43-44` |
    | `if let` assigns a shorter lifetime since Edition 2024 |     1 | `lib/src/manifests/providers/git.rs:26`                                       |

    Population: the workspace members (`app`, `lib`, `jsonschemagen`), all targets, Linux.
    `fuzz/` is excluded from the workspace and was not checked; it is checked separately in
    V5. macOS-gated code (`#[cfg(target_os = "macos")]`) was not compiled, so macOS-only
    sites are not in these counts; V4 covers them.

    Drop-order sites: `app/src/commands/apply.rs:65`, `app/src/commands/rollback.rs:129`,
    `app/src/commands/status.rs:55`, `lib/src/actions/file/copy.rs:77`,
    `lib/src/actions/mod.rs:144`, `lib/src/atoms/command/exec.rs:97`,
    `lib/src/atoms/file/chmod.rs:51`, `lib/src/atoms/file/chown.rs:58`,
    `lib/src/atoms/plugin/globals.rs:54`, `lib/src/contexts/mod.rs:69`,
    `lib/src/contexts/mod.rs:70`, `lib/src/manifests/load.rs:111`.

    Of the 159 env-mutation sites, 157 are test code (115 are `set_var("PATH", …)`
    injecting mock binaries) and 2 are production: `steps/initializers/env_vars_set.rs` and
    `steps/finalizers/env_vars_remove.rs`. Population for the test/production split: every
    `env::set_var`/`env::remove_var` in `lib/src`, classified by whether it sits above or
    below the file's first `#[cfg(test)]`.

5. **The two production sites are a bug, independent of the edition.** `command.run` applies
   its `env:` map by calling `std::env::set_var` on the etch process before the step
   (`SetEnvVars`) and `std::env::remove_var` after it (`RemoveEnvVars`). Three defects
   follow:
    - **The finalizer deletes instead of restoring.** A manifest with
      `env: { PATH: "..." }` removes `PATH` from the etch process after the step, and
      every later step that resolves a binary by name fails.
    - **Privileged commands never receive `env:`.** `sudo` strips the process environment.
      The `Exec` atom already handles this for its own `environment` field by prefixing
      `env K=V` under sudo (`atoms/command/exec.rs:75-82`), but `command.run` never fills
      that field.
    - **The values leak into `Debug` output.** `SetEnvVars` and `RemoveEnvVars` derive
      `Debug` over the raw `HashMap`, which puts every `env:` value into any `{:?}` of the
      step. `Exec` redacts `environment` by hand for this reason (CLAUDE.md, "Two types must
      keep a hand-written `Debug`").

## Math findings that do not transfer

- **"Edition 2024 is near-free."** True in math (0 source lines from `cargo fix`). Not here:
  159 env-mutation sites.
- **"Set `unmaintained = "all"`."** It is already the cargo-deny default, and `deny.toml`
  sets no `unmaintained` key, so etch-cli already has it. This spec writes it explicitly only
  so the pin test (Design 2) catches a later change to `"workspace"`.
- **"Every CI job that runs bats needs cargo-deny installed."** etch-cli has no bats suite.
  The analogous trap is the pytest job, handled in Design 2.

## Design

### 1. Enforce the whole `deny.toml` in CI

The existing `cargo-audit` job in `ci.yml` runs
`cargo deny check advisories licenses bans sources` instead of `check advisories`. The job
keeps its id `cargo-audit` and its place in `auto-merge`'s `needs:`. Renaming it would
change the required-check name, and the job id is a label, not a claim about the tool;
CLAUDE.md's CI table already says the job runs cargo-deny. Only the step name and comment
change.

`bans` stays `multiple-versions = "warn"`: duplicate versions are warnings, not failures.

### 2. Pin `deny.toml` with a test, and prove the policy rejects

`tests/test_deny_policy.py` (pytest, already run by the `test` job):

- **Pin test.** Parse `deny.toml` with `tomllib` and compare the whole document to one
  expected dict literal. Any change to an allow list, ignore, exception, clarify, source
  or ban becomes a deliberate edit to the test, reviewed in the same diff. `deny.toml` gains
  `unmaintained = "all"` under `[advisories]`.
- **Ignore lists agree.** The scheduled workflow (Design 4) passes no `--ignore` flags of its
  own, so there is only one list. The test asserts that no workflow file contains
  `--ignore RUSTSEC`, which prevents a second list from returning.
- **Positive control: the policy rejects what it should.** Two fixture crates under
  `tests/fixtures/deny/`: `gpl-fixture` (`license = "GPL-3.0-only"`, no dependencies) and
  `mit-fixture` (`license = "MIT"`). For each, the test copies the fixture to `tmp_path`
  (cargo-deny writes a `Cargo.lock` beside it, and the copy keeps it outside the workspace
  root so cargo does not claim it as a member), then runs
  `cargo deny --config <repo>/deny.toml --manifest-path <tmp>/Cargo.toml check licenses`.
  Expected: `gpl-fixture` exits non-zero with `error[rejected]` in its output;
  `mit-fixture` exits 0. `check licenses` only, so no network is needed. `--config` goes
  before `check`; after it, 0.20.2 exits 2.
- **Missing tool.** If `cargo-deny` is not on `PATH`, the two fixture cases skip locally
  and **fail** when `CI` is set. The `test` job installs cargo-deny through the same pinned
  `taiki-e/install-action` step the `cargo-audit` job uses (`tool: cargo-deny@0.20.2`,
  `fallback: none`), which `tests/test_tool_installs.py` already requires to carry one
  version across every workflow.

### 3. `unsafe_code = "deny"`

Each crate's `[lints.rust]` table (`lib`, `app`, `jsonschemagen`) gains
`unsafe_code = "deny"`. `fuzz/` is left alone: it is outside the workspace and the
`libfuzzer-sys` `fuzz_target!` macro is not ours to annotate.

`deny`, not `forbid`, because the 8 production FFI blocks stay. Each gets
`#[expect(unsafe_code, reason = "...")]` on the smallest enclosing item or statement, with
a reason naming the FFI call and why it is sound. `expect`, not `allow`: if the `unsafe`
block is later removed, the stale `expect` fails the build instead of lingering as an
unused allow-path.

The 2 test blocks in `config_unset.rs` move to `EnvGuard` (Design 6) and need no `expect`.

### 4. Replace the scheduled cargo-audit with scheduled cargo-deny

Delete `.github/workflows/cargo-audit-scheduled.yml`. Add
`.github/workflows/cargo-deny-scheduled.yml`: same trigger (`cron: "0 6 * * 1"`), installs
`cargo-deny@0.20.2` through the pinned install-action, runs
`cargo deny check advisories`, reading the ignore list from `deny.toml`. The weekly job
exists to catch advisories published against an unchanged lockfile, when no PR is open to
run the blocking job; that purpose is kept, and the second ignore list is removed.

The stale `RUSTSEC-2026-0118`/`-0119` ignores go with the deleted file. They are not added
to `deny.toml`.

### 5. Fix `command.run` env handling (test first)

`RunCommand::plan` passes `self.env` into `Exec.environment`, sorted by key so the plan is
deterministic (the source is a `HashMap`). `SetEnvVars` and `RemoveEnvVars` are deleted,
with their modules and re-exports; `command.run` is their only user (`git grep` shows no
other). The `Ensure` env initializer and finalizer go from the plan; the `SkipIf`
initializer stays.

Behaviour after the fix:

- `env:` reaches the child process through `Command::envs`, and through `env K=V` under
  sudo, using `Exec`'s existing path.
- The etch process environment is never modified, so no later step sees a removed or
  leaked variable.
- `env:` values are redacted in `Debug`, through `Exec`'s hand-written impl.

This is a user-visible change for privileged steps (they now receive `env:`), noted in
`docs/src/command.md` and the PR body.

### 6. `EnvGuard`: one `unsafe` for all test env mutation

`lib/src/test_helpers.rs` (already `#[cfg(test)]`) gains:

```rust
/// Sets or removes one environment variable and restores its previous value on drop.
///
/// Every test that holds one must be `#[serial]`; see the SAFETY note on `apply`.
pub struct EnvGuard { key: OsString, previous: Option<OsString> }

impl EnvGuard {
    pub fn set(key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self;
    pub fn remove(key: impl AsRef<OsStr>) -> Self;
    /// Prepends `dir` to `PATH`: the mock-binary pattern used by 115 tests.
    pub fn prepend_path(dir: &Path) -> Self;
}
impl Drop for EnvGuard { /* restores `previous`, or removes the key if it was unset */ }
```

One private `apply(key, Option<value>)` holds the module's only `unsafe` block, under
`#[expect(unsafe_code, reason = "...")]`. Its SAFETY note states the actual guarantee:
`set_var` is sound when no other thread reads or writes the environment concurrently. Under
`cargo nextest` each test is its own process, which holds. Under `cargo test` and tarpaulin,
tests are threads in one process, and only `#[serial]` serialises them against each other.
That is the same exposure as today's bare `set_var`, made explicit, not a new one.

Every test site moves to the guard. Manual save-and-restore code (`original_path` and
similar, 5 files) is deleted: the guard restores on drop, including on panic, which today's
manual restores do not.

A separate test, `tests/test_env_guard_serial.py`, asserts that every Rust test function under `lib/src`
whose body names `EnvGuard` carries `#[serial]` or `#[serial_test::serial]`. Detection is a
regex over function bodies; the test includes a fixture string without `#[serial]` that
must be flagged, so it cannot pass by matching nothing.

### 7. Edition 2024

Order matters: Designs 5 and 6 land first, so the edition bump finds no env-mutation sites.

- `edition = "2024"` in `app`, `lib`, `jsonschemagen` and `fuzz` `Cargo.toml`.
- `rustfmt.toml` at the repo root with `style_edition = "2024"`, so editors and
  `cargo fmt` agree. Then `cargo fmt --all`; the resulting churn (import order, one-line
  `if`/`else`) is its own commit with no other change.
- `cargo fix --edition --all-targets` handles the `gen` keyword sites (raw identifier) and
  the `if let` rescope at `providers/git.rs:26`.
- **Drop-order sites are read, not auto-fixed.** For each of the 12 sites listed in Problem
  4, the PR body records what the tail expression's temporary is and whether dropping it
  earlier changes behaviour (a lock guard or a `RefCell` borrow can; a `String` cannot).
  A site where it does gets a `let` binding that keeps the old order.
- New clippy lints that edition 2024 enables (math hit `collapsible_if` through let-chains)
  are fixed, not allowed.
- The workspace `resolver` stays `"2"`. Resolver 3's only difference is MSRV-aware
  fallback, which does nothing without `rust-version`, and this spec sets no
  `rust-version`.

## Delivery

One branch, one PR, one commit per Design section in the order 1, 2, 3, 4, 5, 6, 7
(7 split into the edition/fix commit and the separate fmt-only commit). The fmt commit is
last so every earlier commit reviews as a small diff.

## Testing

- Design 1: CI's `cargo-audit` job log shows the `licenses ok` and `sources ok` lines.
- Design 2: `pytest tests/test_deny_policy.py`. Mutation control: change one `allow`
  entry in a scratch copy and confirm the pin test fails; change `gpl-fixture`'s license to
  MIT and confirm the rejection case fails.
- Design 3: `make lint` passes. Mutation control: delete one `#[expect(unsafe_code)]` and
  confirm `make lint` fails on that site.
- Design 5: RED first. Unit test: `RunCommand::plan` with `env: {FOO: bar}` yields an
  `Exec` whose `environment` is `[("FOO", "bar")]` and a step with no env initializer or
  finalizer. Integration test (`app/tests/integration.rs`, `ETCH_STATE_DIR` set): a manifest
  with step 1 `command.run` with `env: { ETCH_SPEC_PROBE: "x" }` and step 2 running
  `sh -c 'printf %s "${ETCH_SPEC_PROBE-unset}"'` prints `unset`. A second integration case
  sets `PATH` in step 1's `env:` and checks step 2 still resolves `sh` by name. Both fail on
  current `main` for the stated reason before the fix. `Debug` of the planned step does not
  contain the env value.
- Design 6: the full suite passes under `make test` (nextest) and under `cargo tarpaulin`
  (threads). `git grep -nE 'env::(set_var|remove_var)' -- '*.rs'` returns only
  `lib/src/test_helpers.rs`.
- Design 7: `cargo check --workspace --all-targets` with
  `RUSTFLAGS=-W rust-2024-compatibility` on the final tree reports 0 warnings of the five
  kinds in Problem 4.

## Requirements

- **R1.** `[PR1]` The `cargo-audit` job in `ci.yml` runs `cargo deny check advisories licenses bans sources`, keeps job id `cargo-audit`, and stays in `auto-merge`'s `needs:`.
- **R2.** `[PR1]` `deny.toml` sets `unmaintained = "all"` under `[advisories]`; its other content is unchanged.
- **R3.** `[PR1]` `tests/test_deny_policy.py` compares the parsed `deny.toml` to one expected dict and fails on any difference.
- **R4.** `[PR1]` `tests/test_deny_policy.py` asserts no file under `.github/workflows/` contains `--ignore RUSTSEC`.
- **R5.** `[PR1]` Fixture crates `tests/fixtures/deny/gpl-fixture` (GPL-3.0-only) and `tests/fixtures/deny/mit-fixture` (MIT) exist; the test runs `cargo deny --config deny.toml --manifest-path <tmp copy> check licenses` on each and expects non-zero with `error[rejected]` for GPL and exit 0 for MIT.
- **R6.** `[PR1]` The fixture cases skip when `cargo-deny` is absent and `CI` is unset, and fail when `cargo-deny` is absent and `CI` is set.
- **R7.** `[PR1]` The `test` job in `ci.yml` installs `cargo-deny@0.20.2` through the pinned `taiki-e/install-action` step with `fallback: none`, before the step that runs pytest.
- **R8.** `[PR1]` `lib`, `app` and `jsonschemagen` `Cargo.toml` set `unsafe_code = "deny"` in `[lints.rust]`.
- **R9.** `[PR1]` Each of the 8 production `unsafe` blocks in `lib/src` carries `#[expect(unsafe_code, reason = "...")]` on its smallest enclosing item or statement, with a non-empty reason naming the FFI call.
- **R10.** `[PR1]` `.github/workflows/cargo-audit-scheduled.yml` is deleted and `.github/workflows/cargo-deny-scheduled.yml` runs `cargo deny check advisories` on `cron: "0 6 * * 1"` using `cargo-deny@0.20.2` from the pinned install-action.
- **R11.** `[PR1]` `RunCommand::plan` sets `Exec.environment` from `env:`, sorted by key, and adds no initializer or finalizer that touches the process environment.
- **R12.** `[PR1]` `SetEnvVars`, `RemoveEnvVars` and their modules and re-exports are deleted.
- **R13.** `[PR1]` An integration test proves a `command.run` `env:` variable is not visible to the next step, and another proves a `PATH` set in one step's `env:` leaves the next step able to resolve `sh` by name.
- **R14.** `[PR1]` `lib/src/test_helpers.rs` defines `EnvGuard` with `set`, `remove` and `prepend_path`, restoring the previous value on drop; its single `unsafe` block carries `#[expect(unsafe_code, reason = "...")]` and a SAFETY note naming the nextest-process and `#[serial]`-thread conditions.
- **R15.** `[PR1]` `git grep -nE 'env::(set_var|remove_var)' -- '*.rs'` matches only `lib/src/test_helpers.rs`.
- **R16.** `[PR1]` `tests/test_env_guard_serial.py` fails for any `lib/src` test function using `EnvGuard` without `#[serial]`, and includes a negative fixture that it must flag.
- **R17.** `[PR1]` `app`, `lib`, `jsonschemagen` and `fuzz` `Cargo.toml` set `edition = "2024"`; root `rustfmt.toml` sets `style_edition = "2024"`.
- **R18.** `[PR1]` The fmt-only churn from `style_edition = "2024"` is a single commit with no non-formatting change.
- **R19.** `[PR1]` `docs/src/command.md` states that `env:` now reaches privileged commands and no longer modifies the etch process environment.
- **V1.** CI on the PR: the `cargo-audit` job log contains `licenses ok` and `sources ok`, and every job in `auto-merge`'s `needs:` is green.
- **V2.** Mutation controls run and recorded in the PR body: one `deny.toml` allow edit fails the pin test; the GPL fixture relabelled MIT fails the rejection case; one removed `#[expect(unsafe_code)]` fails `make lint`.
- **V3.** The PR body classifies each of the 12 drop-order sites (temporary type, behaviour change yes/no, action taken).
- **V4.** On macOS (`ssh` to the Mac Studio; there is no macOS CI job), `cargo check --workspace --all-targets` with `RUSTFLAGS=-W rust-2024-compatibility` and `cargo clippy --all-targets -D warnings` pass on the final tree, covering `#[cfg(target_os = "macos")]` sites the Linux count excluded.
- **V5.** `make fuzz-manifest FUZZ_TIMEOUT=10` builds and runs under edition 2024 on nightly.
- **V6.** Before the fix, the two R13 integration tests fail on `main` with the leak and the missing-`PATH` failure respectively; output recorded in the PR body.
- **V7.** Linux CI Rust coverage stays at or above 81% and Python coverage at or above 90%.
- **N1.** No `rust-version` field in any `Cargo.toml`, and no MSRV job.
- **N2.** No change to `[licenses].allow`, `[[licenses.clarify]]`, `[bans]`, `[sources]` or the 6 advisory ignores in `deny.toml` beyond adding `unmaintained = "all"`.
- **N3.** No change to atom or action APIs to inject `PATH` or other env (approach B).
- **N4.** No `unsafe` block added outside `EnvGuard`, and no `#[allow(unsafe_code)]` anywhere.
- **N5.** The workspace `resolver` stays `"2"`.
- **N6.** No `unsafe_code` lint or other lint change in `fuzz/`.
- **N7.** The `cargo-audit` job id in `ci.yml` is not renamed.

## Multi-Lens Review

Reviewed at commit: `9fb5d1f` (Step 7 self-review commit, before Step 8 dispatch)

### Goal-Fit

Finding:
1. Design 5 breaks same-step resolution through `env: { PATH }`. `Exec::execute` resolves
   the program with `utilities::get_binary_path` → `which()` against the etch process
   `PATH` (`atoms/command/exec.rs:152`, `utilities/mod.rs:11`). Today `SetEnvVars` changes
   that `PATH` before the lookup; after the fix the lookup uses the old `PATH` and fails
   with "Command `foo` not found in path". No R13 case covers step 1 resolving through its
   own `env:`. Proposed fix: resolve with `which::which_in` against `environment`'s `PATH`
   when set. No manifest in etch-config uses `env:` on `command.run`; latent, not live.
2. The Design 2 pin test fails the reads-it test: its only reader is the diff, which
   already shows the `deny.toml` change. Keep the GPL/MIT positive control and the
   no-`--ignore RUSTSEC` assertion; drop R3 or narrow it to keys whose cargo-deny default
   would silently change (`unmaintained`, `multiple-versions`).

Assumption: no consumer depends on `env:` changing etch's own process (same-step
resolution, or a later step relying on a leaked variable). etch-config checked, none
found; unversioned local manifests unchecked.
Disposition:

### Ergonomics

Finding:
1. `EnvGuard` without `#[must_use]`: a mechanical port to `let _ = EnvGuard::…` drops the
   guard on the same line, `PATH` stays real, and the test runs real `systemctl`,
   `useradd` or `gem` (breaks tdd-007 inert failure). Add `#[must_use]` and have R16 flag
   `let _ = EnvGuard`.
2. Squash auto-merge destroys the separate fmt commit R18 and Delivery promise; every
   reformatted line blames to the combined commit and `.git-blame-ignore-revs` has no
   fmt-only SHA to name. Ship the `style_edition` fmt as its own PR, or drop the claim.
3. The Design 7 check cannot fail on the final tree: the `rust-2024-compatibility`
   migration lints fire only on editions before 2024, so the check reports 0 after the
   bump whatever happened. Same for V4's compat-lint half. Restate as "on the 2021 tree
   immediately before the bump", or delete.
4. Premise miscount: manual save-and-restore code sits in 14 files, not 5
   (`git grep -lE 'original_path|orig_path|old_path|saved_path|prev_path' -- lib/src`).

Assumption: R16's body-scan assumes guards are created only inside `#[test]` bodies. A
shared helper returning an `EnvGuard` would hide its callers from the check. Settle by
reading how the `write_mock_*` helpers are called in the 14 files.
Disposition:

### Risk

Finding:
1. Same as Goal-Fit 1, independently found: unprivileged `env: { PATH }` resolution
   breaks, while privileged still works (`env PATH=… cmd` under sudo resolves on the new
   `PATH`), so the two paths become inconsistent. Fix internal to `Exec` (`which_in`), not
   an API change, so N3 does not forbid it.
2. The first R13 test passes on `main`: `apply.rs` runs finalizers after a successful
   `execute`, so `RemoveEnvVars` already removes `ETCH_SPEC_PROBE` and step 2 prints
   `unset`. The leak appears only under `--dry-run` (initializers run in the plan
   `filter`, finalizers never run) or after a failed `execute` (`break` skips finalizers).
   Fix: pre-set `ETCH_SPEC_PROBE=orig` and assert step 2 prints `orig` (main prints
   `unset`: the delete-instead-of-restore defect); add a `--dry-run` case. The dry-run
   leak is a further defect Problem 5 omits.
3. A stale `#[expect(unsafe_code)]` is a warning, not an error; it fails `make lint`
   (`-D warnings`) but not `cargo build`. Wording should say lint.

Assumption: same as Goal-Fit — no field manifest relies on `env:` affecting how etch
resolves the command. Moot if `Exec` resolves against `environment`'s `PATH`.
Disposition:

### Adversarial Spec Review (comparison/judge designs only)

N/A — spec has no comparison/evaluator/ambiguous-criteria trigger.
