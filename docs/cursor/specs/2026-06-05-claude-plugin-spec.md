# Spec: claude.plugin — Claude Code Plugin Management

**Date:** 2026-06-05  
**Status:** In Progress

## Problem

`etch update --claude` requires a `update.claude.plugins` list in `etch.yaml` to know which plugins to update — a second list the user must maintain alongside their manifests. There is no declarative action to ensure Claude plugins are installed, so install is handled by ad-hoc `command.run` steps or out-of-band scripts.

## Solution

Two changes shipped together:

1. **`claude.plugin` action** — declarative, idempotent install of Claude Code plugins. At plan time, checks which are already installed and skips them. Mirrors the `npm.install` pattern.

2. **`etch update --claude` auto-discovery** — remove `ClaudeUpdateConfig.plugins` (now unused). `update_claude()` discovers all installed plugins from `claude plugins list` at update time and runs `claude plugins update <name>` for each. No more duplicate list in `etch.yaml`.

---

## Action: `claude.plugin`

### YAML Schema

```yaml
# Single plugin
- action: claude.plugin
  name: superpowers

# Single plugin with explicit marketplace
- action: claude.plugin
  name: superpowers@claude-plugins-official

# Multiple plugins
- action: claude.plugin
  list:
      - superpowers
      - code-review
      - context7
      - context-mode
      - caveman
      - firecrawl
      - rust-analyzer-lsp
      - pyright-lsp
```

Fields:

| Field  | Type             | Required                | Notes                                      |
| ------ | ---------------- | ----------------------- | ------------------------------------------ |
| `name` | `Option<String>` | One of `name` or `list` | Plugin name, optionally `name@marketplace` |
| `list` | `Vec<String>`    | One of `name` or `list` | List of plugin names                       |

### Behavior

At plan time:

1. Run `claude plugins list` and capture stdout.
2. Parse output: extract installed plugin base names (part before `@`) from lines matching `❯ <name>@<marketplace>`.
3. For each requested plugin: strip `@marketplace` suffix if present to get the base name. If base name is in the installed set → skip.
4. For each uninstalled plugin: generate one `Exec` step — `claude plugins install <name>` — with `streaming: true`.

At apply time: `claude plugins install` is run for each missing plugin. Output streams to terminal.

**Bail conditions:**

- Neither `name` nor `list` specified → `bail!("claude.plugin requires either 'name' or 'list'")`

**Idempotency guarantee:** calling `.plan()` twice with the same inputs produces the same steps. Re-running `etch apply` is a no-op if all listed plugins are already installed.

**Fail-safe:** if `claude plugins list` fails (command not found, non-zero exit), treat as "nothing installed" → generate steps for all requested plugins.

### Step Initializer

No `SkipIf` initializer. Idempotency is handled at plan time by filtering installed plugins out of the step list (same pattern as `npm.install`).

### Platform Constraints

No `where:` clause needed. Claude CLI is available on both macOS and Linux.

### File Locations

```
lib/src/actions/claude/           # new namespace
lib/src/actions/claude/plugin.rs  # ClaudePlugin struct + impl
lib/src/actions/claude/mod.rs     # pub mod plugin; pub use plugin::ClaudePlugin;
lib/src/actions/mod.rs            # 7 registration points
```

### Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudePlugin {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
}
```

### Action Catalog Entry

> `claude.plugin` — Ensure Claude Code plugins are installed. Skips already-installed plugins at plan time. Use `etch update --claude` to upgrade.

---

## `etch update --claude` Changes

### Current behavior

`update_claude()` iterates over `config.plugins: Vec<String>` from `ClaudeUpdateConfig` (read from `etch.yaml`) and runs `claude plugins update <name>` for each.

### New behavior

`update_claude()` discovers installed plugins at update time:

1. Run `claude plugins list`, parse output to extract `name@marketplace` tokens.
2. For each installed plugin, run `claude plugins update <name@marketplace>` (full token, preserving marketplace).
3. Count successes and failures, return `UpdateStepResult` as before.

### Config change

Remove `plugins: Vec<String>` from `ClaudeUpdateConfig`. Keep `npm_globals: Vec<String>` (unrelated).

**Implementation note:** verify whether `ClaudeUpdateConfig` (or any parent config struct) uses `#[serde(deny_unknown_fields)]`. If it does, removing the field will cause existing `etch.yaml` files with `plugins:` to fail deserialization. In that case: retain the field as `#[serde(default, skip_serializing)]` so it is accepted and silently ignored rather than removed. If `deny_unknown_fields` is not set, the field can be deleted outright.

```rust
// Before
pub struct ClaudeUpdateConfig {
    pub plugins: Vec<String>,
    pub npm_globals: Vec<String>,
}

// After
pub struct ClaudeUpdateConfig {
    pub npm_globals: Vec<String>,
}
```

---

## Testing

### `claude/plugin.rs` unit tests

| Test                                          | Verifies                                                    |
| --------------------------------------------- | ----------------------------------------------------------- |
| `it_can_be_deserialized`                      | YAML with `name:` round-trips to `ClaudePlugin`             |
| `it_can_be_deserialized_with_list`            | YAML with `list:` round-trips                               |
| `plan_errors_without_name_or_list`            | bail when both absent                                       |
| `summarize_includes_plugin_name`              | name appears in summary                                     |
| `summarize_includes_all_list_plugins`         | all list names appear in summary                            |
| `summarize_with_no_plugins_returns_generic`   | no panic when both absent                                   |
| `plugin_base_name_strips_marketplace`         | `foo@bar` → base name `foo`                                 |
| `plan_returns_exec_for_uninstalled_plugin`    | fake plugin not in list → step generated                    |
| `plan_skips_already_installed_plugin`         | fake `claude` binary reports plugin installed → empty steps |
| `plan_returns_empty_when_all_installed`       | all list entries installed → no steps                       |
| `plan_generates_step_when_claude_not_in_path` | fail-safe: claude missing → generate steps                  |
| `plan_handles_marketplace_suffix_in_name`     | `name: foo@bar` → base name `foo` matched against list      |

All tests using fake binaries or PATH manipulation: `#[serial]`.

### `config/mod.rs` tests

| Test                                                   | Verifies                                             |
| ------------------------------------------------------ | ---------------------------------------------------- |
| `claude_update_config_has_no_plugins_field`            | `ClaudeUpdateConfig` deserializes without `plugins:` |
| existing `claude_update_config_default_has_empty_vecs` | update to remove `plugins` assertion                 |

### `update.rs` tests

| Test                                        | Verifies                                                                    |
| ------------------------------------------- | --------------------------------------------------------------------------- |
| existing tests that assert `claude.plugins` | remove/update                                                               |
| `update_claude_discovers_from_list_output`  | fake `claude` binary returns known list → those plugins get update commands |

---

## What Does NOT Change

- `npm_globals` handling in `update_claude()` — unchanged
- `etch update --claude` CLI flag — unchanged
- All non-claude actions — unaffected
- `where:` behavior — no platform constraint needed
