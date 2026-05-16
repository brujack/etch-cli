# package.install cask Support — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

Homebrew cask installs (GUI applications like Alfred, iTerm2, 1Password) require `brew install --cask <name>`. Currently this works via `extra_args: ["--cask"]`, which is verbose and requires knowing the flag. A dedicated `cask: bool` field is more ergonomic and self-documenting.

This is the second of four Homebrew features (bundle → **cask** → mas → tap docs).

## Scope

**Modify:** `lib/src/actions/package/mod.rs` — add `cask` field to `Package` and `PackageVariant`, include in conversion  
**Modify:** `lib/src/actions/package/providers/homebrew.rs` — use `cask` in `install()`

## Data Flow

`Package.cask` is copied to `PackageVariant.cask` during the `From<&Package>` conversion. Variant-level overrides work: a variant entry can set `cask: true` to override a base `cask: false` (same as `name`, `list`, `extra_args`).

The `cask` field is provider-agnostic at the struct level. Non-Homebrew providers (Aptitude, Snapcraft) ignore it silently — same behaviour as `extra_args`.

## Struct Changes

### `Package` (in `lib/src/actions/package/mod.rs`)

Add after the `file` field:

```rust
#[serde(default = "get_false")]
pub cask: bool,
```

`get_false()` already exists in this module.

### `PackageVariant` (in `lib/src/actions/package/mod.rs`)

Add after the `file` field:

```rust
pub cask: bool,
```

### `From<&Package>` conversion

Include `cask` in the resolved variant output (same pattern as other fields). When a variant exists, use the variant's `cask` value; when no variant exists, use the base package's `cask` value.

## Homebrew Provider Change

In `lib/src/actions/package/providers/homebrew.rs`, in the `install()` method:

```rust
// Before:
arguments: [
    vec![String::from("install")],
    package.extra_args.clone(),
    need_installed,
].concat(),

// After:
let mut base = vec![String::from("install")];
if package.cask {
    base.push(String::from("--cask"));
}
arguments: [base, package.extra_args.clone(), need_installed].concat(),
```

`query()` requires no changes — it already checks both `Cellar/` (formulae) and `Caskroom/` (casks) when determining what needs to be installed.

## YAML Interface

```yaml
# Single cask
- action: package.install
  name: alfred
  provider: homebrew
  cask: true

# Multiple casks
- action: package.install
  list:
      - iterm2
      - 1password
      - alfred
  provider: homebrew
  cask: true

# With OS variant override
- action: package.install
  name: some-formula
  provider: homebrew
  variants:
      macos:
          name: some-cask
          cask: true
```

## Testing

New tests in `lib/src/actions/package/mod.rs` (or the homebrew provider file):

| Test                                         | What it verifies                                     |
| -------------------------------------------- | ---------------------------------------------------- |
| `it_can_be_deserialized_with_cask`           | `cask: true` in YAML → `PackageVariant.cask == true` |
| `install_includes_cask_flag_when_cask_true`  | Homebrew `install()` step contains `--cask` in args  |
| `install_excludes_cask_flag_when_cask_false` | Homebrew `install()` step does NOT contain `--cask`  |

## What Is NOT in Scope

- Validation that `cask` is only used with Homebrew (non-Homebrew providers silently ignore it)
- `brew install --cask` vs. `brew install` split into separate steps (single invocation is correct)
- Cask-specific `query()` changes (existing Caskroom check is sufficient)
