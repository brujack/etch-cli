# Spec: powershell.module Action

## Status: Pending

## Overview

Add a `powershell.module` action that installs PowerShell modules from PSGallery using `pwsh` (PowerShell Core). Idempotent via `Get-Module -ListAvailable`. Supports single-module and list-of-modules installs. Targets macOS and Linux (both use `pwsh`).

## Fields

| Field   | Type              | Required | Default       | Description                                                 |
| ------- | ----------------- | -------- | ------------- | ----------------------------------------------------------- |
| `name`  | `Option<String>`  | no       | —             | Single module name. Mutually exclusive with `list`.         |
| `list`  | `Vec<String>`     | no       | `[]`          | Multiple module names. Preferred over `name` when both set. |
| `scope` | `PowerShellScope` | no       | `CurrentUser` | Install scope: `CurrentUser` or `AllUsers`.                 |

At least one of `name` or `list` must be provided; plan returns an error otherwise.

## Scope Enum

```rust
pub enum PowerShellScope {
    CurrentUser,  // default
    AllUsers,
}
```

Serialised as `"CurrentUser"` / `"AllUsers"` (Pascal case matching PowerShell convention). `AllUsers` requires admin rights — etch emits the command; if insufficient privilege, `pwsh` errors at runtime.

## YAML Examples

```yaml
# Single module
- action: powershell.module
  name: oh-my-posh

# List of modules
- action: powershell.module
  list:
      - Az
      - AWSPowerShell.NetCore
      - Microsoft.Graph
      - oh-my-posh

# Explicit scope (usually not needed)
- action: powershell.module
  name: oh-my-posh
  scope: CurrentUser
```

## Idempotency

Per-module check in `plan()`:

```
pwsh -Command "if (Get-Module -ListAvailable -Name '<name>') { exit 0 } else { exit 1 }"
```

- Exit 0 → already installed → skip
- Exit 1 → not installed → include in install batch
- `pwsh` absent from PATH → `unwrap_or(false)` → false → include in install batch (fail-safe, consistent with gem/npm/pip pattern)

## Install Step

One `Exec` atom batching all uninstalled modules:

```
pwsh -Command "Install-Module -Name '<m1>','<m2>' -Scope CurrentUser -Force -AllowClobber"
```

- `-Force` — suppresses confirmation prompts for unattended install
- `-AllowClobber` — allows overwriting commands from existing modules
- If all modules already installed: returns `Ok(vec![])` — no step emitted

## File Layout

```
lib/src/actions/powershell/
├── mod.rs          # re-export: mod module; pub use module::PowershellModule;
└── module.rs       # PowershellModule struct + PowerShellScope enum + impl Action
```

Register in `lib/src/actions/mod.rs`:

- `mod powershell;` module declaration
- `use powershell::PowershellModule;` import
- Enum variant: `PowershellModule(ConditionalVariantAction<PowershellModule>)`
- `#[serde(rename = "powershell.module")]`
- Match arms in `inner_ref()`, `notify`, `Deref`, `Display`
- Update action count in dispatch tests; add YAML entry to three test lists

`examples/powershell/powershell-module.yaml` — one entry per option combination.

## Tests

Following gem/npm/pip patterns exactly.

**Deserialization:**

- `it_can_be_deserialized` — single name
- `it_can_be_deserialized_with_list`
- `it_can_be_deserialized_with_scope`
- `scope_defaults_to_current_user`

**Summarize:**

- `summarize_includes_module_name`
- `summarize_includes_all_list_modules`
- `summarize_with_no_modules_returns_generic_message`

**module_names helper:**

- `module_names_prefers_list_when_both_set`
- `module_names_returns_single_name_as_vec`
- `module_names_empty_when_no_name_or_list`

**plan():**

- `plan_errors_without_name_or_list`
- `plan_returns_exec_for_uninstalled_module` — fake module name; real or absent pwsh both generate step
- `plan_returns_exec_for_uninstalled_list`
- `plan_skips_already_installed_module` — PATH mock: fake pwsh exits 0
- `plan_skips_already_installed_modules_in_list` — PATH mock
- `plan_generates_step_when_pwsh_not_in_path` — PATH=/nonexistent; unwrap_or(false) → step generated
- `plan_includes_scope_in_command` — scope field appears in generated command string
- `plan_includes_force_and_allowclobber` — flags present in generated command

## Coverage

Structurally uncoverable: the `pwsh` execution at apply time. Unit tests cover `plan()` entirely via PATH mocking. Expected coverage contribution: ~15 coverable lines, all reachable in tests.
