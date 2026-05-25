# Snapshot Testing (`insta`) Design

## Goal

Lock the user-visible CLI output format for `etch`. Silent regressions in help text, version strings, and dry-run messages surface as visible diffs rather than passing tests.

## Background

All existing output assertions use `predicates::str::contains()` — loose substring matches. A renamed flag, reworded banner, or reformatted dry-run summary passes silently. `insta` snapshot tests lock the exact output; any change requires an explicit `cargo insta review` + committed `.snap` update.

## Scope

Five snapshot tests covering three output categories:

| Category  | Tests                                             |
| --------- | ------------------------------------------------- |
| Help text | `etch -h`, `etch apply --help`                    |
| Version   | `etch version` (with `[VERSION]` redaction)       |
| Dry-run   | `etch apply --dry-run`, `etch apply --dry-run -v` |

**Out of scope:**

- `etch contexts` — machine-specific output (username, hostname, OS)
- `etch gen-completions bash/zsh` — 200+ line shell script, too volatile
- Integration test filesystem state — already covered by `integration.rs`

## Implementation

### Dependency

Add to `app/Cargo.toml`:

```toml
[dev-dependencies]
insta = "1"
```

### New file: `app/tests/snapshots.rs`

```rust
use assert_cmd::Command;
use tempfile::TempDir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

#[test]
fn help() {
    let output = etch().arg("-h").output().unwrap();
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
}

#[test]
fn apply_help() {
    let output = etch().args(["apply", "--help"]).output().unwrap();
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
}

#[test]
fn version() {
    let output = etch().arg("version").output().unwrap();
    insta::with_settings!({
        filters => vec![(r"\d+\.\d+\.\d+", "[VERSION]")]
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn dry_run() {
    let dir = TempDir::new().unwrap();
    // ... setup manifest, run etch --no-color -d . apply --dry-run
    // scrub tmpdir path from output
}

#[test]
fn dry_run_verbose() {
    // same setup + -v flag
}
```

### Filters

| Pattern            | Replacement | Used in                      |
| ------------------ | ----------- | ---------------------------- |
| `\d+\.\d+\.\d+`    | `[VERSION]` | `version` test               |
| actual tmpdir path | `[TMPDIR]`  | `dry_run`, `dry_run_verbose` |

Tmpdir path is dynamic — scrub via `insta::with_settings! { filters => vec![(tmpdir_str, "[TMPDIR]")] }` built from the `TempDir` path at test time.

### Snapshot files

Stored at `app/tests/snapshots/snapshots__<test_name>.snap`. Committed to the repo — they are the locked baselines.

### First-run workflow

After writing tests:

```bash
cargo insta test --review   # generates .snap files and opens review UI
# review each snapshot, accept
git add app/tests/snapshots/
git commit -m "test(snapshots): add initial insta snapshot baselines"
```

### Updating snapshots intentionally

```bash
cargo insta test --review   # re-generates changed snapshots
# review diffs, accept intentional changes
git add app/tests/snapshots/
git commit -m "test(snapshots): update snapshots for <reason>"
```

### CI behavior

No CI changes needed. `cargo test` (via `make test`) fails automatically on any snapshot mismatch. Developers must run `cargo insta review` locally to accept changes and commit updated `.snap` files.

## Files Modified

| File                     | Change                                           |
| ------------------------ | ------------------------------------------------ |
| `app/Cargo.toml`         | Add `insta = "1"` dev-dependency                 |
| `app/tests/snapshots.rs` | New file — 5 snapshot tests                      |
| `app/tests/snapshots/`   | New directory — committed `.snap` baseline files |
