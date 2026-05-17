# command.run skip_if_exists — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

`command.run` currently has no way to skip execution when a path already exists. The workaround is an inline shell guard: `[ -d ~/.oh-my-zsh ] || RUNZSH=no ...`. This is verbose, error-prone, and leaks shell logic into what should be a declarative manifest.

The step system already has `FlowControl::SkipIf(Box<dyn Initializer>)` and a `FileExists` initializer. This spec exposes them via a new `skip_if_exists` field on `command.run`.

## Scope

**Modify:** `lib/src/actions/command/run.rs` only — add field and wire initializer.

## Field

```rust
pub struct RunCommand {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
    #[serde(default = "get_cwd")]
    pub dir: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub skip_if_exists: Option<String>,   // ← new
}
```

`None` by default (no `#[serde(default)]` annotation needed — `Option<T>` deserializes as `None` when absent).

## plan() Change

When `skip_if_exists` is set, append a `SkipIf(FileExists(path))` initializer after the existing `SetEnvVars` initializer:

```rust
use crate::steps::initializers::{FileExists, FlowControl as InitFlowControl};
use std::path::PathBuf;

let mut initializers = vec![InitFlowControl::Ensure(Box::new(
    SetEnvVars(self.env.clone()),
))];

if let Some(path) = &self.skip_if_exists {
    initializers.push(InitFlowControl::SkipIf(Box::new(
        FileExists(PathBuf::from(path)),
    )));
}
```

## YAML Interface

```yaml
# Skip oh-my-zsh install if already installed
- action: command.run
  command: bash
  args:
      - "-c"
      - 'RUNZSH=no KEEP_ZSHRC=yes sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"'
  skip_if_exists: "{{ user.home_dir }}/.oh-my-zsh"

# Skip git clone if directory exists
- action: command.run
  command: git
  args:
      - clone
      - https://github.com/romkatv/powerlevel10k.git
      - "{{ user.home_dir }}/.oh-my-zsh/custom/themes/powerlevel10k"
  skip_if_exists: "{{ user.home_dir }}/.oh-my-zsh/custom/themes/powerlevel10k"
```

The path supports Tera templates (`{{ user.home_dir }}`) — manifest loading renders all YAML string values before deserialization.

## Testing

Three tests in `lib/src/actions/command/run.rs`:

| Test                                             | What it verifies                                            |
| ------------------------------------------------ | ----------------------------------------------------------- |
| `it_can_be_deserialized_with_skip_if_exists`     | YAML with `skip_if_exists:` → correct field value           |
| `plan_includes_skip_if_initializer_when_set`     | `skip_if_exists` set → 2 initializers (SetEnvVars + SkipIf) |
| `plan_excludes_skip_if_initializer_when_not_set` | `skip_if_exists` absent → 1 initializer (SetEnvVars only)   |

The existing `plan_returns_one_step_with_initializer_and_finalizer` test (which asserts exactly 1 initializer) must be updated to remain valid with the new optional field.

## Update Phase 2 dotfiles manifest

After implementation, update `~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml` to use `skip_if_exists:` instead of the inline guard for the oh-my-zsh install step.

## What Is NOT in Scope

- `skip_if_command_exists: command` (check PATH) — separate feature
- Multiple paths in a list — YAGNI
- `skip_if_output_contains` — YAGNI
