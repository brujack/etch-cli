# package.upgrade Action Design

## Overview

Add a `package.upgrade` action for upgrading installed packages via apt and snap providers. Homebrew upgrades are already handled by the existing `brew.upgrade` action and are not duplicated here.

## Motivation

`package.install` is presence-only idempotent — it installs a package if absent but does not upgrade it if already installed. There is no action today that upgrades existing packages via apt or snap. Dotfiles manifests that want to keep packages current have no declarative way to express this.

## Manifest Syntax

```yaml
# Upgrade all upgradable apt packages
- action: package.upgrade
  provider: apt

# Upgrade specific apt packages (only if already installed)
- action: package.upgrade
  provider: apt
  list: [git, curl, vim]

# Upgrade a single apt package
- action: package.upgrade
  provider: apt
  name: git

# Refresh all installed snaps
- action: package.upgrade
  provider: snap

# Refresh a specific snap
- action: package.upgrade
  provider: snap
  name: code
```

**Homebrew:** Use `brew.upgrade` directly. `package.upgrade` with `provider: homebrew` fails at plan time with a clear error and a suggestion to use `brew.upgrade` instead.

**`name:` and `list:` are mutually exclusive.** Specifying both fails at plan time.

## Architecture

### New files

**`lib/src/actions/package/upgrade.rs`**

```rust
pub struct PackageUpgrade {
    pub provider: String,
    pub name: Option<String>,
    pub list: Option<Vec<String>>,
}
```

`plan()` logic:

1. Validate: `provider == "homebrew"` → error at plan time with message `"use brew.upgrade directly for Homebrew upgrades"`.
2. Validate: both `name` and `list` are `Some` → error: `"name and list are mutually exclusive"`.
3. Validate: unknown provider → error: `"unknown provider: <provider>; supported: apt, snap"`.
4. Delegate to the provider-specific plan function.

`upgradeable()` method — returns `true` for all `PackageUpgrade` instances. Reserved for a future `etch update` command that identifies upgrade-capable atoms.

**`lib/src/actions/package/providers/apt_upgrade.rs`**

Plan logic:

- Run `apt list --upgradable 2>/dev/null` and strip the `"Listing..."` header line.
- If `name` or `list` is set: filter output to named packages; if none of the named packages appear in the upgradable list, return `Ok(vec![])` — nothing to do.
- If all packages (or no filter) shows no upgradable output: return `Ok(vec![])`.
- Otherwise return two steps: an `apt-get update` step followed by an `apt-get install --only-upgrade -y <pkgs>` step (or `apt-get upgrade -y` when no packages are named). Both steps run with `privileged: true` — apt always requires root.
- Dry-run output: list of package names that would be upgraded.

`--only-upgrade` flag semantics: upgrades already-installed packages only; never installs new packages as a side effect. This is the safe default — unexpected dependency installs are suppressed.

**`lib/src/actions/package/providers/snap_upgrade.rs`**

Plan logic:

- Run `snap refresh --list 2>/dev/null`.
- If `name` is set: check whether the named snap appears in the list; if not, return `Ok(vec![])`.
- If output is empty (no snaps have updates): return `Ok(vec![])`.
- Otherwise return a single step: `snap refresh` (all) or `snap refresh <name>` (specific). Snap handles its own privilege escalation — `privileged: false`.
- Dry-run output: snap names that would be refreshed.

### Modified files

**`lib/src/actions/package/mod.rs`** — add `pub mod upgrade;`

**`lib/src/actions/package/providers/mod.rs`** — add `pub mod apt_upgrade;` and `pub mod snap_upgrade;`

**`lib/src/actions/mod.rs`** — register `Actions::PackageUpgrade`, add to all match arms

## Error Handling

| Condition                                      | Behavior                                                                  |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| `provider: homebrew`                           | `plan()` fails: `"use brew.upgrade directly for Homebrew upgrades"`       |
| `name:` and `list:` both set                   | `plan()` fails: `"name and list are mutually exclusive"`                  |
| Unknown provider                               | `plan()` fails: `"unknown provider: <p>; supported: apt, snap"`           |
| `apt-get update` fails                         | Error propagated; upgrade step does not run                               |
| `apt-get install --only-upgrade` fails mid-way | Error propagated with stderr; apt's own locking prevents index corruption |
| Named package not installed (apt)              | `--only-upgrade` skips it silently; no error                              |
| Named snap not installed                       | `snap refresh <name>` returns non-zero; error propagated as-is            |
| No internet / mirror unreachable               | Provider CLI error propagated as-is                                       |
| Nothing to upgrade                             | `Ok(vec![])` — no steps, no output, handlers not triggered                |

## Testing

**Unit tests (`upgrade.rs`):**

- Deserialization: `provider`, `name`, `list` fields parse correctly from YAML
- Deserialization: omitted `name`/`list` deserialize as `None`
- Plan-time error: `provider: homebrew` returns error containing `"brew.upgrade"`
- Plan-time error: `name` and `list` both set returns error containing `"mutually exclusive"`
- Plan-time error: unknown provider returns error containing `"unknown provider"`
- `upgradeable()` returns `true`

**Unit tests (`apt_upgrade.rs`):**

- Mock `apt list --upgradable` returning empty output → `plan()` returns `Ok(vec![])`
- Mock returning upgradable packages with no filter → two steps (update + upgrade)
- Mock returning upgradable packages with `name: git` in the list → two steps with `--only-upgrade git`
- Mock returning upgradable packages with `name: git` not in the list → `Ok(vec![])`
- Mock returning upgradable packages with `list: [git, curl]`, only `git` upgradable → step includes only `git`

**Unit tests (`snap_upgrade.rs`):**

- Mock `snap refresh --list` returning empty output → `Ok(vec![])`
- Mock returning snap names with no filter → one step (`snap refresh`)
- Mock returning snap names with `name: code` in the list → one step (`snap refresh code`)
- Mock returning snap names with `name: code` not in the list → `Ok(vec![])`

No live package manager tests. All provider tests use command output mocking via injected stdout strings.
