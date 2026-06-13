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

        let now = Utc::now();
        let ts = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let stash_path = hex_dir.join(&ts);
        let meta_path = hex_dir.join(format!("{}.meta.yaml", ts));

        let content =
            std::fs::read(path).with_context(|| format!("rollback: cannot read {:?}", path))?;
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

    pub fn list(&self) -> anyhow::Result<Vec<(PathBuf, Vec<StashEntry>)>> {
        todo!()
    }

    pub fn restore(&self, _path: &Path, _dry_run: bool) -> anyhow::Result<()> {
        todo!()
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
}

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
