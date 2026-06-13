# File Rollback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before `file.copy` overwrites an existing file, stash the original; a new `etch rollback` subcommand lists and restores stashes.

**Architecture:** A new `lib/src/rollback/` module owns a `StashStore` (mirrors `lib/src/state/`) that manages `~/.local/share/etch/backups/<path-hex>/<timestamp>` files. A new `Stash` file atom is prepended to `file.copy` steps so the original is captured before any write. A new `etch rollback` command lists and restores stashes.

**Tech Stack:** Rust, `sha256` crate (already present), `chrono` crate (already present), `similar = "2"` (new — unified diff), `serde_yaml_ng` (already present), `dirs-next` (already present in lib + app).

---

## File Map

| File                           | Status     | Purpose                                                                  |
| ------------------------------ | ---------- | ------------------------------------------------------------------------ |
| `lib/src/rollback/mod.rs`      | **Create** | `StashMeta`, `StashEntry`, `StashStore` — mirrors `lib/src/state/mod.rs` |
| `lib/src/atoms/file/stash.rs`  | **Create** | `Stash` atom — stashes a file before overwrite                           |
| `lib/src/atoms/file/mod.rs`    | **Modify** | Add `mod stash; pub use stash::Stash;`                                   |
| `lib/src/actions/file/copy.rs` | **Modify** | Prepend `Stash` step in non-privileged and privileged paths              |
| `lib/src/lib.rs`               | **Modify** | Add `pub mod rollback;`                                                  |
| `lib/Cargo.toml`               | **Modify** | Add `similar = "2"`                                                      |
| `app/src/commands/rollback.rs` | **Create** | `Rollback` clap struct + `EtchCommand` impl (tarpaulin-bifurcated)       |
| `app/src/commands/mod.rs`      | **Modify** | Add `mod rollback; pub(crate) use rollback::Rollback;`                   |
| `app/src/config/mod.rs`        | **Modify** | Add `Rollback(commands::Rollback)` to `Commands` enum                    |
| `app/src/main.rs`              | **Modify** | Add `Commands::Rollback(r) => r.execute(&runtime)`                       |
| `app/tests/rollback.rs`        | **Create** | Integration tests using `ETCH_STASH_DIR` env var                         |

---

## Task 1: Dependencies + rollback module scaffold

**Files:**

- Modify: `lib/Cargo.toml`
- Create: `lib/src/rollback/mod.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add `similar` dependency**

In `lib/Cargo.toml`, add after the `sha256` line:

```toml
similar = "2"
```

- [ ] **Step 2: Create rollback module scaffold**

Create `lib/src/rollback/mod.rs`:

```rust
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StashMeta {
    pub original_path: String,
    pub stashed_at: DateTime<Utc>,
    pub apply_manifest: String,
    pub sha256: String,
}

#[derive(Debug)]
pub struct StashEntry {
    pub original_path: PathBuf,
    pub stashed_at: DateTime<Utc>,
    pub apply_manifest: String,
    pub sha256: String,
    pub stash_path: PathBuf,
    pub meta_path: PathBuf,
}

pub struct StashStore {
    base: PathBuf,
}

impl Default for StashStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StashStore {
    /// Resolves `dirs_next::data_local_dir()/etch/backups/`.
    /// Override with `ETCH_STASH_DIR` env var (integration tests).
    pub fn new() -> Self {
        let base = if let Ok(dir) = std::env::var("ETCH_STASH_DIR") {
            PathBuf::from(dir)
        } else {
            dirs_next::data_local_dir()
                .unwrap_or_else(|| {
                    dirs_next::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".local")
                        .join("share")
                })
                .join("etch")
                .join("backups")
        };
        Self { base }
    }

    /// For unit tests — inject arbitrary base dir.
    pub fn with_base(base: PathBuf) -> Self {
        Self { base }
    }

    pub fn stash(&self, _path: &Path, _manifest: &str, _keep: usize) -> anyhow::Result<bool> {
        todo!()
    }

    pub fn list(&self) -> anyhow::Result<Vec<(PathBuf, Vec<StashEntry>)>> {
        todo!()
    }

    pub fn restore(&self, _path: &Path, _dry_run: bool) -> anyhow::Result<()> {
        todo!()
    }

    pub fn prune(&self, _path: &Path, _keep: usize) -> anyhow::Result<()> {
        todo!()
    }
}
```

- [ ] **Step 3: Register module in lib.rs**

In `lib/src/lib.rs`, add after the last `pub mod` line:

```rust
pub mod rollback;
```

- [ ] **Step 4: Verify compilation**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo check -p etch-lib 2>&1 | tail -5
```

Expected: compiles (todo!() panics are allowed).

---

## Task 2: StashStore::stash() + prune() with tests

**Files:**

- Modify: `lib/src/rollback/mod.rs`

- [ ] **Step 1: Write failing tests for stash() and prune()**

Add to the bottom of `lib/src/rollback/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_store() -> (StashStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = StashStore::with_base(dir.path().to_path_buf());
        (store, dir)
    }

    #[test]
    fn stash_creates_hex_dir_stash_file_and_meta() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("config.txt");
        std::fs::write(&path, "original").unwrap();

        let result = store.stash(&path, "core.yaml", 3).unwrap();
        assert!(result, "stash() should return true for regular file");

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        assert!(hex_dir.is_dir(), "hex dir must exist");

        let stash_files: Vec<_> = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .collect();
        assert_eq!(stash_files.len(), 1, "one stash file");

        let meta_files: Vec<_> = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .collect();
        assert_eq!(meta_files.len(), 1, "one meta file");

        let stash_content = std::fs::read_to_string(stash_files[0].path()).unwrap();
        assert_eq!(stash_content, "original");
    }

    #[test]
    fn stash_returns_false_for_missing_path() {
        let (store, _dir) = make_store();
        let result = store.stash(std::path::Path::new("/nonexistent/path"), "m", 3).unwrap();
        assert!(!result);
    }

    #[test]
    fn stash_returns_false_for_directory() {
        let (store, _dir) = make_store();
        let dir = tempdir().unwrap();
        let result = store.stash(dir.path(), "m", 3).unwrap();
        assert!(!result);
    }

    #[test]
    fn stash_meta_yaml_roundtrip() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "data").unwrap();

        store.stash(&path, "my-manifest.yaml", 3).unwrap();

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let meta_path: std::path::PathBuf = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .unwrap()
            .path();

        let content = std::fs::read_to_string(&meta_path).unwrap();
        let meta: StashMeta = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(meta.original_path, path.to_string_lossy().as_ref());
        assert_eq!(meta.apply_manifest, "my-manifest.yaml");
        assert!(!meta.sha256.is_empty());
        // stashed_at is recent
        let age = chrono::Utc::now() - meta.stashed_at;
        assert!(age.num_seconds() < 5);
    }

    #[test]
    fn prune_keeps_n_newest_deletes_oldest() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");

        // Create 4 stashes
        for i in 0..4u8 {
            std::fs::write(&path, [i]).unwrap();
            store.stash(&path, "m", 10).unwrap(); // keep=10 so prune doesn't auto-trim
            std::thread::sleep(std::time::Duration::from_millis(1100)); // ensure distinct timestamps
        }

        // Now prune to keep=2
        store.prune(&path, 2).unwrap();

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let stash_files: Vec<_> = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .collect();
        assert_eq!(stash_files.len(), 2, "keep=2 leaves 2 stashes");

        let meta_files: Vec<_> = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .collect();
        assert_eq!(meta_files.len(), 2, "keep=2 leaves 2 sidecars");
    }

    #[test]
    fn prune_noop_when_count_le_keep() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");

        for _ in 0..2 {
            std::fs::write(&path, "x").unwrap();
            store.stash(&path, "m", 10).unwrap();
        }

        store.prune(&path, 3).unwrap(); // keep=3, only 2 exist

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let count = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .count();
        assert_eq!(count, 2, "no deletion when count <= keep");
    }

    #[test]
    fn new_respects_etch_stash_dir_env() {
        let dir = tempdir().unwrap();
        let old = std::env::var("ETCH_STASH_DIR").ok();
        std::env::set_var("ETCH_STASH_DIR", dir.path());
        let store = StashStore::new();
        if let Some(v) = old {
            std::env::set_var("ETCH_STASH_DIR", v);
        } else {
            std::env::remove_var("ETCH_STASH_DIR");
        }
        assert_eq!(store.base, dir.path());
    }
}
```

- [ ] **Step 2: Run tests — confirm all fail**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib rollback 2>&1 | tail -20
```

Expected: panics at `todo!()`.

- [ ] **Step 3: Implement stash() and prune()**

Replace the `stash()` and `prune()` `todo!()` stubs in `lib/src/rollback/mod.rs`:

```rust
pub fn stash(&self, path: &Path, manifest: &str, keep: usize) -> anyhow::Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }

    let hex = sha256::digest(path.to_string_lossy().as_ref());
    let hex_dir = self.base.join(&hex);
    std::fs::create_dir_all(&hex_dir)
        .with_context(|| format!("rollback: cannot create backup dir {:?}", hex_dir))?;

    let now = Utc::now();
    let ts = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let stash_path = hex_dir.join(&ts);
    let meta_path = hex_dir.join(format!("{}.meta.yaml", ts));

    let content = std::fs::read(path)
        .with_context(|| format!("rollback: cannot read {:?}", path))?;
    let content_hash = sha256::digest(content.as_slice());

    std::fs::write(&stash_path, &content)
        .with_context(|| format!("rollback: cannot write stash {:?}", stash_path))?;

    let meta = StashMeta {
        original_path: path.to_string_lossy().into_owned(),
        stashed_at: now,
        apply_manifest: manifest.to_string(),
        sha256: content_hash,
    };
    let meta_yaml = serde_yaml_ng::to_string(&meta).context("rollback: serialize meta")?;
    std::fs::write(&meta_path, meta_yaml)
        .with_context(|| format!("rollback: cannot write meta {:?}", meta_path))?;

    self.prune(path, keep)?;

    Ok(true)
}

pub fn prune(&self, path: &Path, keep: usize) -> anyhow::Result<()> {
    let hex = sha256::digest(path.to_string_lossy().as_ref());
    let hex_dir = self.base.join(&hex);
    if !hex_dir.is_dir() {
        return Ok(());
    }

    let mut names: Vec<String> = std::fs::read_dir(&hex_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".meta.yaml") {
                None
            } else {
                Some(name)
            }
        })
        .collect();

    names.sort(); // RFC 3339 timestamps sort lexicographically = chronologically

    if names.len() <= keep {
        return Ok(());
    }

    let to_delete: Vec<String> = names[..names.len() - keep].to_vec();
    for name in to_delete {
        let _ = std::fs::remove_file(hex_dir.join(&name));
        let _ = std::fs::remove_file(hex_dir.join(format!("{}.meta.yaml", name)));
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib rollback 2>&1 | tail -20
```

Expected: all `stash_*` and `prune_*` tests pass. `list` and `restore` tests don't exist yet.

**Note:** The `prune_keeps_n_newest_deletes_oldest` test uses `sleep(1100ms)` to guarantee distinct second-resolution timestamps. This is expected and intentional.

- [ ] **Step 5: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add lib/Cargo.toml lib/src/lib.rs lib/src/rollback/mod.rs
git commit -m "feat(rollback): add StashStore::stash() and prune() with tests"
```

---

## Task 3: StashStore::list() + restore() with tests

**Files:**

- Modify: `lib/src/rollback/mod.rs`

- [ ] **Step 1: Write failing tests for list() and restore()**

Add to the `#[cfg(test)]` block in `lib/src/rollback/mod.rs`:

```rust
    #[test]
    fn list_returns_empty_when_base_missing() {
        let dir = tempdir().unwrap();
        let store = StashStore::with_base(dir.path().join("nonexistent"));
        let result = store.list().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_groups_by_path_newest_first() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");

        std::fs::write(&path, "v1").unwrap();
        store.stash(&path, "m", 10).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&path, "v2").unwrap();
        store.stash(&path, "m", 10).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1, "one path");
        let (listed_path, entries) = &list[0];
        assert_eq!(listed_path, &path);
        assert_eq!(entries.len(), 2);
        // newest first
        assert!(entries[0].stashed_at > entries[1].stashed_at);
    }

    #[test]
    fn restore_dry_run_prints_diff_no_write() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "original").unwrap();
        store.stash(&path, "m", 3).unwrap();

        // Overwrite the file so current != stash
        std::fs::write(&path, "modified").unwrap();

        // dry_run=true: should not panic and should not revert the file
        store.restore(&path, true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "modified", "dry_run must not write");
    }

    #[test]
    fn restore_writes_stash_content_to_original_path() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "original").unwrap();
        store.stash(&path, "m", 3).unwrap();
        std::fs::write(&path, "modified").unwrap();

        store.restore(&path, false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn restore_preserves_stash_file_after_restore() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "original").unwrap();
        store.stash(&path, "m", 3).unwrap();
        std::fs::write(&path, "modified").unwrap();

        store.restore(&path, false).unwrap();

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let count = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .count();
        assert_eq!(count, 1, "stash file must be preserved after restore");
    }

    #[test]
    fn restore_errors_when_no_stash_for_path() {
        let (store, _dir) = make_store();
        let result = store.restore(std::path::Path::new("/no/stash/here"), false);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no stash found"), "expected 'no stash found', got: {msg}");
    }
```

- [ ] **Step 2: Run tests — confirm new tests fail**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib rollback 2>&1 | tail -20
```

Expected: new tests panic at `todo!()`.

- [ ] **Step 3: Implement list() and restore()**

Replace the `list()` and `restore()` `todo!()` stubs in `lib/src/rollback/mod.rs`:

```rust
pub fn list(&self) -> anyhow::Result<Vec<(PathBuf, Vec<StashEntry>)>> {
    if !self.base.is_dir() {
        return Ok(vec![]);
    }

    let mut result: Vec<(PathBuf, Vec<StashEntry>)> = Vec::new();

    for hex_entry in std::fs::read_dir(&self.base)? {
        let hex_dir = hex_entry?.path();
        if !hex_dir.is_dir() {
            continue;
        }

        let mut entries: Vec<StashEntry> = Vec::new();

        for meta_entry in std::fs::read_dir(&hex_dir)? {
            let meta_path = meta_entry?.path();
            let name = meta_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            if !name.ends_with(".meta.yaml") {
                continue;
            }

            let content = match std::fs::read_to_string(&meta_path) {
                Ok(c) => c,
                Err(_) => {
                    tracing::warn!("rollback: cannot read meta {:?}, skipping", meta_path);
                    continue;
                }
            };
            let meta: StashMeta = match serde_yaml_ng::from_str(&content) {
                Ok(m) => m,
                Err(_) => {
                    tracing::warn!("rollback: corrupt meta {:?}, skipping", meta_path);
                    continue;
                }
            };

            let stash_name = name.trim_end_matches(".meta.yaml").to_string();
            let stash_path = hex_dir.join(&stash_name);
            if !stash_path.exists() {
                tracing::warn!("rollback: stash missing for meta {:?}, skipping", meta_path);
                continue;
            }

            entries.push(StashEntry {
                original_path: PathBuf::from(&meta.original_path),
                stashed_at: meta.stashed_at,
                apply_manifest: meta.apply_manifest,
                sha256: meta.sha256,
                stash_path,
                meta_path,
            });
        }

        if entries.is_empty() {
            continue;
        }
        entries.sort_by(|a, b| b.stashed_at.cmp(&a.stashed_at));
        let original_path = entries[0].original_path.clone();
        result.push((original_path, entries));
    }

    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

pub fn restore(&self, path: &Path, dry_run: bool) -> anyhow::Result<()> {
    let hex = sha256::digest(path.to_string_lossy().as_ref());
    let hex_dir = self.base.join(&hex);

    if !hex_dir.is_dir() {
        anyhow::bail!("no stash found for {}", path.display());
    }

    let mut names: Vec<String> = std::fs::read_dir(&hex_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".meta.yaml") { None } else { Some(name) }
        })
        .collect();
    names.sort();

    let latest = names.last()
        .ok_or_else(|| anyhow::anyhow!("no stash found for {}", path.display()))?
        .clone();
    let stash_path = hex_dir.join(&latest);
    let stash_content = std::fs::read_to_string(&stash_path)
        .with_context(|| format!("rollback: cannot read stash {:?}", stash_path))?;

    if dry_run {
        let current = if path.exists() {
            std::fs::read_to_string(path)
                .with_context(|| format!("rollback: cannot read current {:?}", path))?
        } else {
            String::new()
        };
        use similar::{ChangeTag, TextDiff};
        let diff = TextDiff::from_lines(current.as_str(), stash_content.as_str());
        println!("--- current ({})", path.display());
        println!("+++ stash ({})", latest);
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            print!("{}{}", sign, change);
        }
        return Ok(());
    }

    println!("Restoring {} from stash {}", path.display(), latest);
    std::fs::write(path, stash_content.as_bytes())
        .with_context(|| format!("rollback: cannot restore to {:?}", path))?;
    println!("Done.");

    Ok(())
}
```

- [ ] **Step 4: Run all rollback tests**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib rollback 2>&1 | tail -20
```

Expected: all rollback unit tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add lib/src/rollback/mod.rs
git commit -m "feat(rollback): add StashStore::list() and restore() with tests"
```

---

## Task 4: Stash atom + wire into file.copy action

**Files:**

- Create: `lib/src/atoms/file/stash.rs`
- Modify: `lib/src/atoms/file/mod.rs`
- Modify: `lib/src/actions/file/copy.rs`

- [ ] **Step 1: Write failing tests for Stash atom**

Create `lib/src/atoms/file/stash.rs` (tests first, implementation later):

```rust
use crate::atoms::{Atom, AtomStatus, Outcome};
use crate::rollback::StashStore;
use std::path::PathBuf;

pub struct Stash {
    pub path: PathBuf,
    pub manifest: String,
    pub keep: usize,
}

impl std::fmt::Display for Stash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stash {} before overwrite", self.path.display())
    }
}

impl Atom for Stash {
    fn plan(&self) -> anyhow::Result<Outcome> {
        todo!()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollback::StashStore;
    use tempfile::tempdir;

    #[test]
    fn plan_should_run_true_for_regular_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "content").unwrap();
        let atom = Stash { path, manifest: "m".into(), keep: 3 };
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn plan_should_run_false_for_missing_path() {
        let atom = Stash {
            path: std::path::PathBuf::from("/no/such/file.txt"),
            manifest: "m".into(),
            keep: 3,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_false_for_directory() {
        let dir = tempdir().unwrap();
        let atom = Stash { path: dir.path().to_path_buf(), manifest: "m".into(), keep: 3 };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_returns_ok_on_success() {
        let stash_dir = tempdir().unwrap();
        let src_dir = tempdir().unwrap();
        let path = src_dir.path().join("file.txt");
        std::fs::write(&path, "data").unwrap();

        // Use ETCH_STASH_DIR to redirect stash into our temp dir
        let old = std::env::var("ETCH_STASH_DIR").ok();
        std::env::set_var("ETCH_STASH_DIR", stash_dir.path());

        let mut atom = Stash { path, manifest: "m".into(), keep: 3 };
        let result = atom.execute();

        if let Some(v) = old { std::env::set_var("ETCH_STASH_DIR", v); }
        else { std::env::remove_var("ETCH_STASH_DIR"); }

        assert!(result.is_ok());
    }

    #[test]
    fn execute_returns_ok_even_when_stash_fails() {
        // Path that is a directory — stash returns Ok(false), not Err
        let dir = tempdir().unwrap();
        let mut atom = Stash { path: dir.path().to_path_buf(), manifest: "m".into(), keep: 3 };
        assert!(atom.execute().is_ok(), "execute() must never propagate stash failure");
    }
}
```

- [ ] **Step 2: Register the module**

In `lib/src/atoms/file/mod.rs`, add alongside the other private mods:

```rust
mod stash;
pub use stash::Stash;
```

- [ ] **Step 3: Run tests — confirm they fail**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib 'atoms::file::stash' 2>&1 | tail -20
```

Expected: panics at `todo!()`.

- [ ] **Step 4: Implement Stash atom**

Replace the `plan()` and `execute()` `todo!()` stubs in `lib/src/atoms/file/stash.rs`:

```rust
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: self.path.is_file(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        match StashStore::new().stash(&self.path, &self.manifest, self.keep) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    "rollback: stash failed for {}: {e:#}",
                    self.path.display()
                );
            }
        }
        Ok(())
    }
```

Also add the default `status()` implementation (inherits `AtomStatus::Unchecked` from the trait default — no override needed).

- [ ] **Step 5: Run Stash atom tests — confirm they pass**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib 'atoms::file::stash' 2>&1 | tail -20
```

Expected: all stash atom tests pass.

- [ ] **Step 6: Write tests for file.copy action step count with stash**

Add to the existing test module in `lib/src/actions/file/copy.rs`. The non-privileged path currently returns 4 steps (DirCreate + Create + Chmod + SetContents). With Stash prepended it becomes 5:

```rust
    #[test]
    fn plan_prepends_stash_step_when_dest_exists() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("files");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("source.txt"), b"new content").unwrap();

        // Pre-create dest so Stash.plan() returns should_run=true
        let dest = tmp.path().join("dest.txt");
        std::fs::write(&dest, b"original content").unwrap();

        let action = FileCopy {
            from: "source.txt".to_string(),
            to: dest.display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        // Stash + DirCreate + Create + Chmod + SetContents = 5 steps
        assert_eq!(5, steps.len(), "stash step must be prepended when dest exists");
        assert!(steps[0].atom.to_string().contains("Stash"), "first step must be Stash");
    }

    #[test]
    fn plan_has_stash_step_even_when_dest_absent() {
        // Stash atom is always in the steps vec; its own plan() returns
        // should_run=false for missing paths (tested in atoms::file::stash).
        // The action always prepends the step — the atom handles the skip.
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("files");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("source.txt"), b"content").unwrap();

        let action = FileCopy {
            from: "source.txt".to_string(),
            to: tmp.path().join("new_dest.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        // Stash + DirCreate + Create + Chmod + SetContents = 5 steps
        assert_eq!(5, steps.len(), "stash step always present in non-privileged path");
        assert!(steps[0].atom.to_string().contains("Stash"));
    }
```

Update the two existing tests that assert `4` steps to now assert `5` steps:

- `plan_returns_steps_for_valid_source` — change `assert_eq!(4, steps.len())` to `assert_eq!(5, steps.len())`
- `plan_template_rendering` — same change
- `plan_with_passphrase_uses_decrypt_step` — same change
- `plan_with_to_as_directory` — same change

For privileged-path tests, update these to assert 1 extra Stash step:

- `plan_returns_exec_steps_when_privileged_no_template` — change `assert_eq!(5, steps.len())` to `assert_eq!(6, steps.len())`
- `plan_returns_setcontents_then_exec_when_privileged_template` — same change
- `plan_privileged_with_passphrase_uses_decrypt_step` — same change
- `plan_privileged_with_owner_adds_chown_step` — change `assert_eq!(6, steps.len())` to `assert_eq!(7, steps.len())`

- [ ] **Step 7: Run copy action tests — confirm they fail**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib 'actions::file::copy' 2>&1 | tail -20
```

Expected: step-count assertions fail (still 4/5 without Stash).

- [ ] **Step 8: Wire Stash into file.copy action**

In `lib/src/actions/file/copy.rs`, in the non-privileged `plan()` path:

Find the line `let parent = path.clone();` that begins building the non-privileged steps vec. Before it, insert the Stash step at the beginning. The steps vec is built as a `let mut steps = vec![...]`. Refactor to:

```rust
let mut steps = vec![
    Step {
        atom: Box::new(crate::atoms::file::Stash {
            path: path.clone(),
            manifest: manifest.name.clone().unwrap_or_default(),
            keep: 3,
        }),
        initializers: vec![],
        finalizers: vec![],
    },
    Step {
        atom: Box::new(DirCreate {
            path: parent
                .parent()
                .ok_or_else(|| {
                    anyhow!("Failed to get parent directory for FileCopy action")
                })?
                .into(),
        }),
        initializers: vec![],
        finalizers: vec![],
    },
    Step {
        atom: Box::new(Create { path: path.clone() }),
        initializers: vec![],
        finalizers: vec![],
    },
    Step {
        atom: Box::new(Chmod {
            path: path.clone(),
            mode: self.chmod,
        }),
        initializers: vec![],
        finalizers: vec![],
    },
];
```

For the privileged path, find where `let mut steps: Vec<crate::steps::Step> = vec![];` is initialized, then add the Stash step as the first push (before the SetContents/Decrypt push):

```rust
let mut steps: Vec<crate::steps::Step> = vec![];

// Stash original before privileged overwrite (best-effort)
steps.push(crate::steps::Step {
    atom: Box::new(crate::atoms::file::Stash {
        path: path.clone(),
        manifest: manifest.name.clone().unwrap_or_default(),
        keep: 3,
    }),
    initializers: vec![],
    finalizers: vec![],
});

// Write content to tempfile (non-privileged)
if let Some(passphrase) = self.passphrase.clone() {
    // ... rest of existing code
```

- [ ] **Step 9: Run all etch-lib tests**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-lib 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 10: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add lib/src/atoms/file/stash.rs lib/src/atoms/file/mod.rs lib/src/actions/file/copy.rs
git commit -m "feat(rollback): add Stash atom and wire into file.copy plan"
```

---

## Task 5: Rollback command with testable logic

**Files:**

- Create: `app/src/commands/rollback.rs`

- [ ] **Step 1: Write failing tests for rollback logic**

Create `app/src/commands/rollback.rs`:

```rust
use super::EtchCommand;
use crate::Runtime;
use etch_lib::rollback::{StashEntry, StashStore};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub(crate) struct Rollback {
    /// Restore latest stash for this path
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// List all stashed paths with timestamps (default behavior)
    #[arg(long)]
    pub list: bool,
    /// Show diff of what would be restored; no write (requires --path)
    #[arg(long)]
    pub dry_run: bool,
    /// Restore all paths to their latest stash
    #[arg(long)]
    pub all: bool,
    /// Skip confirmation prompt for --all
    #[arg(long)]
    pub yes: bool,
}

/// Validate mutually exclusive / co-required arg combos.
pub(crate) fn validate_args(args: &Rollback) -> anyhow::Result<()> {
    if args.all && args.dry_run {
        anyhow::bail!("--dry-run cannot be combined with --all");
    }
    Ok(())
}

/// Format the stash table as a String (extracted for unit testing).
pub(crate) fn render_list(entries: &[(PathBuf, Vec<StashEntry>)]) -> String {
    use std::fmt::Write as FmtWrite;
    let mut buf = String::new();
    writeln!(buf, "{:<50} {:<8} LATEST", "PATH", "STASHES").unwrap();
    for (path, stashes) in entries {
        let latest = &stashes[0]; // sorted newest-first by StashStore::list()
        writeln!(
            buf,
            "{:<50} {:<8} {}",
            path.display(),
            stashes.len(),
            latest.stashed_at.format("%Y-%m-%d %H:%M:%S UTC"),
        )
        .unwrap();
    }
    buf
}

impl EtchCommand for Rollback {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        use std::io::IsTerminal;

        if let Err(e) = validate_args(self) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }

        let store = StashStore::new();

        // --path: restore or diff
        if let Some(raw_path) = &self.path {
            let path = expand_tilde(raw_path);
            if let Err(e) = store.restore(&path, self.dry_run) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            return Ok(());
        }

        // --all: restore everything
        if self.all {
            if !std::io::stdin().is_terminal() && !self.yes {
                eprintln!("error: --all requires confirmation; pass --yes to skip prompt");
                std::process::exit(1);
            }
            let list = store.list().map_err(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            }).unwrap();

            if list.is_empty() {
                println!("No stashes found.");
                return Ok(());
            }

            println!("Will restore {} path(s) to their latest stash:", list.len());
            for (path, stashes) in &list {
                println!("  {} → stash {}", path.display(), stashes[0].stashed_at.format("%Y-%m-%dT%H:%M:%SZ"));
            }

            if !self.yes {
                use std::io::Write;
                print!("Continue? [y/N] ");
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            for (path, _) in &list {
                if let Err(e) = store.restore(path, false) {
                    eprintln!("error restoring {}: {e}", path.display());
                }
            }
            return Ok(());
        }

        // Default: list
        let list = store.list()?;
        print!("{}", render_list(&list));

        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}

fn expand_tilde(path: &PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(rest);
        }
    }
    path.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use etch_lib::rollback::{StashEntry, StashMeta};

    fn make_entry(path: &str, ts: chrono::DateTime<chrono::Utc>) -> StashEntry {
        StashEntry {
            original_path: PathBuf::from(path),
            stashed_at: ts,
            apply_manifest: "m.yaml".into(),
            sha256: "abc".into(),
            stash_path: PathBuf::from("/tmp/s"),
            meta_path: PathBuf::from("/tmp/s.meta.yaml"),
        }
    }

    #[test]
    fn validate_all_dry_run_errors() {
        let args = Rollback {
            path: None, list: false, dry_run: true, all: true, yes: false,
        };
        assert!(validate_args(&args).is_err());
    }

    #[test]
    fn validate_path_dry_run_ok() {
        let args = Rollback {
            path: Some(PathBuf::from("/tmp/f")), list: false, dry_run: true,
            all: false, yes: false,
        };
        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn validate_no_args_ok() {
        let args = Rollback {
            path: None, list: false, dry_run: false, all: false, yes: false,
        };
        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn render_list_empty_shows_header_only() {
        let output = render_list(&[]);
        assert!(output.contains("PATH"), "header must be present");
        assert!(output.contains("STASHES"));
        assert!(output.contains("LATEST"));
    }

    #[test]
    fn render_list_shows_path_and_count() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(100);
        let entries = vec![(
            PathBuf::from("/home/bruce/.zshrc"),
            vec![make_entry("/home/bruce/.zshrc", now), make_entry("/home/bruce/.zshrc", earlier)],
        )];
        let output = render_list(&entries);
        assert!(output.contains(".zshrc"), "path must appear");
        assert!(output.contains("2"), "stash count must appear");
    }

    #[test]
    fn expand_tilde_replaces_prefix() {
        let home = dirs_next::home_dir().unwrap();
        let path = PathBuf::from("~/.zshrc");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home.join(".zshrc"));
    }

    #[test]
    fn expand_tilde_noop_for_absolute() {
        let path = PathBuf::from("/etc/hosts");
        assert_eq!(expand_tilde(&path), path);
    }
}
```

- [ ] **Step 2: Run tests — confirm they compile and pass**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-cli 'commands::rollback' 2>&1 | tail -20
```

Expected: all 7 unit tests pass. (The `execute()` tests cannot run via unit test due to tarpaulin bifurcation — they are covered by integration tests in Task 7.)

- [ ] **Step 3: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add app/src/commands/rollback.rs
git commit -m "feat(rollback): add Rollback command with testable logic"
```

---

## Task 6: Wire rollback command into app

**Files:**

- Modify: `app/src/commands/mod.rs`
- Modify: `app/src/config/mod.rs`
- Modify: `app/src/main.rs`

- [ ] **Step 1: Add module to commands/mod.rs**

In `app/src/commands/mod.rs`, add:

```rust
mod rollback;
pub(crate) use rollback::Rollback;
```

- [ ] **Step 2: Add variant to Commands enum in config/mod.rs**

In `app/src/config/mod.rs`, add to the `Commands` enum:

```rust
/// List and restore pre-apply file stashes
Rollback(commands::Rollback),
```

- [ ] **Step 3: Add match arm to main.rs execute()**

In `app/src/main.rs`, add to the `match &runtime.args.command` block:

```rust
Commands::Rollback(r) => r.execute(&runtime),
```

- [ ] **Step 4: Build and smoke test**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo build -p etch-cli 2>&1 | tail -10
./target/debug/etch rollback --help
```

Expected: binary builds; `--help` shows `--path`, `--list`, `--dry-run`, `--all`, `--yes` flags.

- [ ] **Step 5: Run full test suite**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-cli 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add app/src/commands/mod.rs app/src/config/mod.rs app/src/main.rs
git commit -m "feat(rollback): wire Rollback command into CLI"
```

---

## Task 7: Integration tests

**Files:**

- Create: `app/tests/rollback.rs`

- [ ] **Step 1: Write integration tests**

Create `app/tests/rollback.rs`:

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn etch(stash_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("etch"));
    cmd.env("ETCH_STASH_DIR", stash_dir);
    cmd.env("ETCH_STATE_DIR", stash_dir); // avoid polluting real state
    cmd
}

/// Write a file.copy manifest and return the manifest dir.
fn setup_copy_manifest(
    manifest_dir: &std::path::Path,
    source_content: &str,
    target: &std::path::Path,
) {
    let files_dir = manifest_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), source_content).unwrap();
    fs::write(
        manifest_dir.join("main.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            target.display()
        ),
    )
    .unwrap();
}

#[test]
fn apply_file_copy_stashes_pre_existing_file() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original content").unwrap();
    setup_copy_manifest(manifest_dir.path(), "new content", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    let hex = sha256::digest(target.to_string_lossy().as_ref());
    let hex_dir = stash_dir.path().join(&hex);
    assert!(hex_dir.is_dir(), "hex dir must exist after stash");

    let stash_files: Vec<_> = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .collect();
    assert_eq!(stash_files.len(), 1, "exactly one stash file");

    let saved = fs::read_to_string(stash_files[0].path()).unwrap();
    assert_eq!(saved, "original content", "stash must contain pre-apply content");
}

#[test]
fn apply_twice_creates_two_stashes() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original").unwrap();
    setup_copy_manifest(manifest_dir.path(), "new", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    // Mutate source so the second apply sees a changed file
    let files_dir = manifest_dir.path().join("files");
    fs::write(files_dir.join("source.txt"), "newer").unwrap();

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    let hex = sha256::digest(target.to_string_lossy().as_ref());
    let hex_dir = stash_dir.path().join(&hex);
    let count = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .count();
    assert_eq!(count, 2, "two applies must produce two stashes");
}

#[test]
fn rollback_path_restores_original_content() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original").unwrap();
    setup_copy_manifest(manifest_dir.path(), "replaced", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&target).unwrap(), "replaced");

    // Mutate after apply to simulate drift
    fs::write(&target, "mutated").unwrap();

    etch(stash_dir.path())
        .args(["--no-color", "rollback", "--path", &target.display().to_string()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "original",
        "rollback must restore pre-apply content"
    );
}

#[test]
fn prune_limits_stash_count_to_three() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "v0").unwrap();
    let files_dir = manifest_dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();

    for i in 1..=4u8 {
        fs::write(files_dir.join("source.txt"), format!("v{}", i)).unwrap();
        fs::write(
            manifest_dir.path().join("main.yaml"),
            format!(
                "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
                target.display()
            ),
        )
        .unwrap();
        etch(stash_dir.path())
            .current_dir(manifest_dir.path())
            .args(["--no-color", "-d", ".", "apply"])
            .assert()
            .success();
        // Brief pause to ensure distinct timestamps (1s resolution)
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    let hex = sha256::digest(target.to_string_lossy().as_ref());
    let hex_dir = stash_dir.path().join(&hex);
    let count = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .count();
    assert_eq!(count, 3, "keep=3 default: 4 applies must leave 3 stashes");
}

#[test]
fn file_link_creates_no_stash() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    let files_dir = manifest_dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "link source").unwrap();
    let target = target_dir.path().join("linked.txt");
    let source_abs = files_dir.join("source.txt").canonicalize().unwrap();

    fs::write(
        manifest_dir.path().join("main.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    // Stash dir should not exist — file.link never stashes
    assert!(
        !stash_dir.path().read_dir().unwrap().any(|_| true),
        "file.link must not create any stash"
    );
}

#[test]
fn rollback_list_exits_zero_with_no_stashes() {
    let stash_dir = tempdir().unwrap();

    etch(stash_dir.path())
        .args(["--no-color", "rollback"])
        .assert()
        .success();
}

#[test]
fn rollback_path_unknown_exits_nonzero() {
    let stash_dir = tempdir().unwrap();

    etch(stash_dir.path())
        .args(["--no-color", "rollback", "--path", "/no/stash/for/this"])
        .assert()
        .failure();
}
```

- [ ] **Step 2: Add `sha256` to app dev-dependencies**

The integration tests use `sha256::digest` to compute the hex key. Add to `app/Cargo.toml` under `[dev-dependencies]`:

```toml
sha256 = "1.6"
```

- [ ] **Step 3: Run integration tests**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run -p etch-cli --test rollback 2>&1 | tail -30
```

Expected: all pass. Note: `prune_limits_stash_count_to_three` takes ~4.4s due to sleep — this is expected.

- [ ] **Step 4: Run full suite**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
cargo nextest run 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
git add app/tests/rollback.rs app/Cargo.toml
git commit -m "test(rollback): add integration tests for stash and restore"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement                                                    | Task covering it            |
| ------------------------------------------------------------------- | --------------------------- |
| Stash store location `~/.local/share/etch/backups/<hex>/<ts>`       | Task 1, 2                   |
| StashMeta fields: original_path, stashed_at, apply_manifest, sha256 | Task 2                      |
| Prune policy (keep N, default 3)                                    | Task 2                      |
| `file.copy` stash hook (non-privileged)                             | Task 4                      |
| `file.copy` stash hook (privileged)                                 | Task 4                      |
| `file.link` never stashes                                           | Task 7 (integration)        |
| Best-effort stash (warns, never blocks)                             | Task 4 (Stash atom execute) |
| `etch rollback` list output                                         | Tasks 5, 7                  |
| `etch rollback --path` restore                                      | Tasks 5, 7                  |
| `etch rollback --path --dry-run` diff                               | Tasks 3, 5                  |
| `etch rollback --all --yes`                                         | Task 5                      |
| `--all --dry-run` error                                             | Task 5                      |
| `--all` without TTY and no `--yes`                                  | Task 5 (execute)            |
| Error on unknown path                                               | Tasks 3, 7                  |
| ETCH_STASH_DIR env var                                              | Tasks 1, 2, 7               |
| `with_base()` for unit test isolation                               | Task 1                      |
| Tarpaulin bifurcation on execute()                                  | Task 5                      |
| `similar` crate for diff                                            | Tasks 1, 3                  |

**Gaps:**

- `etch rollback --all` with TTY confirmation path is tested only via unit test of `validate_args`; full interactive test would require a pseudo-TTY and is outside scope.
- `etch rollback --path --dry-run` diff output format is not asserted in integration tests (stdout capture is complex); covered by unit test of `StashStore::restore(dry_run=true)`.

**Type consistency:** `StashEntry` defined in Task 1 is used as-is in Tasks 3, 5 — no rename divergence. `StashStore::with_base()` used in Tasks 2, 3 tests — consistent.

**Placeholder scan:** No "TBD", "TODO", or vague instructions found.

---

> _Do docs status updates (Pending→Done) post-merge on main, not inside the worktree._
