# macos.service Action — Design Spec

## Overview

Add a `macos.service` action for declaratively managing macOS launchctl services (LaunchDaemons and LaunchAgents). Enables idempotent load/unload of system daemons and user agents from etch manifests.

## Problem

No etch action exists for enabling or disabling macOS daemons. Dotfiles currently use bare `launchctl load -w <plist>` shell scripts for SSH server and ARD, which are not idempotent and cannot be managed declaratively.

## Action Interface

```yaml
# System daemon — requires privileged: true
- action: macos.service
  plist: /System/Library/LaunchDaemons/com.apple.ssh.plist
  state: loaded
  privileged: true

# System daemon with explicit label (skip PlistBuddy label extraction)
- action: macos.service
  plist: /Library/LaunchDaemons/com.myapp.daemon.plist
  label: com.myapp.daemon
  state: loaded
  privileged: true

# User agent — no sudo needed
- action: macos.service
  plist: ~/Library/LaunchAgents/com.myapp.agent.plist
  state: loaded

# Unload a user agent
- action: macos.service
  plist: ~/Library/LaunchAgents/com.myapp.agent.plist
  state: unloaded
```

## Fields

| Field        | Type                | Required | Default | Description                                                                                                 |
| ------------ | ------------------- | -------- | ------- | ----------------------------------------------------------------------------------------------------------- |
| `plist`      | `String`            | yes      | —       | Path to the .plist file. Tilde expanded. Must exist at plan time.                                           |
| `label`      | `Option<String>`    | no       | —       | Service label for `launchctl list` idempotency check. If omitted, extracted from plist via `defaults read`. |
| `state`      | `MacOSServiceState` | yes      | —       | `loaded` or `unloaded`                                                                                      |
| `privileged` | `bool`              | no       | `false` | Run `launchctl` via `privilege_provider` (sudo). Required for system daemons.                               |

## Rust Data Model

```rust
// lib/src/actions/macos/service.rs
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacOSServiceState {
    Loaded,
    Unloaded,
}

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSService {
    pub plist: String,
    pub label: Option<String>,
    pub state: MacOSServiceState,
    #[serde(default)]
    pub privileged: bool,
}
```

## Implementation Architecture

### Action (`lib/src/actions/macos/service.rs`)

`Action::plan()`:

1. Expand tilde in `self.plist`.
2. Return `Err` if the plist file does not exist.
3. Resolve `privilege_provider` from contexts (default `"sudo"`).
4. Return one `Step` wrapping a `atoms::macos::Service` atom.

### Atom (`lib/src/atoms/macos/service.rs`)

`Atom::execute()`:

1. **Resolve label** — if `self.label` is set, use it. Otherwise call `defaults read <plist> Label` and parse stdout. Return `Err` if `defaults` fails or the output is empty. (`defaults` is in PATH; the command accepts `.plist` paths on modern macOS.)
2. **Check current state** — run `launchctl list <label>`. Exit 0 = loaded; non-zero = not loaded.
3. **Compare** — if current state already matches `self.state`, log and return `Ok(())` (idempotent skip).
4. **Act**:
    - `state: loaded` → `launchctl load -w <plist>` (via `privilege_provider` if `privileged`)
    - `state: unloaded` → `launchctl unload -w <plist>` (via `privilege_provider` if `privileged`)

The `-w` flag writes the `Disabled` key in the plist — load/unload persists across reboots.

### Atom Struct

```rust
// lib/src/atoms/macos/service.rs
pub struct Service {
    pub plist: PathBuf,
    pub label: Option<String>,
    pub state: MacOSServiceState,
    pub privileged: bool,
    pub privilege_provider: String,
}
```

## Idempotency

`launchctl list <label>` is the idempotency gate:

- Exit 0: service currently loaded
- Non-zero: service not loaded

If the current state equals the desired state, the atom returns `Ok(())` without calling `launchctl load`/`unload`.

## Privilege Handling

`privileged: true` wraps the `launchctl` call in the configured `privilege_provider` (default: `sudo`). Required for:

- System daemons in `/Library/LaunchDaemons/`
- System daemons in `/System/Library/LaunchDaemons/`

Not required for user agents in `~/Library/LaunchAgents/` or `/Library/LaunchAgents/` (user-scoped).

## Error Handling

| Error condition                                       | Where caught | Result                   |
| ----------------------------------------------------- | ------------ | ------------------------ |
| Plist file does not exist                             | `plan()`     | `Err` — step not created |
| PlistBuddy fails (malformed plist, label key missing) | `execute()`  | `Err` — step fails       |
| `launchctl load` / `unload` returns non-zero          | `execute()`  | `Err` — step fails       |

## Test Plan

Using PATH-mock pattern (same as `git.pull`). Mocks: `launchctl`, `defaults`. Tests marked `#[serial]` due to PATH mutation.

| Test                             | Scenario                                | Expected                            |
| -------------------------------- | --------------------------------------- | ----------------------------------- |
| already_loaded_skips             | state: loaded, launchctl list exits 0   | no load call                        |
| already_unloaded_skips           | state: unloaded, launchctl list exits 1 | no unload call                      |
| loads_when_unloaded              | state: loaded, launchctl list exits 1   | calls `launchctl load -w <plist>`   |
| unloads_when_loaded              | state: unloaded, launchctl list exits 0 | calls `launchctl unload -w <plist>` |
| uses_explicit_label              | label: provided                         | `defaults read` not called          |
| extracts_label_from_plist        | label: omitted                          | `defaults read` called once         |
| privileged_uses_sudo             | privileged: true                        | command run via privilege_provider  |
| plan_errors_if_plist_missing     | plist: /nonexistent/path.plist          | plan() returns Err                  |
| execute_errors_if_defaults_fails | `defaults read` exits non-zero          | execute() returns Err               |
| deserialization                  | action: macos.service yaml              | struct fields populated correctly   |

## Module Wiring

- Add `mod service;` + `pub use service::MacOSService;` to `lib/src/actions/macos/mod.rs`
- Add `mod service;` + `pub use service::Service;` to `lib/src/atoms/macos/mod.rs` (create `lib/src/atoms/macos/` if it doesn't exist)
- Add `MacOSService` variant to `Actions` enum in `lib/src/actions/mod.rs` with three match arms (plan, summarize, Actions::MacOSService)

## Enum Registration

```rust
// lib/src/actions/mod.rs — in Actions enum
#[serde(rename = "macos.service")]
MacOSService(ActionWrapper<MacOSService>),
```

## Examples

Create `examples/macos/service.yaml` covering:

- System SSH daemon (loaded, privileged)
- System ARD (Remote Desktop) daemon (loaded, privileged)
- User LaunchAgent (loaded, no sudo)
- Disabling a service (unloaded)

## CLAUDE.md Update

Add `macos.service` row to the Action Catalog table in `CLAUDE.md`.

## Out of Scope

- Bootstrap Domain (`launchctl enable` for newer macOS bootstrap subsystem) — future work
- `launchctl start`/`stop` (transient, non-persistent) — not needed for declarative provisioning
- Automatic `privileged:` inference from plist path — user must declare it explicitly
