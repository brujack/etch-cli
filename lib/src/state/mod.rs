use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StateEntry {
    pub manifest: String,
    pub action: String,
    pub key: String,
    pub applied_at: DateTime<Utc>,
    pub sha256: Option<String>,
    pub changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub schema_version: u32,
    pub last_apply: DateTime<Utc>,
    pub atoms: Vec<StateEntry>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            last_apply: Utc::now(),
            atoms: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct StateStore {
    path: PathBuf,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    /// Resolves `dirs_next::data_local_dir()/etch/state.yaml`.
    /// Falls back to `~/.local/share/etch/state.yaml` if dirs_next returns None.
    /// Override with `ETCH_STATE_DIR` env var (used by integration tests).
    pub fn new() -> Self {
        let base = if let Ok(dir) = std::env::var("ETCH_STATE_DIR") {
            PathBuf::from(dir)
        } else {
            dirs_next::data_local_dir()
                .unwrap_or_else(|| {
                    tracing::debug!(
                        "data_local_dir() returned None; using ~/.local/share fallback"
                    );
                    dirs_next::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".local")
                        .join("share")
                })
                .join("etch")
        };
        let path = base.join("state.yaml");
        Self { path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns empty State if file is missing; treats corrupt YAML as empty.
    pub fn load(&self) -> anyhow::Result<State> {
        if !self.path.exists() {
            return Ok(State::default());
        }
        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("cannot read state file {:?}", self.path))?;
        match serde_yaml_ng::from_str::<State>(&content) {
            Ok(state) => Ok(state),
            Err(_) => {
                tracing::warn!("state: corrupt file, resetting");
                Ok(State::default())
            }
        }
    }

    /// Atomic write: serialize to `.state.yaml.tmp`, then `fs::rename`.
    pub fn save(&self, state: &State) -> anyhow::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("state path has no parent"))?;
        std::fs::create_dir_all(dir)
            .with_context(|| format!("state: cannot create directory {:?}", dir))?;
        let tmp = self.path.with_extension("yaml.tmp");
        let content = serde_yaml_ng::to_string(state).context("state: failed to serialize")?;
        std::fs::write(&tmp, &content)
            .with_context(|| format!("state: cannot write tmp file {:?}", tmp))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("state: cannot rename {:?} to {:?}", tmp, self.path))?;
        Ok(())
    }

    /// Load, merge entries (replace same manifest+action+key triple), save.
    /// Updates `last_apply` to now. Never appends duplicates.
    pub fn record(&self, entries: Vec<StateEntry>) -> anyhow::Result<()> {
        let mut state = self.load()?;
        for entry in entries {
            let pos = state.atoms.iter().position(|e| {
                e.manifest == entry.manifest && e.action == entry.action && e.key == entry.key
            });
            match pos {
                Some(i) => state.atoms[i] = entry,
                None => state.atoms.push(entry),
            }
        }
        state.last_apply = Utc::now();
        self.save(&state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_entry(manifest: &str, action: &str, key: &str, changed: bool) -> StateEntry {
        StateEntry {
            manifest: manifest.to_string(),
            action: action.to_string(),
            key: key.to_string(),
            applied_at: Utc::now(),
            sha256: None,
            changed,
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("state.yaml"));
        let entry = make_entry("core.yaml", "file.copy", "~/.zshrc", true);
        let mut state = State::default();
        state.atoms.push(entry.clone());
        store.save(&state).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.atoms.len(), 1);
        assert_eq!(loaded.atoms[0].manifest, "core.yaml");
        assert_eq!(loaded.atoms[0].action, "file.copy");
        assert_eq!(loaded.atoms[0].key, "~/.zshrc");
        assert!(loaded.atoms[0].changed);
    }

    #[test]
    fn atomic_write_no_tmp_after_save() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.yaml");
        let store = StateStore::with_path(state_path.clone());
        store.save(&State::default()).unwrap();
        let tmp = state_path.with_extension("yaml.tmp");
        assert!(!tmp.exists(), "tmp file should not exist after save");
        assert!(state_path.exists(), "state file should exist after save");
    }

    #[test]
    fn record_merge_same_triple_updates_not_appends() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("state.yaml"));
        let entry1 = make_entry("core.yaml", "file.copy", "~/.zshrc", true);
        store.record(vec![entry1]).unwrap();

        let entry2 = make_entry("core.yaml", "file.copy", "~/.zshrc", false);
        store.record(vec![entry2]).unwrap();

        let state = store.load().unwrap();
        assert_eq!(state.atoms.len(), 1, "merge should not append");
        assert!(!state.atoms[0].changed, "should be updated to false");
    }

    #[test]
    fn record_append_distinct_triples() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("state.yaml"));
        store
            .record(vec![make_entry("core.yaml", "file.copy", "~/.zshrc", true)])
            .unwrap();
        store
            .record(vec![make_entry(
                "core.yaml",
                "git.clone",
                "~/.oh-my-zsh",
                false,
            )])
            .unwrap();
        store
            .record(vec![make_entry(
                "tools.yaml",
                "file.copy",
                "~/.zshrc",
                false,
            )])
            .unwrap();

        let state = store.load().unwrap();
        assert_eq!(state.atoms.len(), 3);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("nonexistent.yaml"));
        let state = store.load().unwrap();
        assert_eq!(state.schema_version, 1);
        assert!(state.atoms.is_empty());
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.yaml");
        std::fs::write(&path, "not: valid: yaml: {{{").unwrap();
        let store = StateStore::with_path(path);
        let state = store.load().unwrap();
        assert_eq!(state.schema_version, 1);
        assert!(state.atoms.is_empty());
    }

    #[test]
    fn schema_version_roundtrips_as_1() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("state.yaml"));
        store.save(&State::default()).unwrap();
        let state = store.load().unwrap();
        assert_eq!(state.schema_version, 1);
    }

    #[test]
    fn last_apply_updated_on_record() {
        let dir = tempdir().unwrap();
        let store = StateStore::with_path(dir.path().join("state.yaml"));
        let before = Utc::now();
        store.record(vec![make_entry("m", "a", "k", true)]).unwrap();
        let after = Utc::now();

        let state = store.load().unwrap();
        assert!(state.last_apply >= before);
        assert!(state.last_apply <= after);
    }

    #[test]
    fn new_with_etch_state_dir_env_var() {
        let dir = tempdir().unwrap();
        let old = std::env::var("ETCH_STATE_DIR").ok();
        std::env::set_var("ETCH_STATE_DIR", dir.path());
        let store = StateStore::new();
        if let Some(v) = old {
            std::env::set_var("ETCH_STATE_DIR", v);
        } else {
            std::env::remove_var("ETCH_STATE_DIR");
        }
        // The store should write to dir/state.yaml
        store.record(vec![make_entry("m", "a", "k", true)]).unwrap();
        assert!(dir.path().join("state.yaml").exists());
    }
}
