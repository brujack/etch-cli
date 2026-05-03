# Test Coverage Improvement — Design Spec

**Date:** 2026-05-02
**Status:** Pending implementation

## Goal

Raise test coverage from 39% to >90% by writing thorough tests for every action type, provider, and atom, following the mandatory test categories: happy path execution, error paths, and boundary conditions.

## Current state

- **Coverage:** 39.16% (1050/2681 lines)
- **CI gate:** `cargo tarpaulin --fail-under 25`
- **Biggest gaps:** 3 action types with 0% (binary, git, macos), Linux providers untested, most existing tests only cover YAML deserialization

## Approach

Thoroughness by action type — work through each action type and write complete tests before moving on. Maps to subagent-driven development (one agent per group). Produces a test suite that catches real bugs, not just a coverage number.

## Section 1: Scope and exclusions

**Excluded from coverage target:**

- `jsonschemagen/` — schema generation utility, not production logic

**Everything else is in scope**, including `app/src/main.rs` (covered automatically when `assert_cmd` runs the binary) and the four thin `app/src/commands/` files.

**Tarpaulin invocation after this work:**

```bash
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 90
```

## Section 2: Test strategy by task group

### Group 1: Zero-coverage action types — git, binary, macos

**`actions/git/clone.rs` and `atoms/git/clone.rs`**

- Deserialization: parse `repo_url` and `directory` fields from YAML
- Plan: verify `plan()` returns a `git clone` Exec step with correct args
- Execute: clone a local bare repo created in `tempfile::tempdir()` — no network, real git execution
- Error: invalid repo URL returns error or empty step list; target directory already exists

**`actions/binary/github.rs`**

- Deserialization: parse `name`, `version`, `url` fields
- Plan: verify step list includes a download step and an install step
- Execute (network, `#[ignore]`): download a known small GitHub release binary, verify it lands on disk
- Error: malformed URL, missing version field

**`actions/macos/default.rs`**

- Deserialization: all supported value types (string, bool, int, float, array)
- Plan: verify `plan()` returns a `defaults write` Exec step with correct domain, key, and value args
- No execute test — `defaults write` modifies system state; plan-level verification is sufficient

---

### Group 2: File actions — copy, chown, remove, unarchive

**`actions/file/copy.rs`** (currently 2.9%)

- Execution: copy a real file from source to destination in tempdir; verify destination exists with correct contents
- Template rendering: copy a file with `template: true`, verify Tera substitution ran
- Error: source file missing; destination directory missing
- Boundary: zero-byte file; file with no read permission

**`actions/file/chown.rs`** (0% plan coverage)

- Plan: returns a `Chown` atom step with correct `owner` and `group` populated from action fields
- Boundary: `user: null` and `group: null` both produce empty string in atom

**`actions/file/remove.rs`** (0%)

- Plan: returns correct remove step
- Execute: file is gone after execution
- Boundary: file does not exist before execution (idempotent)

**`actions/file/unarchive.rs`** (0%)

- Deserialization: parse `source` and `destination` fields
- Execute: unarchive `fixtures/test.tar.gz` into tempdir; verify contents
- Error: source archive missing; destination not writable

**`atoms/file/unarchive.rs`** (0%)

- Plan: `should_run` true when destination does not exist, false when already extracted
- Execute: actual extraction into tempdir

---

### Group 3: Directory and command actions

**`actions/directory/create.rs`** (0%)

- Plan: returns `mkdir -p` step
- Execute: directory exists after run; idempotent (run twice, no error)
- Error: path is an existing file

**`actions/directory/remove.rs`** (0%)

- Plan: returns remove step
- Execute: directory gone after run
- Boundary: directory does not exist (idempotent)

**`actions/command/run.rs`** execution (currently 22.7%, deserialization only)

- Execute happy path: run `echo hello`, verify step executes without error
- Execute error path: run `false` (non-zero exit), verify error propagates
- Env vars: injected vars are visible to the command
- Privileged: privileged command produces a step with elevation configured

---

### Group 4: Linux providers — user and group

**Mock strategy:** each test creates a `tempfile::tempdir()`, writes a bash mock script for the relevant system command (`useradd`, `groupadd`, `usermod`, `dscl`, etc.) that appends its arguments to a log file and exits 0 (or a controlled non-zero code). The test prepends the tempdir to `PATH` via `std::env::set_var("PATH", ...)`. Tests in these modules are annotated `#[serial]` (via `serial_test` crate) to prevent PATH mutation races.

**`actions/user/providers/linux.rs`** (0%)

- `add_user`: mock `useradd`; verify step list includes correct args (username, home dir, shell)
- `add_to_group`: mock `usermod`; verify `-aG` args
- Error: mock exits non-zero; verify error propagates
- Boundary: username with special characters; no home dir specified

**`actions/group/providers/linux.rs`** (0%)

- `add_group`: mock `groupadd`; verify group name arg
- Error: mock exits non-zero

---

### Group 5: Package providers — homebrew, snapcraft

**`actions/package/providers/homebrew.rs`** (0%)

- `available()`: returns false in CI (brew not installed); test this explicitly
- `bootstrap()`: returns a step list containing the brew install script command
- `install()`: plan returns `brew install <name>` step
- `has_repository()` / `add_repository()`: homebrew taps — verify step args

**`actions/package/providers/snapcraft.rs`** (36% — improve error paths)

- Error path: `install()` with empty package name
- `available()`: returns false when snap not installed

---

### Group 6: Remaining atom gaps

**`atoms/command/exec.rs`** (31.4% — improve error paths and edge cases)

- Error: command not found on PATH
- Stdin/stdout capture
- Working directory override

**`atoms/file/chmod.rs`** (42.9%)

- Plan: `should_run` false when permissions already match
- Execute: permissions change is reflected in `stat`
- Boundary: invalid permission mode string

**`atoms/file/contents.rs`** (41.7%)

- Plan: `should_run` false when file already has expected contents
- Execute: file written with correct contents
- Boundary: empty contents; binary contents

---

### Group 7: CLI commands and action dispatch

**`app/src/commands/version.rs`**

- `assert_cmd`: `etch version` outputs a version string matching the crate version

**`app/src/commands/gen_completions.rs`**

- `assert_cmd`: generate completions for bash, zsh, fish — each produces non-empty output

**`app/src/commands/contexts.rs`**

- `assert_cmd`: `etch contexts` exits 0 and produces output containing `os.` keys

**`app/src/commands/plugin.rs`**

- Unit test: plugin subcommand handler does not panic with empty plugin list

**`lib/src/actions/mod.rs`** (20.4%)

- Dispatch: each `Actions` enum variant round-trips through deserialization
- All 9 action types represented in a single manifest parse test

---

### Group 8: CI gate update

- `--fail-under 25` → `--fail-under 90` in `.github/workflows/ci.yml`
- Add `--exclude-files 'jsonschemagen/*'` to tarpaulin invocation
- Add `-- --include-ignored` to a second tarpaulin run so the binary network test counts
- Update CLAUDE.md coverage figure

## Section 3: Test infrastructure

### New files

**`lib/src/test_helpers.rs`** — `#[cfg(test)]` module with:

- `make_manifest(dir: &Path) -> Manifest` — deserializes a minimal YAML manifest string (`actions: []`) with `manifest_directory` overridden to the given path
- `make_contexts() -> Contexts` — empty contexts for plan tests

**`lib/src/fixtures/`** — committed test assets:

- `test.tar.gz` — single file (`hello.txt`) archived
- `test.zip` — same content, zip format

### New dependency

`serial_test = "3"` added to `[dev-dependencies]` in `lib/Cargo.toml` — required for serializing PATH-mutation tests in Linux provider groups.

### Binary network test convention

Tests that make real network calls are marked `#[ignore]`. The CI tarpaulin step runs two passes:

```bash
# Pass 1: offline tests only
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 90

# Pass 2: include ignored (network) tests for binary action
cargo tarpaulin --exclude-files 'jsonschemagen/*' -- --include-ignored
```

Only Pass 1 enforces the `--fail-under 90` gate. Pass 2 is informational.

## Out of scope

- Refactoring production code to improve testability (tests must work with code as-is)
- Adding tests for `docs/`, `smoke-tests/`, or `examples/`
- Increasing coverage beyond 90% on files that are already well-covered
