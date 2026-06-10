# git.clone update_existing field — Design Spec

## Context

`git.clone` skips when the target directory already exists (`should_run: false`). `git.pull`
handles clone-or-pull, but requires knowing whether the repo already exists or not —
and always runs, meaning it always hits the network. The common dotfiles pattern is
"clone if missing, pull if exists", currently worked around with `command.run` blocks.

A `update_existing: bool` field on `git.clone` collapses both cases into one action.

---

## Scope

One new field on `GitClone` action and Clone atom. No new files, no new action type.

---

## Behavior

| Scenario                                | `update_existing` | Result                                          |
| --------------------------------------- | ----------------- | ----------------------------------------------- |
| Directory missing                       | false (default)   | Clone via gix                                   |
| Directory missing                       | true              | Clone via gix                                   |
| Directory exists, is git repo           | false             | Skip (should_run: false) — unchanged            |
| Directory exists, is git repo           | true              | `git pull` via subprocess                       |
| Directory exists, not git repo          | false             | Skip — unchanged                                |
| Directory exists, not git repo          | true              | `plan()` returns `Err` (early, dry-run visible) |
| `git pull` fails (dirty tree, conflict) | true              | Propagate error from `execute()`                |

Bare repos (no `.git` entry) are not supported. Personal workstation repos are always
standard non-bare; documenting this limitation in the example.

---

## Implementation

### Action — `lib/src/actions/git/clone.rs`

Add one field:

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitClone {
    pub repo_url: String,
    pub directory: String,
    #[serde(default)]
    pub update_existing: bool,
}
```

`plan()` passes `update_existing` to the atom:

```rust
Ok(vec![Step {
    atom: Box::new(crate::atoms::git::Clone {
        repository: url.clone(),
        directory: PathBuf::from(self.directory.clone()),
        update_existing: self.update_existing,
    }),
    initializers: vec![],
    finalizers: vec![],
}])
```

### Atom — `lib/src/atoms/git/clone.rs`

Add field to struct:

```rust
pub struct Clone {
    pub repository: Url,
    pub directory: PathBuf,
    pub update_existing: bool,
}
```

Updated `plan()`:

```rust
fn plan(&self) -> anyhow::Result<Outcome> {
    if self.directory.exists() {
        if self.update_existing {
            if !self.directory.join(".git").exists() {
                anyhow::bail!(
                    "directory {} exists but is not a git repository",
                    self.directory.display()
                );
            }
            return Ok(Outcome { side_effects: vec![], should_run: true });
        }
        return Ok(Outcome { side_effects: vec![], should_run: false });
    }
    Ok(Outcome { side_effects: vec![], should_run: true })
}
```

Updated `execute()` — new early-return branch for the pull path:

```rust
fn execute(&mut self) -> anyhow::Result<()> {
    if self.directory.exists() {
        // update_existing=true; plan() already validated .git exists
        let status = std::process::Command::new("git")
            .args(["-C", &self.directory.to_string_lossy(), "pull"])
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "git -C {} pull failed with {}",
                self.directory.display(),
                status
            );
        }
        return Ok(());
    }
    // existing gix clone path unchanged
    unsafe { interrupt::init_handler(1, || {})? };
    std::fs::create_dir_all(&self.directory)?;
    let mut prepare_clone = gix::prepare_clone(self.repository.clone(), &self.directory)?;
    let (mut prepare_checkout, _) =
        prepare_clone.fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED)?;
    let (repo, _) = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED)?;
    let _ = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("always present after clone")?;
    Ok(())
}
```

---

## Testing

### Atom unit tests (`lib/src/atoms/git/clone.rs`)

Uses PATH-injection mock pattern (same as Pull atom tests). Requires `serial_test`.

| Test                                                      | Validates                                             |
| --------------------------------------------------------- | ----------------------------------------------------- |
| `plan_skips_when_dir_exists_update_existing_false`        | existing dir + false → should_run: false              |
| `plan_runs_when_dir_exists_with_git_update_existing_true` | dir + `.git` + true → should_run: true                |
| `plan_errors_when_dir_exists_no_git_update_existing_true` | dir, no `.git`, true → Err                            |
| `plan_runs_when_dir_missing_update_existing_true`         | missing dir + true → should_run: true                 |
| `execute_pulls_when_dir_exists`                           | mock git, dir+`.git` exist → log contains `-C … pull` |
| `execute_propagates_pull_failure`                         | mock git exits 1 → execute returns Err                |

### Action unit tests (`lib/src/actions/git/clone.rs`)

| Test                                             | Validates                                           |
| ------------------------------------------------ | --------------------------------------------------- |
| `deserialization_with_update_existing_true`      | YAML `update_existing: true` deserializes correctly |
| `deserialization_defaults_update_existing_false` | YAML without field → false                          |

---

## Documentation

### `examples/git/clone.yaml`

Add second entry:

```yaml
# Clone-or-pull: clones if directory is missing, pulls if it already exists.
# Errors if directory exists but is not a git repository (bare repos not supported).
- action: git.clone
  repo_url: https://github.com/brujack/dotfiles
  directory: "{{ user.home_dir }}/git-repos/personal/dotfiles"
  update_existing: true
```

Update existing entry comment: remove "Use git.pull when you need clone-or-pull semantics."

### `README.md`

Update `git.clone` catalog row to note `update_existing` field.
