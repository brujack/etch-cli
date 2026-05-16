# brew.bundle Action — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

The dotfiles repo has a 196-line Brewfile covering taps, formulae, casks, and App Store apps. Currently there is no etch action to execute it declaratively. A `brew.bundle` action wrapping `brew bundle install --file=<path>` enables the whole Brewfile to be run from a manifest.

This is the first of four Homebrew-related features planned in sequence:

1. **brew.bundle** (this spec)
2. `cask: true` field on `package.install`
3. `mas.install` action
4. Tap documentation update

## Scope

**Create:** `lib/src/actions/brew/mod.rs` and `lib/src/actions/brew/bundle.rs`

**Modify:** `lib/src/actions/mod.rs` — add `brew` module, `BrewBundle` import, enum variant, `inner_ref`/`Deref`/`Display` arms, round-trip tests

## Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewBundle {
    pub file: String,

    #[serde(default = "get_false")]
    pub no_upgrade: bool,

    #[serde(default = "get_false")]
    pub cleanup: bool,
}
```

`file` is Tera-rendered at manifest load time. `no_upgrade` defaults `false`. `cleanup` defaults `false` — it is destructive (removes packages not in the Brewfile) and must be explicitly opted in.

## YAML Interface

```yaml
- action: brew.bundle
  file: "{{ user.home_dir }}/git-repos/personal/dotfiles/Brewfile"
  # no_upgrade: false   # skip upgrading already-installed packages
  # cleanup: false      # remove packages not in the Brewfile (destructive)
```

## plan() Logic

Returns a single `Exec` atom step. No initializers or finalizers.

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    use crate::atoms::command::Exec;

    let mut args = vec![
        String::from("bundle"),
        String::from("install"),
        format!("--file={}", self.file),
    ];
    if self.no_upgrade {
        args.push(String::from("--no-upgrade"));
    }
    if self.cleanup {
        args.push(String::from("--cleanup"));
    }

    Ok(vec![Step {
        atom: Box::new(Exec {
            command: String::from("brew"),
            arguments: args,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }])
}
```

`brew bundle install` is idempotent by design — it skips already-installed packages.

## summarize()

```rust
fn summarize(&self) -> String {
    format!("Installing Homebrew bundle from {}", self.file)
}
```

## Registration in `lib/src/actions/mod.rs`

New module: `mod brew;` alongside existing `mod file;`, `mod git;`, etc.

Import: `use brew::bundle::BrewBundle;`

Enum variant:

```rust
#[serde(rename = "brew.bundle")]
BrewBundle(ConditionalVariantAction<BrewBundle>),
```

`inner_ref()`, `Deref::deref()`, and `Display::fmt()` arms added: `Actions::BrewBundle(a) => a` / `Actions::BrewBundle(_) => "brew.bundle"`.

## Testing

Four tests in `lib/src/actions/brew/bundle.rs`:

| Test                            | Verifies                                                                                     |
| ------------------------------- | -------------------------------------------------------------------------------------------- |
| `it_can_be_deserialized`        | YAML `brew.bundle` with `file:` → correct struct; `no_upgrade` and `cleanup` default `false` |
| `plan_returns_exec_step`        | Single step; atom Display contains `"brew"` and `"bundle"`; args include `--file=`           |
| `plan_includes_no_upgrade_flag` | `no_upgrade: true` → `--no-upgrade` in arguments                                             |
| `plan_includes_cleanup_flag`    | `cleanup: true` → `--cleanup` in arguments                                                   |

Round-trip tests in `lib/src/actions/mod.rs` updated: add `brew.bundle` entry (with `file:` field) to `all_major_action_variants_can_be_deserialized` and `actions_display_names`.

## What Is NOT in Scope

- `brew bundle check` or `brew bundle dump` sub-commands
- Lock file control (`--no-lock`)
- `brew.upgrade`, `mas.upgrade`, `brew.cleanup` actions (separate backlog items)
- Cask support on `package.install` (next spec in this series)
