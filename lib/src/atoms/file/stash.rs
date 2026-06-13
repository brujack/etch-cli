use crate::atoms::{Atom, Outcome};
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
        Ok(Outcome {
            side_effects: vec![],
            should_run: self.path.is_file(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        match StashStore::new().stash(&self.path, &self.manifest, self.keep) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("rollback: stash failed for {}: {e:#}", self.path.display());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn plan_should_run_true_for_regular_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "content").unwrap();
        let atom = Stash {
            path,
            manifest: "m".into(),
            keep: 3,
        };
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
        let atom = Stash {
            path: dir.path().to_path_buf(),
            manifest: "m".into(),
            keep: 3,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_returns_ok_on_success() {
        let stash_dir = tempdir().unwrap();
        let src_dir = tempdir().unwrap();
        let path = src_dir.path().join("file.txt");
        std::fs::write(&path, "data").unwrap();

        let old = std::env::var("ETCH_STASH_DIR").ok();
        std::env::set_var("ETCH_STASH_DIR", stash_dir.path());

        let mut atom = Stash {
            path,
            manifest: "m".into(),
            keep: 3,
        };
        let result = atom.execute();

        if let Some(v) = old {
            std::env::set_var("ETCH_STASH_DIR", v);
        } else {
            std::env::remove_var("ETCH_STASH_DIR");
        }

        assert!(result.is_ok());
    }

    #[test]
    fn execute_returns_ok_even_when_stash_skips() {
        // Path is a directory — stash returns Ok(false), not Err; execute must return Ok
        let dir = tempdir().unwrap();
        let mut atom = Stash {
            path: dir.path().to_path_buf(),
            manifest: "m".into(),
            keep: 3,
        };
        assert!(
            atom.execute().is_ok(),
            "execute() must never fail even when stash skips"
        );
    }
}
