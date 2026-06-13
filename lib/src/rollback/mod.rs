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
