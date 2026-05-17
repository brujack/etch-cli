# brew.upgrade, brew.cleanup, mas.upgrade — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

Three Homebrew/MAS maintenance actions are missing. All follow the exact same pattern as `brew.bundle` and `mas.install`: new action files in the existing `brew/` and `mas/` modules, single `Exec` atom, registered in the `Actions` enum.

These are the final items from the original brew/mas backlog cluster.

## Scope

**Create:**

- `lib/src/actions/brew/upgrade.rs`
- `lib/src/actions/brew/cleanup.rs`
- `lib/src/actions/mas/upgrade.rs`

**Modify:**

- `lib/src/actions/brew/mod.rs` — re-export `BrewUpgrade`, `BrewCleanup`
- `lib/src/actions/mas/mod.rs` — re-export `MasUpgrade`
- `lib/src/actions/mod.rs` — add `mod brew` already exists; add new imports, enum variants, `inner_ref`/`Deref`/`Display` arms, round-trip tests

## brew.upgrade

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewUpgrade {
    #[serde(default = "get_false")]
    pub greedy: bool,
}
```

`greedy: true` passes `--greedy`, which also upgrades casks that have auto-update enabled. Default false.

`plan()` returns a single `Exec` step: `brew upgrade` + optional `--greedy`.

`summarize()`: `"Upgrading Homebrew packages"` or `"Upgrading Homebrew packages (greedy)"`.

```yaml
- action: brew.upgrade
  greedy: true # optional, default false
```

## brew.cleanup

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewCleanup {
    pub prune: Option<u32>,
}
```

`prune: N` passes `--prune=N` (remove versions older than N days). `None` = no `--prune` flag (brew uses its own default of 120 days).

`plan()` returns a single `Exec` step: `brew cleanup` + optional `--prune=N`.

`summarize()`: `"Cleaning up Homebrew cache"`.

```yaml
- action: brew.cleanup
  prune: 30 # optional; omit to use brew's default (120 days)
```

## mas.upgrade

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasUpgrade {
    pub id: Option<u64>,
}
```

`id: None` → `mas upgrade` (upgrade all installed App Store apps). `id: Some(n)` → `mas upgrade <n>` (upgrade a specific app).

`plan()` returns a single `Exec` step: `mas upgrade` + optional app ID.

`summarize()`: `"Upgrading all App Store apps"` or `"Upgrading App Store app {id}"`.

macOS-only. No platform guard in the action itself — callers use `where: 'os.name == "macos"'`.

```yaml
- action: mas.upgrade # upgrade all
  where: 'os.name == "macos"'

- action: mas.upgrade # upgrade specific app
  id: 414209656
  where: 'os.name == "macos"'
```

## Registration

All three registered in `lib/src/actions/mod.rs`:

```rust
#[serde(rename = "brew.upgrade")]
BrewUpgrade(ConditionalVariantAction<BrewUpgrade>),

#[serde(rename = "brew.cleanup")]
BrewCleanup(ConditionalVariantAction<BrewCleanup>),

#[serde(rename = "mas.upgrade")]
MasUpgrade(ConditionalVariantAction<MasUpgrade>),
```

## Testing

For each action, three tests in the action file:

| Test                          | What it verifies                                |
| ----------------------------- | ----------------------------------------------- |
| `it_can_be_deserialized`      | YAML round-trip → correct struct fields         |
| `plan_returns_exec_step`      | Single step; Exec Display contains command name |
| `plan_includes_flag_when_set` | Optional flag appears in Display when enabled   |

`brew.upgrade`: `greedy: true` → `--greedy` in Display.
`brew.cleanup`: `prune: Some(30)` → `--prune=30` in Display.
`mas.upgrade`: `id: Some(414209656)` → `414209656` in Display.

Round-trip tests in `lib/src/actions/mod.rs` updated to include all three new actions.

## What Is NOT in Scope

- `brew upgrade <formula>` (upgrade a specific formula) — upgrade-all covers the primary use case
- `mas upgrade --all` flag variations — `mas upgrade` with no args already upgrades all
- `brew cleanup -s` (also remove cached downloads) — separate backlog item (`brew.cleanup` already covers the primary use case)
