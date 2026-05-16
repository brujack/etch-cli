# mas.install Action — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

The dotfiles Brewfile has 15 Mac App Store entries like `mas "Better Rename 9", id: 414209656`. No etch action exists to install App Store apps declaratively. `mas.install` wraps the `mas` CLI tool (`brew install mas` installs it).

This is the third of four Homebrew features (bundle → cask → **mas** → tap docs).

## Scope

**Create:** `lib/src/actions/mas/mod.rs` and `lib/src/actions/mas/install.rs`  
**Modify:** `lib/src/actions/mod.rs` — add `mas` module, `MasInstall` import, enum variant, `inner_ref`/`Deref`/`Display` arms, round-trip tests

## Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasInstall {
    pub name: String,   // human-readable label — used in summarize(), not passed to mas
    pub id: u64,        // App Store numeric ID — what mas install actually uses
}
```

`name` is required (not `Option<String>`) to match the Brewfile format and keep manifests self-documenting. `id` is `u64` — YAML auto-parses numeric values, App Store IDs (9 digits) fit comfortably.

## YAML Interface

```yaml
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
```

Multiple App Store apps = multiple action entries. This matches the Brewfile 1:1 and keeps each app independently controllable with `where:`.

## plan() Logic

Returns a single `Exec` atom step with no initializers or finalizers.

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    use crate::atoms::command::Exec;
    Ok(vec![Step {
        atom: Box::new(Exec {
            command: String::from("mas"),
            arguments: vec![String::from("install"), self.id.to_string()],
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }])
}
```

`mas` handles already-installed apps gracefully (exits 0), so no pre-check or idempotency guard is needed.

## summarize()

```rust
fn summarize(&self) -> String {
    format!("Installing {} from the Mac App Store", self.name)
}
```

## Platform Note

`mas` is macOS-only. The action itself has no platform guard — users apply `where: 'os.name == "macos"'` at the manifest level when needed.

## Registration in `lib/src/actions/mod.rs`

New module: `mod mas;` alongside existing `mod brew;`, `mod git;`, etc.

Import: `use mas::MasInstall;`

Enum variant:

```rust
#[serde(rename = "mas.install")]
MasInstall(ConditionalVariantAction<MasInstall>),
```

`inner_ref()`, `Deref::deref()`, `Display::fmt()` arms: `Actions::MasInstall(a) => a` / `Actions::MasInstall(_) => "mas.install"`.

## Testing

Three tests in `lib/src/actions/mas/install.rs`:

| Test                      | Verifies                                                    |
| ------------------------- | ----------------------------------------------------------- |
| `it_can_be_deserialized`  | YAML `mas.install` with name and id → correct struct fields |
| `plan_returns_exec_step`  | Single step; Exec Display contains `"mas"` and the app ID   |
| `summarize_includes_name` | `summarize()` contains the app name string                  |

Round-trip tests in `lib/src/actions/mod.rs` updated: add `mas.install` entry to `all_major_action_variants_can_be_deserialized` and `actions_display_names`.

## What Is NOT in Scope

- `mas upgrade` (separate backlog item)
- Checking whether an app is already installed (mas handles this gracefully)
- Searching by name (`mas lucky`) — id-based install only
