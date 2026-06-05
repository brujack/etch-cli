# Spec: claude.marketplace, claude.marketplace.remove, package.remove

**Date:** 2026-06-05
**Status:** Pending

---

## Problem

Three gaps in the current action set:

1. No way to declaratively manage Claude Code plugin marketplaces (add/remove).
2. No way to remove installed packages via apt, snap, or homebrew.
3. apt removal has no `--purge` support (removes config files).

---

## Scope

Three new actions:

- `claude.marketplace` — ensure a Claude Code marketplace is registered
- `claude.marketplace.remove` — ensure a Claude Code marketplace is removed
- `package.remove` — uninstall packages via apt, snap, or homebrew; apt supports `--purge`

---

## Action 1: `claude.marketplace`

### YAML Surface

```yaml
- action: claude.marketplace
  name: caveman # marketplace handle (used for idempotency check)
  source: juliusbrussee/caveman # GitHub owner/repo or full git URL
  scope: user # optional: user (default) | project | local
  sparse: # optional: monorepo path filter
      - .claude-plugin
```

### Fields

| Field    | Type             | Required | Default | Notes                                       |
| -------- | ---------------- | -------- | ------- | ------------------------------------------- |
| `name`   | `String`         | yes      | —       | Handle used to identify marketplace in list |
| `source` | `String`         | yes      | —       | `owner/repo` (GitHub) or full git URL       |
| `scope`  | `Option<String>` | no       | `user`  | Passed as `--scope`; omitted when not set   |
| `sparse` | `Vec<String>`    | no       | `[]`    | Passed as `--sparse path1 path2 …`          |

### Behaviour

1. Run `claude plugins marketplace list`; parse `❯ <name>` lines via `parse_marketplace_list()`.
2. If `name` is already present → emit no steps (idempotent skip).
3. Otherwise emit one `Exec` step: `claude plugins marketplace add <source> [--scope <scope>] [--sparse <paths...>]`.

### CLI Command

```
claude plugins marketplace add <source> [--scope <scope>] [--sparse path...]
```

`--scope` is only appended when `scope` is explicitly set (non-None). `--sparse` is only appended when `sparse` is non-empty.

---

## Action 2: `claude.marketplace.remove`

### YAML Surface

```yaml
- action: claude.marketplace.remove
  name: caveman
  scope: user # optional: user | project | local; omit to remove from all scopes
```

### Fields

| Field   | Type             | Required | Default | Notes                                  |
| ------- | ---------------- | -------- | ------- | -------------------------------------- |
| `name`  | `String`         | yes      | —       | Marketplace handle to remove           |
| `scope` | `Option<String>` | no       | —       | Passed as `--scope`; omit = all scopes |

### Behaviour

1. Run `claude plugins marketplace list`; parse via `parse_marketplace_list()`.
2. If `name` is absent → emit no steps (idempotent skip).
3. Otherwise emit one `Exec` step: `claude plugins marketplace remove <name> [--scope <scope>]`.

### CLI Command

```
claude plugins marketplace remove <name> [--scope <scope>]
```

---

## Shared Helper: `parse_marketplace_list()`

Added to `lib/src/actions/claude/mod.rs` alongside the existing `parse_plugin_list()`.

Parses `claude plugins marketplace list` stdout. Output format:

```
Configured marketplaces:

  ❯ claude-plugins-official
    Source: GitHub (anthropics/claude-plugins-official)

  ❯ caveman
    Source: Git (https://github.com/juliusbrussee/caveman.git)
```

Returns `Vec<String>` of marketplace names (the token after `❯ `).

```rust
pub(crate) fn parse_marketplace_list(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('❯')
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}
```

Same structure as `parse_plugin_list()` — same `❯` prefix convention.

---

## Action 3: `package.remove`

### YAML Surface

```yaml
# Single package
- action: package.remove
  name: htop
  provider: apt
  where: 'os.family == "linux"'

# apt purge (removes config files)
- action: package.remove
  name: nginx
  provider: apt
  purge: true

# List form
- action: package.remove
  list: [htop, curl]
  provider: homebrew
  where: 'os.name == "macos"'

# Snap
- action: package.remove
  name: htop
  provider: snap
  where: "variables.has_snap"
```

### Fields

| Field      | Type               | Required | Default                                 | Notes                         |
| ---------- | ------------------ | -------- | --------------------------------------- | ----------------------------- |
| `name`     | `Option<String>`   | no\*     | —                                       | Single package name           |
| `list`     | `Vec<String>`      | no\*     | `[]`                                    | Multiple package names        |
| `provider` | `PackageProviders` | no       | OS-detected: Ubuntu→apt, macOS→homebrew | apt \| snap \| homebrew       |
| `purge`    | `bool`             | no       | `false`                                 | apt only: use `apt-get purge` |

\* One of `name` or `list` required.

### Behaviour

For each package in `name`/`list`:

1. Call `provider.installed_version(name)` — if `None` (not installed) → skip (idempotent).
2. If installed → emit one `Exec` step per package.

### CLI Commands

| Provider | Command                                                     |
| -------- | ----------------------------------------------------------- |
| apt      | `apt-get remove --yes <pkg>` or `apt-get purge --yes <pkg>` |
| snap     | `snap remove <pkg>`                                         |
| homebrew | `brew uninstall <pkg>`                                      |

All apt steps are `privileged: true`. Snap remove does not need `privileged` (snap escalates internally).

### Trait Change

Add `remove` to `PackageProvider` trait in `lib/src/actions/package/providers/mod.rs`:

```rust
fn remove(
    &self,
    package: &PackageVariant,
    purge: bool,
    contexts: &Contexts,
) -> anyhow::Result<Vec<Step>>;
```

`purge` is ignored by snap and homebrew implementations.

---

## Implementation Files

### New files

| File                                           | Contents                                |
| ---------------------------------------------- | --------------------------------------- |
| `lib/src/actions/claude/marketplace.rs`        | `ClaudeMarketplace` struct + impl       |
| `lib/src/actions/claude/marketplace_remove.rs` | `ClaudeMarketplaceRemove` struct + impl |
| `lib/src/actions/package/remove.rs`            | `PackageRemove` struct + impl           |
| `examples/claude/claude-marketplace.yaml`      | Example manifest                        |
| `examples/package/package-remove.yaml`         | Example manifest                        |

### Modified files

| File                                             | Change                                                                          |
| ------------------------------------------------ | ------------------------------------------------------------------------------- |
| `lib/src/actions/claude/mod.rs`                  | Add `marketplace`, `marketplace_remove` modules; add `parse_marketplace_list()` |
| `lib/src/actions/package/providers/mod.rs`       | Add `fn remove()` to `PackageProvider` trait                                    |
| `lib/src/actions/package/providers/aptitude.rs`  | Implement `remove()` (with/without purge)                                       |
| `lib/src/actions/package/providers/snapcraft.rs` | Implement `remove()`                                                            |
| `lib/src/actions/package/providers/homebrew.rs`  | Implement `remove()`                                                            |
| `lib/src/actions/mod.rs`                         | Register 3 new action variants (6 edits each)                                   |
| `README.md`                                      | Add 3 entries to action catalog table                                           |
| `docs/knowledge/action-catalog.md`               | Add 3 entries                                                                   |
| `docs/superpowers/README.md`                     | Add row to All Plans table                                                      |

---

## Testing

### `claude.marketplace`

- `plan_skips_when_marketplace_already_present` — list output contains name → empty steps
- `plan_adds_when_marketplace_absent` — list output missing name → one step with correct args
- `plan_includes_scope_when_set` — scope field → `--scope` in step args
- `plan_includes_sparse_when_set` — sparse field → `--sparse` in step args
- `plan_omits_scope_when_not_set` — no `--scope` in args
- `parse_marketplace_list_extracts_names` — unit test for parser
- `parse_marketplace_list_empty` — empty input → empty vec
- Deserialize round-trip

### `claude.marketplace.remove`

- `plan_skips_when_marketplace_absent` — not in list → empty steps
- `plan_removes_when_marketplace_present` — in list → one step
- `plan_includes_scope_when_set`
- `plan_omits_scope_when_not_set`
- Deserialize round-trip

### `package.remove`

- `plan_skips_when_package_not_installed` — `installed_version()` returns None → empty
- `plan_removes_when_installed` — returns Some → one step per package
- `plan_uses_purge_flag_for_apt` — `purge: true` → `purge` in args
- `plan_does_not_use_purge_for_snap` — `purge: true` ignored for snap
- `plan_does_not_use_purge_for_homebrew` — `purge: true` ignored for homebrew
- `plan_removes_all_packages_in_list` — list form → one step per package
- Deserialize round-trip (name, list, purge)

---

## Error Handling

- `claude` not in PATH: `bail!` with message (same pattern as `claude.install`)
- `claude plugins marketplace list` fails: propagate error
- Package provider unavailable: return error (same as `package.install`)
- Neither `name` nor `list` set on `package.remove`: return error at plan time

---

## Out of Scope

- `claude.marketplace.update` (updating marketplace from source) — separate feature
- `package.remove` for providers other than apt, snap, homebrew
- apt `--purge` equivalent for snap (`snap remove --purge` exists but not requested)
