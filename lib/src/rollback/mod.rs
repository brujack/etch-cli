use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
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
    pub base: PathBuf,
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

    pub fn stash(&self, path: &Path, manifest: &str, keep: usize) -> anyhow::Result<bool> {
        if !path.is_file() {
            return Ok(false);
        }

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = self.base.join(&hex);
        std::fs::create_dir_all(&hex_dir)
            .with_context(|| format!("rollback: cannot create backup dir {:?}", hex_dir))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hex_dir, std::fs::Permissions::from_mode(0o700))
                .with_context(|| format!("rollback: cannot set permissions on {:?}", hex_dir))?;
        }

        let now = Utc::now();
        let ts = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let stash_path = hex_dir.join(&ts);
        let meta_path = hex_dir.join(format!("{}.meta.yaml", ts));

        let content =
            std::fs::read(path).with_context(|| format!("rollback: cannot read {:?}", path))?;
        let content_hash = sha256::digest(content.as_slice());

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&stash_path)
                .with_context(|| format!("rollback: cannot write stash {:?}", stash_path))?;
            f.write_all(&content)
                .with_context(|| format!("rollback: cannot write stash {:?}", stash_path))?;
        }
        #[cfg(not(unix))]
        std::fs::write(&stash_path, &content)
            .with_context(|| format!("rollback: cannot write stash {:?}", stash_path))?;

        let meta = StashMeta {
            original_path: path.to_string_lossy().into_owned(),
            stashed_at: now,
            apply_manifest: manifest.to_string(),
            sha256: content_hash,
        };
        let meta_yaml = serde_yaml_ng::to_string(&meta).context("rollback: serialize meta")?;
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&meta_path)
                .with_context(|| format!("rollback: cannot write meta {:?}", meta_path))?;
            f.write_all(meta_yaml.as_bytes())
                .with_context(|| format!("rollback: cannot write meta {:?}", meta_path))?;
        }
        #[cfg(not(unix))]
        std::fs::write(&meta_path, meta_yaml)
            .with_context(|| format!("rollback: cannot write meta {:?}", meta_path))?;

        self.prune(path, keep)?;

        Ok(true)
    }

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
                let name = meta_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
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
            entries.sort_by_key(|e| std::cmp::Reverse(e.stashed_at));
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
                if name.ends_with(".meta.yaml") {
                    None
                } else {
                    Some(name)
                }
            })
            .collect();
        names.sort();

        let latest = names
            .last()
            .ok_or_else(|| anyhow::anyhow!("no stash found for {}", path.display()))?
            .clone();
        let stash_path = hex_dir.join(&latest);
        let stash_bytes = std::fs::read(&stash_path)
            .with_context(|| format!("rollback: cannot read stash {:?}", stash_path))?;

        if dry_run {
            let current = if path.exists() {
                std::fs::read_to_string(path)
                    .with_context(|| format!("rollback: cannot read current {:?}", path))?
            } else {
                String::new()
            };
            let stash_content = String::from_utf8_lossy(&stash_bytes);
            let diff = TextDiff::from_lines(current.as_str(), stash_content.as_ref());
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("rollback: cannot create parent dir for {:?}", path))?;
        }
        std::fs::write(path, &stash_bytes)
            .with_context(|| format!("rollback: cannot restore to {:?}", path))?;
        println!("Done.");

        Ok(())
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
            if let Err(e) = std::fs::remove_file(hex_dir.join(&name)) {
                tracing::warn!("rollback: prune failed to delete {}: {e}", name);
            }
            if let Err(e) = std::fs::remove_file(hex_dir.join(format!("{}.meta.yaml", name))) {
                tracing::warn!("rollback: prune failed to delete {}.meta.yaml: {e}", name);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
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
        let result = store
            .stash(std::path::Path::new("/nonexistent/path"), "m", 3)
            .unwrap();
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
        let age = chrono::Utc::now() - meta.stashed_at;
        assert!(age.num_seconds() < 5);
    }

    #[test]
    fn prune_keeps_n_newest_deletes_oldest() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");

        // Create 4 stashes with distinct second-resolution timestamps
        for i in 0..4u8 {
            std::fs::write(&path, [i]).unwrap();
            store.stash(&path, "m", 10).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }

        store.prune(&path, 2).unwrap();

        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let stash_count = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .count();
        assert_eq!(stash_count, 2, "keep=2 leaves 2 stashes");

        let meta_count = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .count();
        assert_eq!(meta_count, 2, "keep=2 leaves 2 sidecars");
    }

    #[test]
    fn prune_noop_when_count_le_keep() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");

        for i in 0..2u8 {
            std::fs::write(&path, [i]).unwrap();
            store.stash(&path, "m", 10).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }

        store.prune(&path, 3).unwrap();

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
    #[serial]
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
        assert!(
            entries[0].stashed_at > entries[1].stashed_at,
            "newest first"
        );
    }

    #[test]
    fn restore_dry_run_does_not_write() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "original").unwrap();
        store.stash(&path, "m", 3).unwrap();
        std::fs::write(&path, "modified").unwrap();

        store.restore(&path, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "modified",
            "dry_run must not write"
        );
    }

    #[test]
    fn restore_writes_stash_content_and_preserves_stash_file() {
        let (store, _dir) = make_store();
        let src = tempdir().unwrap();
        let path = src.path().join("file.txt");
        std::fs::write(&path, "original").unwrap();
        store.stash(&path, "m", 3).unwrap();
        std::fs::write(&path, "modified").unwrap();

        store.restore(&path, false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");

        // stash file must still exist
        let hex = sha256::digest(path.to_string_lossy().as_ref());
        let hex_dir = store.base.join(&hex);
        let count = std::fs::read_dir(&hex_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
            .count();
        assert_eq!(count, 1, "stash file preserved after restore");
    }

    #[test]
    fn restore_errors_when_no_stash_for_path() {
        let (store, _dir) = make_store();
        let result = store.restore(std::path::Path::new("/no/stash/here"), false);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("no stash found"),
            "expected 'no stash found', got: {msg}"
        );
    }
}
