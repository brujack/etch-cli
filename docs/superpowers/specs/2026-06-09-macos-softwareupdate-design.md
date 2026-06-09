# macos.softwareupdate Action — Design Spec

## Overview

Add a `macos.softwareupdate` action that runs `softwareupdate --install --all` on macOS. Zero-field, privileged, self-idempotent.

## Motivation

`setup_env.sh` runs `softwareupdate --install --all` during the update workflow. No etch-cli action exists for this. Users must shell out via `command.run`, which doesn't signal intent or respect the privilege provider.

## Design

### Action struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "macos.softwareupdate")]
pub struct MacOSSoftwareUpdate {}
```

No fields. Zero configuration surface — the only sensible invocation is `softwareupdate --install --all`.

### Plan output

Single step:

- **Atom:** `Exec { command: "softwareupdate", arguments: ["--install", "--all"], privileged: true, privilege_provider, … }`
- **Initializers:** none — `softwareupdate` is self-idempotent (exits 0 and prints "No new software available." when nothing to install)
- **Finalizers:** none

### Privilege

`privileged: true`. Installing system updates requires admin. Privilege provider resolved from context (defaults to `sudo`).

### YAML name

```yaml
- action: macos.softwareupdate
```

Users add `where: 'os.name == "macos"'` in their manifests. The action does not enforce the OS guard internally — consistent with all other `macos.*` actions.

### Module registration

Follows the standard 6-edit pattern in `lib/src/actions/mod.rs`:

- `mod macos` already exists; add `softwareupdate.rs` inside `lib/src/actions/macos/`
- Export from `macos/mod.rs`
- Register variant, `inner_ref`, `notify`, `Deref`, `Display` arms in `actions/mod.rs`
- Update three test YAML lists and variant counts

### Example manifest entry

```yaml
- action: macos.softwareupdate
  where: 'os.name == "macos"'
```

## Tests

Unit tests in `lib/src/actions/macos/softwareupdate.rs`:

| Test                                        | Assertion                                                                |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| `it_can_be_deserialized`                    | `serde_yaml_ng` round-trips to `MacOSSoftwareUpdate`                     |
| `summarize_returns_non_empty_string`        | `summarize()` not empty                                                  |
| `plan_returns_one_step`                     | `plan()` returns exactly 1 step                                          |
| `plan_step_runs_softwareupdate_install_all` | atom `to_string()` contains `"softwareupdate"`, `"--install"`, `"--all"` |
| `plan_step_is_privileged`                   | atom `to_string()` contains `"privileged=true"`                          |
| `plan_step_has_no_initializers`             | `steps[0].initializers` is empty                                         |

No integration tests — action shells out to `softwareupdate`, which requires a live macOS environment with pending updates.

## Files changed

| File                                                      | Change                                                                     |
| --------------------------------------------------------- | -------------------------------------------------------------------------- |
| `lib/src/actions/macos/softwareupdate.rs`                 | new — action impl + unit tests                                             |
| `lib/src/actions/macos/mod.rs`                            | add `pub mod softwareupdate; pub use softwareupdate::MacOSSoftwareUpdate;` |
| `lib/src/actions/mod.rs`                                  | register variant + 5 match arms + update test YAML lists                   |
| `examples/macos-softwareupdate/macos-softwareupdate.yaml` | new example manifest                                                       |
| `docs/superpowers/README.md`                              | add row, status In Progress                                                |
| `README.md`                                               | add `macos.softwareupdate` to action catalog                               |
