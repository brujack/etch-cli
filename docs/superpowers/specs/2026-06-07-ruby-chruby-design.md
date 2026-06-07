# ruby.chruby Action Design

## Overview

Add a `ruby.chruby` action that installs chruby via Homebrew and optionally writes a default Ruby version to `~/.ruby-version`. Simultaneously extend the existing `ruby.install` action so `version_manager: chruby` writes `~/.ruby-version` after installing the ruby.

## Motivation

`ruby.install` with `version_manager: chruby` is currently a no-op beyond ruby-install — it does not set a default ruby. There is also no action to install chruby itself. setup_env.sh handles both via `brew install chruby` and writing `~/.ruby-version`; this feature brings those steps into etch manifests.

## New Action: `ruby.chruby`

### YAML

```yaml
- action: ruby.chruby
  # optional: write this value to ~/.ruby-version
  # verbatim string; use the chruby directory-name format, e.g. "ruby-3.3.0"
  # omit to only install chruby without touching ~/.ruby-version
  default_version: "ruby-3.3.0"
```

### Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyChruby {
    /// Ruby version to set as default in ~/.ruby-version.
    /// Verbatim string written to the file (e.g. "ruby-3.3.0").
    /// If omitted, ~/.ruby-version is not written.
    pub default_version: Option<String>,
}
```

### Plan Logic

1. **Install chruby** — always emit `Exec { command: "brew", arguments: ["install", "chruby"] }`. `brew install` is idempotent: exits 0 with a warning when already installed.
2. **Set default** — if `default_version` is set, emit `SetContents { path: ~/.ruby-version, contents: b"{default_version}\n" }`.

### Idempotency

- Brew step: `brew install chruby` is a no-op when already installed — no plan-time query needed.
- `SetContents.plan()` reads the current file content and sets `should_run = false` when it already matches — idempotency is built into the atom.
- No command execution in `plan()` — keeps plan() pure and tests simple.

### Platform

No hard macOS gate in the action. Follows the existing brew-action convention: silently skips if `brew` is absent. Use `where: 'os.name == "macos"'` in the manifest for platform guarding.

## Extension: `ruby.install` with `version_manager: chruby`

### Change

When `version_manager: Some(VersionManager::Chruby)` and the ruby dir does not exist (install will proceed), append a `SetContents` step after the ruby-install step:

```
SetContents {
    path: ~/.ruby-version,
    contents: b"{impl_name}-{version}\n",
}
```

`impl_name` defaults to `"ruby"` (same as `impl_name()`), so `jruby-3.3.0` for JRuby installs.

### Idempotency

When `ruby_dir.exists()`, `plan()` already returns `Ok(vec![])` for the whole action — no steps, including the `SetContents` step. Correct: if the ruby is already installed the default was set on the first run.

### Existing Test Updates

- `plan_with_chruby_emits_one_step` → rename to `plan_with_chruby_emits_two_steps`, update assertion to 2 steps and verify the second step writes `ruby-{version}\n` to `~/.ruby-version`.
- `plan_skips_if_installed_with_chruby` → unchanged (still expects 0 steps).

## Combined Manifest Pattern

```yaml
# Install chruby itself (macOS only)
- action: ruby.chruby
  where: 'os.name == "macos"'

# Install ruby and set as default via chruby
- action: ruby.install
  version: "3.3.0"
  version_manager: chruby
  where: 'os.name == "macos"'
```

Or, when the ruby is already installed and only the default needs setting:

```yaml
- action: ruby.chruby
  default_version: "ruby-3.3.0"
  where: 'os.name == "macos"'
```

## Files

| File                               | Change                                                           |
| ---------------------------------- | ---------------------------------------------------------------- |
| `lib/src/actions/ruby/chruby.rs`   | New — `RubyChruby` struct + `impl Action`                        |
| `lib/src/actions/ruby/mod.rs`      | Export `chruby` module                                           |
| `lib/src/actions/ruby/install.rs`  | Extend chruby branch of `plan()` to emit `SetContents`           |
| `lib/src/actions/mod.rs`           | Register `ruby.chruby` variant (6 edits per CLAUDE.md checklist) |
| `examples/ruby/ruby-chruby.yaml`   | New example                                                      |
| `docs/knowledge/action-catalog.md` | Add `ruby.chruby` row                                            |
| `README.md`                        | Add `ruby.chruby` to action catalog table                        |

## Tests

### `ruby.chruby` (in `chruby.rs`)

| Scenario                                 | Steps                          |
| ---------------------------------------- | ------------------------------ |
| No `default_version`                     | 1 (brew install)               |
| `default_version` set                    | 2 (brew install + SetContents) |
| Deserialization: bare action (no fields) | `default_version: None`        |
| Deserialization: with `default_version`  | field populated                |

### `ruby.install` (in `install.rs`)

| Scenario                                   | Steps                                                     |
| ------------------------------------------ | --------------------------------------------------------- |
| `version_manager: chruby`, ruby dir absent | 2 (ruby-install + SetContents writing `ruby-{version}\n`) |
| `version_manager: chruby`, ruby dir exists | 0                                                         |
| `version_manager: chruby`, impl `jruby`    | SetContents writes `jruby-{version}\n`                    |
| `version_manager: rbenv` (regression)      | 3 (ruby-install + rbenv global + rbenv rehash)            |

## Out of Scope

- Sourcing chruby in shell RC — left to `file.link`/`file.copy` in the manifest.
- Linux chruby install (git clone) — macOS Homebrew only; Linux uses rbenv.
- `chruby` version pinning — always installs latest from Homebrew.
