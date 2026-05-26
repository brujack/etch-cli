# git.config Action Design Spec

**Goal:** Add a `git.config` action that sets or unsets git configuration values at global, local, or system scope, for declarative per-machine gitconfig management in dotfiles manifests.

## Motivation

etch-cli has `git.clone` but no way to manage gitconfig values. Dotfiles require per-machine git settings: `user.name`, `user.email`, credential helpers, proxy settings, conditional includes, etc. Without this action, users fall back to `command.run` with raw `git config` calls.

## Action Interface

Registered as `git.config` (alias `git.cfg`). Two forms, mutually exclusive within a single action:

```yaml
# Single key/value — set
- action: git.config
  scope: global
  key: user.email
  value: bjackson@pobox.com

# Single key — unset
- action: git.config
  scope: global
  key: credential.helper
  unset: true

# Bulk set — map (order preserved)
- action: git.config
  scope: global
  settings:
      user.name: Bruce Jackson
      user.email: bjackson@pobox.com
      core.autocrlf: "false"

# Local scope — requires directory
- action: git.config
  scope: local
  directory: /path/to/repo
  key: user.email
  value: work@company.com

# System scope — auto-privileged, no extra field
- action: git.config
  scope: system
  key: credential.helper
  value: osxkeychain
```

Bulk unset is not supported in `settings` — use multiple single-key `unset: true` actions.

## Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitConfigScope {
    #[default]
    Global,
    Local,
    System,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    pub scope: GitConfigScope,
    pub key: Option<String>,
    pub value: Option<String>,
    pub unset: Option<bool>,
    pub settings: Option<IndexMap<String, String>>,
    pub directory: Option<String>,
}
```

`IndexMap<String, String>` preserves YAML insertion order → deterministic step ordering. `indexmap` is a transitive dep; add it explicitly to `lib/Cargo.toml`.

## Files

- Create: `lib/src/actions/git/config.rs`
- Create: `lib/src/atoms/git/config_unset.rs` (new atom — see Execution)
- Modify: `lib/src/actions/git/mod.rs` (add `mod config; pub use config::GitConfig;`)
- Modify: `lib/src/atoms/git/mod.rs` (add `mod config_unset; pub use config_unset::GitConfigUnset;`)
- Modify: `lib/src/actions/mod.rs` (register `GitConfig` in `Actions` enum as `git.config` / `git.cfg`)
- Modify: `lib/Cargo.toml` (add `indexmap = "2"`)

## Idempotency

**Set operations:** action's `plan()` always emits the Exec step — no read-before-write. `git config` is idempotent by nature (overwriting a key with its current value is a no-op from the system's perspective). This keeps `plan()` pure and testable without spawning subprocesses.

**Unset operations:** `git config --unset key` exits 5 when the key is absent — the Exec atom would treat this as an error. Instead, use the `GitConfigUnset` atom, whose `plan()` runs `git config --get` via `std::process::Command` and returns `should_run: false` if the key is absent.

## Execution

**Set:** existing `Exec` atom. One step per key.

**Scope → set arguments:**

| Scope    | Arguments                                |
| -------- | ---------------------------------------- |
| `global` | `config --global key value`              |
| `local`  | `-C dir config --local key value`        |
| `system` | `config --system key value` (privileged) |

**Unset:** new `GitConfigUnset` atom in `lib/src/atoms/git/config_unset.rs`:

```rust
pub struct GitConfigUnset {
    pub scope_args: Vec<String>,   // e.g. ["--global"] or ["-C", "/repo", "--local"]
    pub key: String,
    pub privileged: bool,
    pub privilege_provider: String,
}
```

`plan()` runs `git [scope_args] config --get key` via `Command::new("git")`:

- exit 0 → key exists → `should_run: true`
- exit 1 → key absent → `should_run: false`

`execute()` runs `git [scope_args] config --unset key` (via `privilege_provider` if `privileged`).

**`system` scope:** `privileged: true` + `privilege_provider` from contexts for both set (Exec) and unset (GitConfigUnset). Same pattern as `file.chmod`.

`settings` map: iterate entries in insertion order, one `Step` per key.

## Validation Errors (returned from `plan()`)

- `key` and `settings` both present → error
- Neither `key` nor `settings` → error
- `unset: true` with `settings` → error
- `unset: true` with `value` → error
- `scope: local` without `directory` → error
- `key` present, `value` absent, `unset` absent/false → error (ambiguous intent)

## Testing

Action tests in `lib/src/actions/git/config.rs` — pure unit tests, no subprocess:

- Deserialization: single key/value, bulk settings map, unset, all three scopes
- `plan()` validation errors: each invalid combination
- `plan()` set single key: 1 step with correct command/args
- `plan()` set bulk: 3-key `settings` → 3 steps with correct args each
- `plan()` unset: 1 `GitConfigUnset` step with correct scope_args and key
- `plan()` local scope: args include `-C dir --local`
- `plan()` system scope: Exec step has `privileged: true`
- `summarize()` includes scope and key(s)

Atom tests in `lib/src/atoms/git/config_unset.rs` — use real `git` binary + `tempfile::tempdir()`:

```rust
fn setup_git_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    Command::new("git").args(["-C", tmp.path().to_str().unwrap(), "init"]).status().unwrap();
    tmp
}
```

- `plan()` returns `should_run: false` when key absent in repo config
- `plan()` returns `should_run: true` when key present in repo config
- `execute()` removes the key; subsequent `git config --get` exits 1
- `display` format includes key name

## Decisions

- **Exec atom, not gix-config:** `git` binary is always present when gitconfig needs managing. `gix-config` write support is newer and doesn't handle `--system` privilege escalation. Exec reuses existing infrastructure.
- **`settings` map, not list of pairs:** YAML maps are the natural shape for key/value config; `IndexMap` preserves order without the verbosity of a list of `{key, value}` objects.
- **Bulk unset not in `settings`:** No sentinel value for "unset this key" in a map without special syntax. Explicit separate actions are clearer.
- **`directory` required for local scope:** `git config --local` requires a git repo; making `directory` explicit surfaces the dependency at plan time rather than failing at execution.
- **`system` auto-privileged:** `system` scope always requires root; no separate `privileged` field since it's never optional for that scope.
- **Set always emits step, unset uses atom:** `git config` set is naturally idempotent (overwrite with same value is silent). No read-before-write keeps action `plan()` pure. Unset needs `GitConfigUnset` atom because `git config --unset` exits 5 on missing key — Exec would surface that as an error.
