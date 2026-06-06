# claude.plugin.update Action — Design Spec

**Date:** 2026-06-06
**Status:** Approved

## Summary

Add a `claude.plugin.update` action that updates already-installed Claude Code plugins via `claude plugins update <name>`. Complements the existing `claude.install` action (install-if-missing) by covering the update path.

## Motivation

`setup_env.sh` manages Claude plugins via both `claude plugins install` and `claude plugins update`. `claude.install` handles the install-if-missing case. There is no action for keeping installed plugins current.

## Action Name and Fields

YAML name: `claude.plugin.update`
Rust struct: `ClaudePluginUpdate`
File: `lib/src/actions/claude/plugin_update.rs`

```yaml
# Single plugin
- action: claude.plugin.update
  name: superpowers

# Multiple plugins
- action: claude.plugin.update
  list:
      - superpowers
      - context7
      - context-mode

# name@marketplace format supported
- action: claude.plugin.update
  name: superpowers@claude-plugins-official
```

Fields:

| Field  | Type             | Required         | Notes                                |
| ------ | ---------------- | ---------------- | ------------------------------------ |
| `name` | `Option<String>` | one of name/list | Single plugin; `name@marketplace` ok |
| `list` | `Vec<String>`    | one of name/list | Multiple plugins                     |

Validation: bail if both `name` and `list` are absent/empty.

## Plan Logic

No idempotency pre-check. Always emit update steps — `claude plugins update` is itself safe to re-run (no-ops if already current).

One `Exec` step per plugin:

```
command: claude
arguments: [plugins, update, <name>]
streaming: true
```

If `claude` is not in PATH, the step is still emitted — failure surfaces at execution time with a clear error from the atom.

## Registration

Six edits to `lib/src/actions/mod.rs`:

1. Add `ClaudePluginUpdate` to the `use claude::{ ... }` import
2. Enum variant: `ClaudePluginUpdate(ConditionalVariantAction<ClaudePluginUpdate>)` with `#[serde(rename = "claude.plugin.update")]`
3. Match arm in `inner_ref()`
4. Match arm in `notify` accessor
5. Match arm in `Deref` impl
6. Match arm in `Display` impl → `"claude.plugin.update"`

Update the three dispatch tests (action count + YAML entries + `names.contains` assertion).

## Tests

In `lib/src/actions/claude/plugin_update.rs`:

| Test                                        | Asserts                             |
| ------------------------------------------- | ----------------------------------- |
| `it_can_be_deserialized_name`               | `name:` field parses                |
| `it_can_be_deserialized_list`               | `list:` field parses                |
| `summarize_includes_plugin_name`            | summarize contains the name         |
| `summarize_includes_all_list_plugins`       | summarize contains all list entries |
| `summarize_with_no_plugins_returns_generic` | default summarize is non-empty      |
| `plan_errors_without_name_or_list`          | bail on empty action                |
| `plan_returns_exec_for_name`                | single step, correct args           |
| `plan_returns_exec_for_list`                | one step per entry                  |

In `lib/src/actions/mod.rs` dispatch tests: update count to 45, add YAML entry, add `names.contains("claude.plugin.update")`.

## Example

`examples/claude.plugin.update/claude.plugin.update.yaml` — one entry for `name:` form, one for `list:` form, with inline comments.

## Out of Scope

- No install-if-missing fallback (that is `claude.install`)
- No enable/disable/uninstall
- No version pinning
