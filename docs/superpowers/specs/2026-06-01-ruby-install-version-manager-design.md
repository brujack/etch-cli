# Spec: ruby.install version_manager post-install steps

## Problem

`ruby.install` calls `ruby-install` to install a Ruby version but emits no post-install steps. When the target version manager is rbenv, two additional steps are required after installation:

1. `rbenv global <version>` — writes `~/.rbenv/version`, making the new Ruby the global default
2. `rbenv rehash` — regenerates shims in `~/.rbenv/shims` so executables are accessible on PATH

Without these steps, users must follow every `ruby.install` action with a `command.run` workaround. chruby has no equivalent post-install requirement (it auto-discovers rubies from configured paths).

## Design

### Struct changes (`lib/src/actions/ruby/install.rs`)

Add a `VersionManager` enum and an optional `version_manager` field to `RubyInstall`:

```rust
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionManager {
    Rbenv,
    Chruby,
}

pub struct RubyInstall {
    pub version: String,
    pub implementation: Option<String>,
    pub rubies_dir: Option<String>,
    pub version_manager: Option<VersionManager>,  // new
}
```

`Chruby` is a valid, accepted value that emits no extra steps today. Extensible when chruby post-install needs are identified.

### `plan()` changes

Post-install steps are emitted only when ruby-install runs (i.e., the ruby directory does not yet exist). If the ruby is already installed, `plan()` returns empty as before — idempotency is preserved.

```
ruby dir exists? → return []
otherwise:
  step 1: ruby-install <impl> <version> [--rubies-dir <dir>]
  if version_manager == Rbenv:
    step 2: rbenv global <version>
    step 3: rbenv rehash
```

`version_manager: chruby` and `version_manager` absent both produce one step (ruby-install only).

### Manifest usage

```yaml
- action: ruby.install
  version: "3.3.0"
  version_manager: rbenv
```

```yaml
# chruby — no post-install steps, but value is accepted
- action: ruby.install
  version: "3.3.0"
  version_manager: chruby
```

### Error handling

No special error handling required. The rbenv steps are standard `Exec` atoms — failure propagates via the existing step execution infrastructure.

### Testing

| Test                                          | Assertion                                                                                                       |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `plan_with_rbenv_emits_three_steps`           | ruby dir absent, `version_manager: rbenv` → 3 steps; step 2 = `rbenv global <version>`, step 3 = `rbenv rehash` |
| `plan_with_chruby_emits_one_step`             | `version_manager: chruby` → 1 step (ruby-install only)                                                          |
| `plan_skips_all_if_installed_with_rbenv`      | ruby dir exists, `version_manager: rbenv` → empty (idempotency holds)                                           |
| `it_can_be_deserialized_with_version_manager` | YAML `version_manager: rbenv` deserializes to `VersionManager::Rbenv`                                           |

All existing tests remain unchanged.

### Documentation updates

- `CLAUDE.md` Action Catalog: add `version_manager` to `ruby.install` row
- `README.md` Action Catalog: same
- `examples/ruby/`: add or update example showing `version_manager: rbenv`
