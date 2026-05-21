# Integration Tests — Design Spec

**Date:** 2026-05-20
**Status:** Accepted

---

## Context

etch-cli has 431 unit tests covering atoms and actions in isolation. The core user
story — write a YAML manifest, run `etch apply`, verify filesystem changed correctly —
has no test coverage. The gap means regressions in manifest parsing, action dispatch,
or end-to-end application can only be caught manually.

`assert_cmd` is already in the dev dependencies.

---

## Decision

Add `tests/integration.rs` at the etch-cli repo root with 8 integration tests covering
4 action types × 2 tests each (happy path + idempotency). Tests spawn the real `etch`
binary using `assert_cmd` + `env!("CARGO_BIN_EXE_etch")` with a fake HOME in a
`tempfile::tempdir()`.

Integration tests do not contribute to tarpaulin line coverage (subprocess invocation)
and the 70% gate is unaffected.

---

## Test Structure

```
tests/integration.rs
├── helper: apply(manifest_dir, home) -> assert_cmd::assert::Assert
├── file_link_creates_symlink
├── file_link_is_idempotent
├── file_copy_copies_content
├── file_copy_is_idempotent
├── command_run_creates_file
├── command_run_is_idempotent
├── directory_create_makes_dir
└── directory_create_is_idempotent
```

### Helper

```rust
fn apply(manifest_dir: &std::path::Path, home: &std::path::Path) -> assert_cmd::assert::Assert {
    assert_cmd::Command::cargo_bin("etch").unwrap()
        .env("HOME", home)
        .arg("apply")
        .arg("-d")
        .arg(manifest_dir)
        .assert()
}
```

---

## Implementation Note: files/ Subdirectory Requirement

`FileAction::resolve()` in etch-lib joins `manifest_root + "files/" + source`. This means
source files for `file.link` and `file.copy` must be placed in a `files/` subdirectory
relative to the manifest directory, not directly alongside the manifest YAML.

Targets and output paths use absolute paths (via `dir.path().join(...)`) to avoid working
directory ambiguity at execution time.

---

## Action Coverage

### file.link (YAML fields: `source`, `target`)

```yaml
actions:
    - action: file.link
      source: "{home}/dotfile.txt"
      target: "{home}/linked.txt"
```

- **file_link_creates_symlink** — write `dotfile.txt` to fake home with known content,
  apply, assert `linked.txt` is a symlink (`path.is_symlink()`), assert
  `std::fs::read_link()` resolves to `dotfile.txt`, assert content matches.
- **file_link_is_idempotent** — apply twice, assert `.success()` both times,
  symlink target unchanged.

### file.copy (YAML fields: `from`, `to`)

```yaml
actions:
    - action: file.copy
      from: "{home}/source.txt"
      to: "{home}/dest.txt"
```

- **file_copy_copies_content** — write `source.txt` with known content, apply, assert
  `dest.txt` exists and `read_to_string()` matches original content.
- **file_copy_is_idempotent** — apply twice, assert `.success()` both times,
  `dest.txt` content unchanged.

### command.run (YAML fields: `command`, `args`)

```yaml
actions:
    - action: command.run
      command: touch
      args:
          - "{home}/created.txt"
```

- **command_run_creates_file** — apply, assert `created.txt` exists.
- **command_run_is_idempotent** — apply twice, assert `.success()` both times
  (`touch` on an existing file is a no-op).

### directory.create (YAML field: `path`)

```yaml
actions:
    - action: directory.create
      path: "{home}/myconfig"
```

- **directory_create_makes_dir** — apply, assert `myconfig` exists and
  `path.is_dir()` is true.
- **directory_create_is_idempotent** — apply twice, assert `.success()` both times,
  `myconfig` is still a directory.

---

## Manifest Writing Pattern

Each test writes the manifest YAML to a `NamedTempFile` or directly into the manifest
tempdir. Paths inside manifests use the absolute tempdir paths (no `~` expansion needed
since HOME is overridden to the tempdir):

```rust
let home = tempfile::tempdir().unwrap();
let manifest_dir = tempfile::tempdir().unwrap();
let source = home.path().join("dotfile.txt");
std::fs::write(&source, "hello").unwrap();
let manifest = format!(
    "actions:\n  - action: file.link\n    source: {}\n    target: {}\n",
    source.display(),
    home.path().join("linked.txt").display(),
);
std::fs::write(manifest_dir.path().join("test.yaml"), manifest).unwrap();
apply(manifest_dir.path(), home.path()).success();
```

---

## CI Impact

- `make test` → `cargo nextest run` — integration tests run automatically (nextest
  discovers all `tests/*.rs` files)
- Build time: ~10s overhead (binary spawn ×8, filesystem ops)
- Coverage gate: unchanged at 70% (subprocess invocations not instrumented)
- Platform: all 8 tests run on Linux CI (`ubuntu-latest`)

---

## Consequences

- First regression catch for the full `etch apply` path
- Idempotency guarantee is now machine-verified for the 4 core action types
- Pattern established for future action types (git.clone, package.install) once
  network/platform gating is worked out
