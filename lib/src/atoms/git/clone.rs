use super::super::Atom;
use crate::atoms::Outcome;
use gix::interrupt;
use gix::{progress::Discard, Url};
use std::path::PathBuf;
use tracing::instrument;

#[derive(Default)]
pub struct Clone {
    pub repository: Url,
    pub directory: PathBuf,
}

impl std::fmt::Display for Clone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GitClone {} to {}",
            self.repository,
            self.directory.display()
        )
    }
}

impl Atom for Clone {
    #[instrument(name = "git.clone.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: !self.directory.exists(),
        })
    }

    #[instrument(name = "git.clone.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        unsafe {
            interrupt::init_handler(1, || {})?;
        };

        std::fs::create_dir_all(&self.directory)?;

        let mut prepare_clone = gix::prepare_clone(self.repository.clone(), &self.directory)?;
        let (mut prepare_checkout, _) = prepare_clone
            .fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED)?;

        let (repo, _) = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED)?;

        let _ = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .expect("always present after clone")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_should_run_when_directory_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not_yet_cloned");
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_directory_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
        };
        assert!(!atom.plan().unwrap().should_run);
    }
}
