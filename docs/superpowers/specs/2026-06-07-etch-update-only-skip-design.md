# etch update --only/--skip Design

## Overview

Replace the 10 individual per-category bool flags on `etch update` with two mutually exclusive filter flags: `--only <categories>` and `--skip <categories>`. Both accept comma-separated category names. The default (no flags) continues to run all categories.

## Motivation

The current API requires knowing every category name upfront and adding `--brew --rust --pip` for multi-category runs. `--only` and `--skip` express intent more directly and scale better as categories are added or removed.

## CLI Surface

```
etch update                        # run all (unchanged default)
etch update --only brew,rust       # run only brew and rust
etch update --skip pip,gems        # run all except pip and gems
etch update --only brew --skip pip # clap error: conflicting args
etch update --only foobar          # hard error: unknown category 'foobar'
                                   # valid: brew, system, mas, claude,
                                   #        packages, pip, rust, git-tools,
                                   #        gems, cheatsh
```

`--only` and `--skip` are declared with clap `conflicts_with` so providing both is a parse-time error.

## Struct Changes

Remove all 10 bool fields from `Update`. Add:

```rust
/// Run only these categories (comma-separated: brew,rust)
#[arg(long, value_delimiter = ',', conflicts_with = "skip")]
pub only: Vec<String>,

/// Skip these categories (comma-separated: pip,gems)
#[arg(long, value_delimiter = ',', conflicts_with = "only")]
pub skip: Vec<String>,
```

## Valid Categories

```
brew        system      mas         claude      packages
pip         rust        git-tools   gems        cheatsh
```

These map 1:1 to the existing 10 bool flags. The mapping is encoded as a `const` slice in the module for validation and help text.

## Selection Logic

`any_flag_set()` and `step_should_run(flag, run_all)` are deleted. Replaced by a method on `Update`:

```rust
fn should_run(&self, category: &str) -> bool {
    if !self.only.is_empty() {
        self.only.iter().any(|c| c == category)
    } else if !self.skip.is_empty() {
        !self.skip.iter().any(|c| c == category)
    } else {
        true
    }
}
```

All call sites in `execute()` change from:

```rust
if step_should_run(self.brew, run_all) {
```

to:

```rust
if self.should_run("brew") {
```

The `run_all` local variable in `execute()` is removed.

## Validation

At the top of `execute()`, before any steps run:

1. Collect all names from `self.only` and `self.skip` into one iterator
2. For each name not in `VALID_CATEGORIES`, return `Err` with message:
   `unknown category '{name}'; valid: brew, system, mas, claude, packages, pip, rust, git-tools, gems, cheatsh`
3. Mixed valid+invalid input (e.g. `--only brew,foobar`) catches the invalid name and errors

## Tests

### `should_run` unit tests (pure logic)

| Scenario                       | Assertion                        |
| ------------------------------ | -------------------------------- |
| `only=[]`, `skip=[]`           | every category returns `true`    |
| `only=[brew,rust]`             | brew→true, rust→true, pip→false  |
| `skip=[pip,gems]`              | brew→true, pip→false, gems→false |
| `only=[brew]`, category `brew` | true                             |
| `only=[brew]`, category `rust` | false                            |

### Validation tests

| Input                 | Expected                            |
| --------------------- | ----------------------------------- |
| `only=[foobar]`       | `Err` containing "unknown category" |
| `skip=[badname]`      | `Err` containing "unknown category" |
| `only=[brew,badname]` | `Err` (mixed valid+invalid)         |

### Deleted tests

`any_flag_set_false_when_all_default`, `any_flag_set_true_with_brew`, `any_flag_set_true_with_cheatsh`, `any_flag_set_true_with_git_tools`, `step_should_run_when_run_all`, `step_should_run_when_flag_set`, `step_should_not_run_when_not_run_all` — all deleted with their functions.

## Out of Scope

- Shell completion for category names (future)
- Config-file defaults for `--only`/`--skip`
- `--only` + `--skip` combined (clap error, not supported)
