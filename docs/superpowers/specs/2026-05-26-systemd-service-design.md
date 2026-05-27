# systemd.service Action — Design Spec

## Overview

Add a `systemd.service` action for declaratively managing systemd service units on Linux. Enables idempotent control of boot persistence (`enabled`/`disabled`) and runtime state (`started`/`stopped`) independently.

## Problem

No etch action exists for managing systemd services. Linux dotfiles currently use bare `systemctl enable --now` shell commands, which are not idempotent and cannot be managed declaratively.

## Action Interface

```yaml
# Enable and start SSH daemon
- action: systemd.service
  unit: sshd.service
  enabled: true
  started: true
  privileged: true
  where: 'os.family == "linux"'

# Disable bluetooth at boot, leave runtime state alone
- action: systemd.service
  unit: bluetooth.service
  enabled: false
  privileged: true

# Stop a service without disabling it
- action: systemd.service
  unit: cups.service
  started: false
  privileged: true

# Enable at boot without starting now
- action: systemd.service
  unit: nginx.service
  enabled: true
  privileged: true
```

## Fields

| Field        | Type           | Required | Default | Description                                                                                            |
| ------------ | -------------- | -------- | ------- | ------------------------------------------------------------------------------------------------------ |
| `unit`       | `String`       | yes      | —       | Service unit name (e.g. `sshd.service`). Accepted with or without `.service` suffix.                   |
| `enabled`    | `Option<bool>` | no       | —       | Boot persistence. `true` → `systemctl enable`, `false` → `systemctl disable`. Omit to leave unchanged. |
| `started`    | `Option<bool>` | no       | —       | Runtime state. `true` → `systemctl start`, `false` → `systemctl stop`. Omit to leave unchanged.        |
| `privileged` | `bool`         | no       | `false` | Wrap `systemctl` calls in `privilege_provider` (sudo). Required for system daemons.                    |

At least one of `enabled` or `started` must be set. `plan()` returns `Err` if both are `None`.

## Out of Scope

- `--user` scope (user-level systemd services) — system daemons only
- `systemctl mask`/`unmask` — not needed for dotfiles use cases
- `systemctl reload` / `restart` — transient operations, not declarative state

## Rust Data Model

```rust
// lib/src/actions/systemd/service.rs
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemdService {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    #[serde(default)]
    pub privileged: bool,
}
```

## Implementation Architecture

### Action (`lib/src/actions/systemd/service.rs`)

`Action::plan()`:

1. Return `Err` if both `enabled` and `started` are `None`.
2. Resolve `privilege_provider` from contexts (default `"sudo"`).
3. Return one `Step` wrapping a `atoms::systemd::Service` atom.

No filesystem check at plan time — `unit` is a service name, not a path.

### Atom (`lib/src/atoms/systemd/service.rs`)

```rust
pub struct Service {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    pub privileged: bool,
    pub privilege_provider: String,
}
```

`Atom::execute()` runs up to two sub-operations in order:

**1. enabled (if Some):**

- Run `systemctl is-enabled <unit>`, capture stdout
- stdout `"enabled"` (trimmed) = currently enabled; anything else = disabled
- If current state matches desired, log and skip
- Otherwise: `systemctl enable <unit>` or `systemctl disable <unit>`

**2. started (if Some):**

- Only runs if enabled step succeeded (or was skipped)
- Run `systemctl is-active <unit>`, check exit code
- Exit 0 = active; non-zero = inactive
- If current state matches desired, log and skip
- Otherwise: `systemctl start <unit>` or `systemctl stop <unit>`

`privileged: true` wraps each `systemctl` call with `privilege_provider`. Applied independently to each sub-operation.

### Idempotency

| Check command                 | Condition for "true"          |
| ----------------------------- | ----------------------------- |
| `systemctl is-enabled <unit>` | stdout trimmed == `"enabled"` |
| `systemctl is-active <unit>`  | exit code 0                   |

`is-enabled` can return: `enabled`, `disabled`, `static`, `masked`, `indirect`, `enabled-runtime`. Only `"enabled"` is treated as enabled.

## Error Handling

| Error condition                         | Where caught | Result                                   |
| --------------------------------------- | ------------ | ---------------------------------------- |
| Both `enabled` and `started` are `None` | `plan()`     | `Err` — step not created                 |
| `systemctl is-enabled` fails            | `execute()`  | `Err` — step fails                       |
| `systemctl enable`/`disable` fails      | `execute()`  | `Err` — started step skipped, step fails |
| `systemctl is-active` fails             | `execute()`  | `Err` — step fails                       |
| `systemctl start`/`stop` fails          | `execute()`  | `Err` — step fails                       |

## Module Wiring

- Create `lib/src/actions/systemd/mod.rs` + `service.rs`
- Create `lib/src/atoms/systemd/mod.rs` + `service.rs`
- Add `pub mod systemd;` to `lib/src/actions/mod.rs`
- Add `pub mod systemd;` to `lib/src/atoms/mod.rs`
- Add to `Actions` enum in `lib/src/actions/mod.rs`:
    ```rust
    #[serde(rename = "systemd.service")]
    SystemdService(ConditionalVariantAction<SystemdService>),
    ```
- Add three match arms: `inner_ref()`, `Deref::deref()`, `Display::fmt()`

## Test Plan

PATH-mock pattern (same as `macos.service`, `git.pull`). Mock `systemctl`. Tests marked `#[serial]`.

| Test                                  | Scenario                                | Expected                         |
| ------------------------------------- | --------------------------------------- | -------------------------------- |
| `plan_errors_if_neither_field_set`    | enabled: None, started: None            | `plan()` returns `Err`           |
| `plan_succeeds_with_enabled_only`     | enabled: Some(true), started: None      | one Step returned                |
| `plan_succeeds_with_started_only`     | enabled: None, started: Some(true)      | one Step returned                |
| `execute_skips_when_already_enabled`  | enabled: true, is-enabled → "enabled"   | no enable call                   |
| `execute_skips_when_already_disabled` | enabled: false, is-enabled → "disabled" | no disable call                  |
| `execute_skips_when_already_started`  | started: true, is-active exits 0        | no start call                    |
| `execute_skips_when_already_stopped`  | started: false, is-active exits 1       | no stop call                     |
| `execute_enables_when_disabled`       | enabled: true, is-enabled → "disabled"  | calls `systemctl enable <unit>`  |
| `execute_disables_when_enabled`       | enabled: false, is-enabled → "enabled"  | calls `systemctl disable <unit>` |
| `execute_starts_when_stopped`         | started: true, is-active exits 1        | calls `systemctl start <unit>`   |
| `execute_stops_when_started`          | started: false, is-active exits 0       | calls `systemctl stop <unit>`    |
| `execute_handles_enabled_and_started` | both set, both need change              | enable called, then start called |
| `execute_errors_if_enable_fails`      | systemctl enable exits non-zero         | `execute()` returns `Err`        |
| `execute_errors_if_start_fails`       | systemctl start exits non-zero          | `execute()` returns `Err`        |
| `deserialize_enabled_started`         | YAML with both fields                   | struct populated correctly       |
| `deserialize_enabled_only`            | YAML with enabled only                  | started is None                  |

## Examples

Create `examples/systemd/service.yaml` covering:

- Enable and start (SSH)
- Disable and stop (bluetooth)
- Enable at boot without starting
- Stop without disabling
- With `where: 'os.family == "linux"'` guard
