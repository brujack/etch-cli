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
   the lockfile and are stale. cargo-audit and cargo-deny read the same RustSec database,
   **but with today's `deny.toml` they do not report the same advisories.** The backlog row
   "Raw `cargo audit` advisories `cargo deny` skips" records cargo-audit reporting
   RUSTSEC-2026-0306 (faster-hex 0.10.0, via gix) and RUSTSEC-2026-0205 (scc 2.4.0, via
   dev-dependency `serial_test` 3.4.0). Both are `informational = "unsound"`. cargo-deny's
   `unsound` default is `"workspace"`, which drops unsound advisories in transitive crates.
   Measured on `claude` at `412dac2`: with `unsound = "all"` added to a copy of `deny.toml`,
   `cargo deny check advisories` exits 1 with exactly those two `error[unsound]` lines; with
   today's file it exits 0. `cargo update -p faster-hex` and `cargo update -p scc` lock 0
   packages, so neither clears with a lockfile bump; `serial_test` 4.0.1 exists. Population:
   the Linux lockfile on `claude`, one advisory-db snapshot. Retiring cargo-audit is safe
   only once cargo-deny reports what it reports.

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
   (`SetEnvVars`) and `std::env::remove_var` after it (`RemoveEnvVars`). Four defects
   follow:
    - **The finalizer deletes instead of restoring.** A manifest with
      `env: { PATH: "..." }` removes `PATH` from the etch process after the step, and
      every later step that resolves a binary by name fails. A variable etch inherited
      (say `FOO=orig`) is gone after a step that set `FOO` in `env:`.
    - **The finalizer does not always run, so `env:` leaks.** `apply.rs` runs initializers
      inside the plan `filter` but finalizers only after a successful `execute`. After a
      failed `execute` (`break`) the variable stays set for every later action. Under
      `--dry-run` initializers run and finalizers never do; nothing executes afterwards in
      a dry run, so that leak has no observer today.
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
  sets no `unmaintained` key, so etch-cli already has it. This spec writes it explicitly so
  the default is visible in the file and does not move with a cargo-deny release.
- **"Pin the whole `deny.toml` with a test."** In math the pin forces a reviewed edit. Here
  `auto-merge` squash-merges on green with no human review step, so an agent editing
  `deny.toml` edits the pin dict in the same diff, and the test changes no verdict. Dropped
  in Step 8 round 1; the positive control below is kept because it is the only check that
  fails if cargo-deny measures nothing.
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

### 2. Prove the policy rejects, and keep one ignore list

`deny.toml` gains `unmaintained = "all"` and `unsound = "all"` under `[advisories]`. The
second closes the gap in Problem 2 and turns the two unsound advisories into blocking
`cargo-audit` errors, so PR1 triages them, in this order per advisory:

1. Clear it by a dependency bump if one exists without an API change etch must follow
   (scc: try `serial_test` 4.x; faster-hex: a gix release that admits faster-hex ≥0.10.1).
2. Otherwise add a dated ignore to `deny.toml` in the existing comment style: path to the
   crate, why the unsound code is not reached by etch or why no fix is available, the date
   checked, and what resolves it.

The PR body records which path each took. The backlog row "Raw `cargo audit` advisories
`cargo deny` skips" is deleted in PR1.

`tests/test_deny_policy.py` (pytest, already run by the `test` job):

- **One ignore list.** The scheduled workflow (Design 4) passes no `--ignore` flags of its
  own, so there is only one list. The test asserts that no workflow file contains
  `--ignore RUSTSEC`, which prevents a second list from returning. It also asserts its glob
  matched at least one workflow file, so it cannot pass on an empty directory listing.
- **Positive control: the policy rejects what it should.** Two fixture crates under
  `tests/fixtures/deny/`: `gpl-fixture` (`license = "GPL-3.0-only"`, no dependencies) and
  `mit-fixture` (`license = "MIT"`). For each, the test copies the fixture to `tmp_path`
  (cargo-deny writes a `Cargo.lock` beside it, and the copy keeps it outside the workspace
  root so cargo does not claim it as a member), then runs
  `cargo deny --config <repo>/deny.toml --manifest-path <tmp>/Cargo.toml check licenses`.
  Expected: `gpl-fixture` exits non-zero with `error[rejected]` in its output;
  `mit-fixture` exits 0. `check licenses` only, so no network is needed. `--config` goes
  before `check`; after it, 0.20.2 exits 2. The MIT case is also the proof that `deny.toml`
  loaded: cargo-deny 0.20.2 silently falls back to its built-in config when `--config`
  names a missing or empty file, and that config's allow-list is empty, so MIT would be
  rejected. GPL is rejected either way, so the GPL case alone proves nothing about the
  config (reported by math, math#188).
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
block is later removed, the stale `expect` raises `unfulfilled_lint_expectations`, which
`make lint` (`clippy -D warnings`) turns into a failure, instead of lingering as an unused
allow-path. A plain `cargo build` only warns. Verified by the risk lens on rustc 1.99.1:
`#[expect]` is accepted on a `let` statement, an expression statement and an item.

The 2 test blocks in `config_unset.rs` move to `EnvGuard` (Design 6) and need no `expect`.

### 4. Replace the scheduled cargo-audit with scheduled cargo-deny

Delete `.github/workflows/cargo-audit-scheduled.yml`. Add
`.github/workflows/cargo-deny-scheduled.yml`: same trigger (`cron: "0 6 * * 1"`), installs
`cargo-deny@0.20.2` through the pinned install-action, runs
`cargo deny check advisories`, reading the ignore list from `deny.toml`. The weekly job
exists to catch advisories published against an unchanged lockfile, when no PR is open to
run the blocking job; that purpose is kept, and the second ignore list is removed.

The stale `RUSTSEC-2026-0118`/`-0119` ignores go with the deleted file. They are not added
to `deny.toml`. Design 4 lands after Design 2's `unsound = "all"`, so at no point does the
repo have neither cargo-audit nor unsound coverage.

### 5. Fix `command.run` env handling (test first)

`RunCommand::plan` passes `self.env` into `Exec.environment`, sorted by key so the plan is
deterministic (the source is a `HashMap`). `SetEnvVars` and `RemoveEnvVars` are deleted,
with their modules and re-exports; `command.run` is their only user (`git grep` shows no
other). The `Ensure` env initializer and finalizer go from the plan; the `SkipIf`
initializer stays.

**Program resolution moves with it.** `Exec::execute` resolves the program with
`utilities::get_binary_path` → `which()` against the etch process `PATH`
(`atoms/command/exec.rs:152`, `utilities/mod.rs:11`). Today `SetEnvVars` has already
changed that `PATH`, so `command: foo` with `env: { PATH: "/opt/foo/bin" }` finds `foo`.
Moving `env:` into `Command::envs` alone would break that. So, when the command is not
elevated and `environment` contains `PATH`, `Exec` resolves the program with
`which::which_in(program, Some(<that PATH>), std::env::current_dir()?)`; otherwise it
keeps `which()`. The `cwd` argument is the etch process cwd, not `working_dir`: `which()`
resolves a name containing a separator (`./x.sh`) against the process cwd, and `which_in`
resolves it against its `cwd` argument, so passing `working_dir` would make `./x.sh`
resolve differently depending on whether `env:` sets `PATH`. This keeps today's relative
resolution in both branches.

"Elevated" means the `(privileged: true, effective user not root)` arm of
`elevate_if_required` (`exec.rs:68-89`), the arm that prefixes the provider. Root with
`privileged: true` is not elevated and takes the `which_in` branch. When elevated, the provider (`sudo`, `doas`, `run0`) still resolves
against the etch process `PATH`, and the inner program is resolved by `env` from the
`env K=V` prefix, which already sees the new `PATH`. Both paths then resolve the program
against the step's own `PATH`. This is internal to `Exec`; no field or signature changes,
so N3 does not apply.

Behaviour after the fix:

- `env:` reaches the child process through `Command::envs`, and through `env K=V` under
  sudo, using `Exec`'s existing path.
- An unprivileged program is found on the step's own `env:` `PATH` when one is set.
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

`EnvGuard` carries
`#[must_use = "bind to a named variable (let _g = ...); let _ = drops and restores at once"]`.
Without it, a mechanical port to `let _ = EnvGuard::prepend_path(...)` drops the guard on
the same line, `PATH` stays real, and the test runs the host's real `systemctl`,
`useradd` or `gem` instead of the mock (core.md tdd-007). `#[must_use]` warns on a bare
expression statement only; it does not fire on `let _ =`, so the source check below covers
that form.

Every test site moves to the guard. Manual save-and-restore code (`original_path` and
similar) is deleted from 14 files
(`git grep -lE 'original_path|orig_path|old_path|saved_path|prev_path' -- lib/src`): the
guard restores on drop, including on panic, which today's manual restores do not. Sites
that replace `PATH` outright (e.g. `pyenv/virtualenv.rs:327`, `PATH=/nonexistent`) use
`set`.

Four closure helpers set env today (`group/providers/linux.rs:58` and
`user/providers/linux.rs:143` `with_mock_bins`, `user/providers/macos.rs:143`
`with_mock_id`, `:158` `with_failing_id`; 9 callers). They are inlined into their callers,
so every guard is built inside a test body. Two env-mutating tests lack `#[serial]` today
(`contexts/mod.rs:170`, `state/mod.rs:254`) and gain it.

`tests/test_env_guard_serial.py` scans `lib/src/**/*.rs`, matching any path to the type
(`EnvGuard`, `test_helpers::EnvGuard`, `crate::test_helpers::EnvGuard`), and fails on any
of:

- `EnvGuard` imported under another name (`use … EnvGuard as …`);
- a function whose body names `EnvGuard` and which is not `#[test]`, except the
  `impl EnvGuard` and `impl Drop for EnvGuard` blocks in `test_helpers.rs`. No shared
  helper, in `test_helpers.rs` or elsewhere, can then hide an unserialised caller;
- a `#[test]` function whose body names `EnvGuard` without a `#[serial]` attribute
  (`#[serial]`, `#[serial(key)]`, `#[serial_test::serial]`, in any order relative to
  `#[test]`);
- `let _ = <path>EnvGuard` or `drop(<path>EnvGuard` anywhere.

Detection is a regex over function bodies. The test carries one must-flag fixture string
per rule and must-not-flag fixtures for `#[serial]` before `#[test]`, `#[serial(key)]`, a
module-level `use crate::test_helpers::EnvGuard;` in `mod tests`, and a closure inside a
`#[test]` body. It cannot pass by matching nothing, and it cannot pass by matching
everything.

### 7. Edition 2024

**What it buys.** Under 2024, `set_var`/`remove_var` are `unsafe`; with
`unsafe_code = "deny"`, any new process-env mutation in production code fails to compile,
so the defect class in Problem 5 cannot return without an `#[expect]` a reviewer sees. It
also brings let-chains and `unsafe extern` blocks, and matches math (math ADR-0036).

Order matters: Designs 5 and 6 land first, so the edition bump finds no env-mutation sites.

- `edition = "2024"` in `app`, `lib`, `jsonschemagen` and `fuzz` `Cargo.toml`.
- **`style_edition` pinned to 2021 in PR1.** With no `style_edition` set, rustfmt follows
  `edition`, so the bump alone fails `cargo fmt --check` (risk lens, round 2: 119 hunks in
  63 files on a `git archive` copy). PR1 adds root `rustfmt.toml` with
  `style_edition = "2021"`, so PR1 carries no reformat.
- **Pre-check on the 2021 tree.** Immediately before the edition bump (after Designs 5 and
  6), `cargo check --workspace --all-targets` with `RUSTFLAGS=-W rust-2024-compatibility`
  reports 0 `set_var`/`remove_var` warnings, 4 `gen` keyword warnings in
  `app/src/commands/gen_completions.rs` and `lib/src/utilities/mod.rs`, 1 `if let`
  warning in `lib/src/manifests/providers/git.rs`, and drop-order warnings only in the files
  of the Problem 4 list (counts per file compared, not line numbers, since Design 5 edits
  `exec.rs`). This must run on 2021: the migration lints fire only on editions before 2024,
  so the same command after the bump reports 0 whatever happened.
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
- **Style edition is a separate PR (PR2).** `auto-merge` squash-merges, so a fmt-only
  commit inside PR1 would not survive onto `main`. PR2 changes `rustfmt.toml` to
  `style_edition = "2024"`, runs `cargo fmt --all` (63 files, +250/-207 measured), and
  nothing else. No `.git-blame-ignore-revs`: the branch guard refuses its revert
  provenance proof whenever that file is in the tree (`sdlc-branch-guard.sh:514-515`),
  which would close the `rollback-cycle` direct-revert path in etch-cli, and the blame
  noise of a 63-file reformat does not justify that.

## Delivery

- **PR1:** Designs 1-7 with `style_edition = "2021"`, one commit per Design section in
  order.
- **PR2:** the style-edition flip and reformat, opened after PR1 merges.

## Testing

- Design 1: CI's `cargo-audit` job log shows the `licenses ok` and `sources ok` lines.
- Design 2: `pytest tests/test_deny_policy.py`. Mutation controls: change `gpl-fixture`'s
  license to MIT and confirm the rejection case fails; point `--config` at an empty file and
  confirm the MIT case fails; add an `--ignore RUSTSEC-0000-0000`
  line to a scratch workflow and confirm the one-list case fails.
- Design 3: `make lint` passes. Mutation control: delete one `#[expect(unsafe_code)]` and
  confirm `make lint` fails on that site.
- Design 5, unit: `RunCommand::plan` with `env: {FOO: bar}` yields an `Exec` whose
  `environment` is `[("FOO", "bar")]` and a step with no env initializer or finalizer
  (this also removes the `--dry-run` leak, which has no runtime observer). `Debug` of the
  planned step does not contain the env value.
- Design 5, integration (`app/tests/integration.rs`, `ETCH_STATE_DIR` set; etch run with
  `ETCH_SPEC_PROBE=orig` in its own environment). Each probe step runs
  `sh -c 'printf %s "${ETCH_SPEC_PROBE-unset}" > <tmp>/probe'` and the test reads
  `<tmp>/probe`: `command.run` is non-streaming, so `Exec` captures stdout and logs it only
  at `debug!`, which the default INFO level never prints. A missing probe file fails the
  case, so an empty observation cannot pass. Both actions sit in one manifest: a failed step
  breaks only the step loop, later actions in the manifest still run, and the run exits
  non-zero, which case (b) expects.
    - **(a) Restore.** Action 1 sets `env: { ETCH_SPEC_PROBE: "x" }`; action 2 probes.
      Expected `orig`. On `main`: `unset` (deleted, not restored). RED.
    - **(b) Failed step.** Action 1 sets `env: { ETCH_SPEC_PROBE: "x" }` and runs `false`;
      action 2 probes. Expected `orig`. On `main`: `x` (finalizer skipped). RED.
    - **(c) PATH survives.** Action 1 sets `env: { PATH: "<tmp>" }`; action 2 resolves
      `sh` by name and succeeds. On `main`: "Command `sh` not found in path". RED.
    - **(d) Same-step resolution.** Action 1 sets `env: { PATH: "<tmp>:<real PATH>" }`
      with `command: etch-spec-mock`, an executable only in `<tmp>`. It succeeds. Green
      on `main` too: a regression guard. Mutation control: revert `which_in` to `which()`
      and confirm (d) fails.
- Design 6: the full suite passes under `make test` (nextest) and under `cargo tarpaulin`
  (threads). `git grep -nE 'env::(set_var|remove_var)' -- '*.rs'` returns only
  `lib/src/test_helpers.rs`.
- Design 6: `pytest tests/test_env_guard_serial.py`, with each must-flag fixture flagged,
  each must-not-flag fixture passed, and the real `lib/src` tree passing.
- Design 7: the 2021-tree pre-check in Design 7, then `make test` on the 2024 tree.
  Compilation is the gate for `set_var` (now `unsafe`, denied) and `gen`; V3 is the gate
  for drop order.

## Requirements

- **R1.** `[PR1]` The `cargo-audit` job in `ci.yml` runs `cargo deny check advisories licenses bans sources`, keeps job id `cargo-audit`, and stays in `auto-merge`'s `needs:`.
- **R2.** `[PR1]` `deny.toml` sets `unmaintained = "all"` and `unsound = "all"` under `[advisories]`, and RUSTSEC-2026-0306 and RUSTSEC-2026-0205 are each cleared by a dependency bump or carry a dated, reasoned ignore; its other content is unchanged.
- **R3.** `[PR1]` When `Exec` is not elevated (not the `privileged: true`, non-root arm of `elevate_if_required`) and `environment` contains `PATH`, it resolves the program with `which::which_in` against that `PATH` and the etch process cwd; otherwise it resolves with `which()` against the etch process `PATH`.
- **R4.** `[PR1]` `tests/test_deny_policy.py` asserts no file under `.github/workflows/` contains `--ignore RUSTSEC`, and that its glob matched at least one workflow file.
- **R5.** `[PR1]` Fixture crates `tests/fixtures/deny/gpl-fixture` (GPL-3.0-only) and `tests/fixtures/deny/mit-fixture` (MIT) exist; the test runs `cargo deny --config deny.toml --manifest-path <tmp copy> check licenses` on each and expects non-zero with `error[rejected]` for GPL and exit 0 for MIT.
- **R6.** `[PR1]` The fixture cases skip when `cargo-deny` is absent and `CI` is unset, and fail when `cargo-deny` is absent and `CI` is set.
- **R7.** `[PR1]` The `test` job in `ci.yml` installs `cargo-deny@0.20.2` through the pinned `taiki-e/install-action` step with `fallback: none`, before the step that runs pytest.
- **R8.** `[PR1]` `lib`, `app` and `jsonschemagen` `Cargo.toml` set `unsafe_code = "deny"` in `[lints.rust]`.
- **R9.** `[PR1]` Each of the 8 production `unsafe` blocks in `lib/src` carries `#[expect(unsafe_code, reason = "...")]` on its smallest enclosing item or statement, with a non-empty reason naming the FFI call.
- **R10.** `[PR1]` `.github/workflows/cargo-audit-scheduled.yml` is deleted and `.github/workflows/cargo-deny-scheduled.yml` runs `cargo deny check advisories` on `cron: "0 6 * * 1"` using `cargo-deny@0.20.2` from the pinned install-action.
- **R11.** `[PR1]` `RunCommand::plan` sets `Exec.environment` from `env:`, sorted by key, and adds no initializer or finalizer that touches the process environment.
- **R12.** `[PR1]` `SetEnvVars`, `RemoveEnvVars` and their modules and re-exports are deleted.
- **R13.** `[PR1]` Integration tests (a)-(d) from Testing exist, each probe writing to a file the test reads: with etch started under `ETCH_SPEC_PROBE=orig`, a later action sees `orig` after an earlier action set it in `env:` (a) and after that earlier action failed (b); a `PATH` set in one action's `env:` leaves the next action able to resolve `sh` (c); and an unprivileged action resolves a program present only on its own `env:` `PATH` (d).
- **R14.** `[PR1]` `lib/src/test_helpers.rs` defines `EnvGuard` with `set`, `remove` and `prepend_path`, marked `#[must_use]`, restoring the previous value on drop; its single `unsafe` block carries `#[expect(unsafe_code, reason = "...")]` and a SAFETY note naming the nextest-process and `#[serial]`-thread conditions.
- **R15.** `[PR1]` `git grep -nE 'env::(set_var|remove_var)' -- '*.rs'` matches only `lib/src/test_helpers.rs`.
- **R16.** `[PR1]` `tests/test_env_guard_serial.py` fails on an aliased `EnvGuard` import, on a non-`#[test]` function naming `EnvGuard` other than the `EnvGuard` impls in `test_helpers.rs`, on a `#[test]` function naming `EnvGuard` without a `#[serial]` attribute in any form or order, and on `let _ =` or `drop(` of an `EnvGuard`; it carries a must-flag fixture per rule and must-not-flag fixtures for `#[serial]` before `#[test]`, `#[serial(key)]`, a module-level `use`, and a closure in a test body.
- **R17.** `[PR1]` `app`, `lib`, `jsonschemagen` and `fuzz` `Cargo.toml` set `edition = "2024"`, and root `rustfmt.toml` sets `style_edition = "2021"`.
- **R18.** `[PR2]` Root `rustfmt.toml` changes `style_edition` from `"2021"` to `"2024"`, and PR2's diff is that line plus `cargo fmt --all` output with no non-formatting change.
- **R19.** `[PR1]` `docs/src/command.md` states that `env:` now reaches privileged commands, that an unprivileged program is resolved on the step's own `env:` `PATH`, and that `env:` no longer modifies the etch process environment.
- **R20.** `[PR1]` The four env-setting closure helpers (`with_mock_bins` ×2, `with_mock_id`, `with_failing_id`) are inlined into their callers, and `contexts/mod.rs:170` and `state/mod.rs:254` tests gain `#[serial]`.
- **V1.** CI on the PR: the `cargo-audit` job log contains `licenses ok` and `sources ok`, and every job in `auto-merge`'s `needs:` is green.
- **V2.** Mutation controls run and recorded in the PR body: the GPL fixture relabelled MIT fails the rejection case; a scratch `--ignore RUSTSEC` line fails the one-list case; one removed `#[expect(unsafe_code)]` fails `make lint`; `which_in` reverted to `which()` fails integration case (d).
- **V3.** The PR body classifies each of the 12 drop-order sites (temporary type, behaviour change yes/no, action taken).
- **V4.** On macOS (`ssh` to the Mac Studio; there is no macOS CI job), `cargo clippy --all-targets -- -D warnings` and `cargo nextest run` pass on the final 2024 tree, covering `#[cfg(target_os = "macos")]` sites the Linux build excluded.
- **V5.** `make fuzz-manifest FUZZ_TIMEOUT=10` builds and runs under edition 2024 on nightly.
- **V6.** Before the fix, R13 cases (a), (b) and (c) fail on `main` with `unset`, `x` and "Command `sh` not found in path" respectively, and (d) passes; output recorded in the PR body.
- **V7.** Linux CI Rust coverage stays at or above 81% and Python coverage at or above 90%.
- **V8.** The Design 7 pre-check runs on the 2021 tree immediately before the edition bump and reports 0 `set_var`/`remove_var` warnings, 4 `gen`, 1 `if let`, and drop-order warnings only in Problem 4's files; per-file counts recorded in the PR body.
- **V9.** On the Mac Studio, `mkdir -p /tmp/p && printf '#!/bin/sh\necho ok\n' > /tmp/p/etchprobe && chmod +x /tmp/p/etchprobe && /usr/bin/env PATH=/tmp/p etchprobe` prints `ok`, confirming BSD `env` resolves the program on the `PATH` it was given (the elevated branch of Design 5 depends on it).
- **V10.** Before PR1 deletes `cargo-audit-scheduled.yml`, raw `cargo audit` and `cargo deny check advisories` (with PR1's `deny.toml`) report the same set of non-ignored advisory IDs on the PR1 lockfile; both lists recorded in the PR body.
- **N1.** No `rust-version` field in any `Cargo.toml`, and no MSRV job.
- **N2.** No change to `[licenses].allow`, `[[licenses.clarify]]`, `[bans]`, `[sources]` or the 6 existing advisory ignores in `deny.toml`; the only additions are `unmaintained = "all"`, `unsound = "all"`, and at most two ignores for RUSTSEC-2026-0306 and RUSTSEC-2026-0205.
- **N3.** No change to atom or action public fields or signatures to inject `PATH` or other env (approach B).
- **N4.** No `unsafe` block added outside `EnvGuard`, and no `#[allow(unsafe_code)]` anywhere.
- **N5.** The workspace `resolver` stays `"2"`.
- **N6.** No `unsafe_code` lint or other lint change in `fuzz/`.
- **N7.** The `cargo-audit` job id in `ci.yml` is not renamed.
- **N8.** No whole-file pin test of `deny.toml`.
- **N9.** No `.git-blame-ignore-revs` file.
- **N10.** No fix to the dead `elevate()` provider check at `exec.rs:157` (backlog row).

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
Disposition: Addressed (operator, 2026-10-31) — 1: `Exec` resolves on the step's own
`env:` `PATH` (Design 5, R3, R13 (d)). 2: whole-file pin test dropped (N8); positive
control and one-list assertion kept.

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
Disposition: Addressed (operator, 2026-10-31) — 1: `#[must_use]` plus a `let _ = EnvGuard`
source check (R14, R16). 2: style edition moved to PR2 plus `.git-blame-ignore-revs` (R18,
R20). 3: compat check moved to the 2021 tree before the bump (V8), removed from V4.
4: count corrected to 14. Assumption: R16 now fails on any non-test function naming
`EnvGuard`, so a helper cannot own a guard.

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
Disposition: Addressed (operator, 2026-10-31) — 1: as Goal-Fit 1. 2: R13 rewritten to
cases (a)-(d) with etch started under `ETCH_SPEC_PROBE=orig`; failed-step leak added to
Problem 5; dry-run leak noted as unobserved and covered by the plan unit test. 3: wording
now says `make lint`.

### Adversarial Spec Review (comparison/judge designs only)

N/A — spec has no comparison/evaluator/ambiguous-criteria trigger.

## Multi-Lens Review — round 2

Reviewed at commit: `9136b92` (round-1 revisions). All three lenses re-run: the revision
changed design substance (`which_in`, delivery split).

### Goal-Fit (round 2)

Finding:
1. R13 probes print to a stream the test cannot read: `command.run` leaves
   `Exec.streaming` false, so stdout is captured and logged only at `debug!`
   (`exec.rs` ~191-205). Probes must write to a file the test reads.
2. V8 compares against file:line positions PR1 itself moves (`exec.rs:97` is rewritten by
   Design 5). Restate as counts per category plus a per-file list.
3. The spec never states what edition 2024 buys; without it the edition work fails the
   reads-it test on motive.
4. R20 is tagged `[PR3]` while Delivery says direct to `main`.
5. R4 passes vacuously if its glob matches no workflow file; assert at least one match.

Assumption: no manifest relies on `env:` leaking into a later action. Checked in
etch-config only.
Disposition: Addressed (operator, 2026-10-31) — 1: probes write to a file. 2: V8 compares
counts per category and per file. 3: Design 7 states the payoff. 4: R20/PR3 dropped.
5: R4 asserts at least one file matched.

### Ergonomics (round 2)

Finding:
1. `.git-blame-ignore-revs` disables the branch guard's revert provenance proof:
   `sdlc-branch-guard.sh:514-515` refuses with "GitHub's tree holds
   .git-blame-ignore-revs". R20 would close the `rollback-cycle` direct-revert hatch in
   etch-cli. PR2 measured at 63 files, +250/-207; the blame noise is small. Drop R20/PR3.
2. R16 rule 1 exempts all of `test_helpers.rs`, inviting a shared env helper there that
   hides unserialised callers. Four closure helpers set env today
   (`group/providers/linux.rs:58`, `user/providers/linux.rs:143`,
   `user/providers/macos.rs:143`, `:158`; 9 callers).
3. R16 has only must-flag fixtures; no must-not-flag fixture covers `#[serial]` before
   `#[test]`, a module-level `use … EnvGuard`, or a closure inside a test body. Two existing
   env-mutating tests lack `#[serial]` (`contexts/mod.rs:170`, `state/mod.rs:254`).

Assumption: confirmed by the orchestrator at `sdlc-branch-guard.sh:514-515`.
Disposition: Addressed (operator, 2026-10-31) — 1: `.git-blame-ignore-revs` dropped (N9).
2: exemption narrowed to the `EnvGuard` impls; closure helpers inlined (R20). 3:
must-not-flag fixtures added; the two unserialised tests gain `#[serial]`.

### Risk (round 2)

Finding:
1. Blocking: with no `style_edition` set, rustfmt follows `edition`, so PR1's edition bump
   fails `cargo fmt --check` (119 hunks in 63 files on a `git archive` copy). PR1 must pin
   `style_edition = "2021"` in `rustfmt.toml`; PR2 flips it to `"2024"`.
2. Same as Goal-Fit 1 (probe output is debug-only).
3. Relative program names resolve differently by branch: `which_in` resolves a name with a
   separator against its `cwd` argument, `which()` against the etch process cwd, and
   `RunCommand` always sets `working_dir`. Use one cwd in both branches.
4. "Elevated" must be defined as the `(privileged, non-root)` arm of `elevate_if_required`
   (`exec.rs:68-89`). Separately, `exec.rs:157` compares the resolved absolute path to
   `"sudo"|"doas"|"run0"`, never matches, so `elevate()` is dead code: a pre-existing bug.
5. R16 false negatives: an alias (`use … EnvGuard as G`), `let _ = test_helpers::EnvGuard`,
   `drop(EnvGuard::…)`; false positive on `#[serial(key)]`.

Assumption: on macOS, `/usr/bin/env PATH=/x prog` resolves `prog` on `/x`. Settle on the
Mac Studio: `mkdir -p /tmp/p && printf '#!/bin/sh\necho ok\n' > /tmp/p/etchprobe && chmod +x /tmp/p/etchprobe && /usr/bin/env PATH=/tmp/p etchprobe`.
Disposition: Addressed (operator, 2026-10-31) — 1: PR1 pins `style_edition = "2021"`, PR2
flips it. 2: as Goal-Fit 1. 3: `which_in` takes the etch process cwd. 4: "elevated"
defined in Design 5 and R3; dead `elevate()` check written as a backlog row (N10).
5: R16 covers aliases, qualified paths, `drop(` and `#[serial(key)]`. Assumption: V9.
