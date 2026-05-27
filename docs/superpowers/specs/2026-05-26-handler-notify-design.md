# Handler/Notify Pattern — Design Spec

## Overview

Add Ansible-style handlers to etch manifests. Actions declare `notify: [handler-name]`; named handlers defined in a `handlers:` section run once at the end of the manifest if any notifying action made a change.

## Problem

No way to run an action only when a preceding action made a change. Example: `macos.defaults` sets dock preferences, but `killall Dock` must run unconditionally or not at all — there is no "run this only if something above changed" mechanism. Workaround is bare shell scripts outside etch, defeating declarative management.

## YAML Interface

```yaml
actions:
    - action: macos.defaults
      domain: com.apple.dock
      key: autohide
      kind: bool
      value: "true"
      notify: [restart-dock]

    - action: macos.defaults
      domain: com.apple.dock
      key: tilesize
      kind: integer
      value: "48"
      notify: [restart-dock] # same handler — runs once

    - action: systemd.service
      unit: nginx.service
      enabled: true
      started: true
      notify: [reload-nginx]
      where: 'os.family == "linux"'

handlers:
    - name: restart-dock
      action: command.run
      command: killall
      args: [Dock]

    - name: reload-nginx
      action: command.run
      command: systemctl
      args: [reload, nginx]
      privileged: true
      where: 'os.family == "linux"'
```

## Fields

### On actions (`ConditionalVariantAction<T>`)

| Field    | Type          | Default | Description                                             |
| -------- | ------------- | ------- | ------------------------------------------------------- |
| `notify` | `Vec<String>` | `[]`    | Handler names to notify when this action makes a change |

`notify:` must be a YAML list. Single-handler case: `notify: [restart-dock]`.

### On manifests (`Manifest`)

| Field      | Type                   | Default | Description           |
| ---------- | ---------------------- | ------- | --------------------- |
| `handlers` | `Vec<ManifestHandler>` | `[]`    | Named handler actions |

### `ManifestHandler`

| Field             | Type     | Required | Description                                                  |
| ----------------- | -------- | -------- | ------------------------------------------------------------ |
| `name`            | `String` | yes      | Unique handler identifier within this manifest               |
| _(action fields)_ | —        | yes      | Any valid action (same YAML structure as `actions:` entries) |

Handlers support all action fields: `where:`, `privileged:`, `variants:`.

## Semantics

**When a handler is notified:**
An action notifies its handlers when all of its steps execute successfully (none fail, no finalizer abort). An action with `should_run = false` on all atoms produces no steps and never notifies.

**Deduplication:**
Multiple actions notifying the same handler name result in the handler running exactly once.

**Execution order:**
Handlers run in manifest declaration order, regardless of notification order.

**Scope:**
Per-manifest only. Handlers in one manifest cannot be notified by actions in another manifest.

**Handler `notify:` field:**
Ignored during handler execution. Handlers do not trigger other handlers (no chain reactions).

**Undeclared handler names:**
If an action notifies a name not present in `handlers:`, it is silently ignored.

**Duplicate handler names:**
If two handlers share a name, both run when that name is notified (declaration order).

## Rust Data Model

### `lib/src/actions/mod.rs`

```rust
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConditionalVariantAction<T> {
    #[serde(flatten)]
    pub action: T,
    #[serde(rename = "where")]
    pub condition: Option<String>,
    #[serde(default)]
    pub variants: Vec<Variant<T>>,
    #[serde(default)]
    pub notify: Vec<String>,   // new
}

impl Actions {
    pub fn notify(&self) -> &[String] {
        match self {
            Actions::CommandRun(a) => &a.notify,
            // ... all 23 variants ...
        }
    }
}
```

### `lib/src/manifests/mod.rs`

```rust
#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestHandler {
    pub name: String,
    #[serde(flatten)]
    pub action: Actions,
}

pub struct Manifest {
    // existing fields unchanged ...
    #[serde(default)]
    pub handlers: Vec<ManifestHandler>,
}
```

`ManifestHandler` uses `#[serde(flatten)]` on `Actions` — the YAML structure is identical to a regular action entry plus a `name:` field.

## Execution Flow

Changes in `app/src/commands/apply.rs`:

```
per-manifest:
  let mut notified: IndexSet<String> = IndexSet::new()  // ordered + dedup

  for raw_action in manifest.actions:
    plan → filter initializers → filter atom.plan().should_run → peekable

    if steps empty → continue (no notification)

    let all_succeeded = true
    let steps_ran = 0
    for step in steps:
      if dry_run: record would-run count; continue
      execute step
        Ok  → steps_ran += 1
        Err → successful = false; all_succeeded = false; break
      if finalizers abort → successful = false; all_succeeded = false; break

    if !dry_run && all_succeeded && steps_ran > 0:
      notified.extend(raw_action.notify())

    if dry_run && dry_run_count > 0:
      for name in raw_action.notify():
        print "[dry run] handler '{name}' would run"

  // Run notified handlers in declaration order
  for handler in manifest.handlers:
    if !notified.contains(handler.name): continue

    plan handler action → filter initializers → filter atom.plan().should_run → peekable
    if steps empty: continue

    for step in steps:
      execute step
        Ok  → ()
        Err → successful = false; break  ← stops THIS handler's steps
                                           outer loop continues (behavior B)
```

`IndexSet` is from the `indexmap` crate (already in `Cargo.toml`).

## Error Handling

| Scenario                  | Behavior                                                                    |
| ------------------------- | --------------------------------------------------------------------------- |
| Handler fails             | `successful = false`; remaining handlers continue                           |
| Handler `where:` false    | `plan()` returns empty steps → skipped silently                             |
| Handler name not declared | `notified.contains()` never matches → silently ignored                      |
| Action fails mid-steps    | `all_succeeded = false` → action does not notify                            |
| Dry run                   | Handlers not executed; would-fire names printed after each notifying action |

## Out of Scope

- Cross-manifest handler notification
- Handler-to-handler chain reactions
- `notify:` on handlers themselves (field is present but ignored at handler execution time)
- `handler.flush` or explicit flush actions

## Module Changes

| File                                   | Change                                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------------------------ |
| `lib/src/actions/mod.rs`               | Add `notify: Vec<String>` to `ConditionalVariantAction<T>`; add `Actions::notify()` method |
| `lib/src/manifests/mod.rs`             | Add `ManifestHandler` struct; add `handlers: Vec<ManifestHandler>` to `Manifest`           |
| `app/src/commands/apply.rs`            | Collect notifications during action loop; run handlers after                               |
| `examples/handler-notify/service.yaml` | Example manifest with handlers                                                             |

## Test Plan

### Unit tests (`lib/src/actions/mod.rs`, `lib/src/manifests/mod.rs`)

| Test                                                 | Scenario                                                        |
| ---------------------------------------------------- | --------------------------------------------------------------- |
| `notify_deserializes_from_yaml`                      | `notify: [foo, bar]` on an action deserializes correctly        |
| `notify_defaults_empty`                              | Action with no `notify:` field → empty Vec                      |
| `handlers_section_deserializes`                      | `handlers:` with one entry → `ManifestHandler` populated        |
| `handler_not_triggered_when_no_steps_ran`            | `should_run=false` → steps empty → no notification              |
| `handler_triggered_when_steps_ran`                   | Steps execute → handler runs                                    |
| `handler_runs_once_when_multiple_actions_notify`     | Two actions notify same handler → runs once                     |
| `handler_runs_in_declaration_order`                  | Handler B declared before A; A notified before B → B runs first |
| `unknown_handler_name_silently_ignored`              | `notify:` references undeclared name → no error                 |
| `failed_handler_marks_unsuccessful_continues_others` | First handler fails → second runs; `successful=false`           |
| `handler_where_false_skips`                          | Handler `where:` evaluates false → skipped                      |
| `handler_not_triggered_when_action_fails`            | Action step fails → no notification                             |

### Integration test (`app/tests/integration.rs`)

One end-to-end test: manifest with two `command.run` actions (first writes a sentinel file, notifies handler), handler appends a line to a second file. Verifies handler ran exactly once regardless of how many actions notified it.
