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
    pub update_existing: bool,
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
        if self.directory.exists() {
            if self.update_existing {
                if !self.directory.join(".git").exists() {
                    anyhow::bail!(
                        "directory {} exists but is not a git repository",
                        self.directory.display()
                    );
                }
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: true,
                });
            }
            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "git.clone.execute", level = "info", skip(self))]
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
    use serial_test::serial;

    fn write_mock_git(mock_dir: &std::path::Path, calls_file: &std::path::Path, exit_code: i32) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("git");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'git %s\\n' \"$*\" >> '{}'\nexit {}\n",
            calls_file.display(),
            exit_code
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn display_format() {
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: std::path::PathBuf::from("/tmp/repo"),
            update_existing: false,
        };
        let display = format!("{atom}");
        assert!(display.contains("repo.git"));
        assert!(display.contains("/tmp/repo"));
    }

    #[test]
    fn plan_should_run_when_directory_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not_yet_cloned");
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target,
            update_existing: false,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_directory_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
            update_existing: false,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_skips_when_dir_exists_update_existing_false() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
            update_existing: false,
        };
        let outcome = atom.plan().unwrap();
        assert!(!outcome.should_run);
    }

    #[test]
    fn plan_runs_when_dir_exists_with_git_update_existing_true() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
            update_existing: true,
        };
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
    }

    #[test]
    fn plan_errors_when_dir_exists_no_git_update_existing_true() {
        let tmp = tempfile::tempdir().unwrap();
        // No .git directory — not a git repo
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
            update_existing: true,
        };
        assert!(atom.plan().is_err());
    }

    #[test]
    fn plan_runs_when_dir_missing_update_existing_true() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not_yet");
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target,
            update_existing: true,
        };
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
    }

    #[test]
    #[serial]
    fn execute_pulls_when_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 0);

        let target = tmp.path().join("existing_repo");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(target.join(".git")).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target.clone(),
            update_existing: true,
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(log.contains("pull"), "expected 'pull' in log, got: {log}");
        assert!(log.contains("-C"), "expected '-C' flag in log, got: {log}");
    }

    #[test]
    #[serial]
    fn execute_propagates_pull_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 1);

        let target = tmp.path().join("existing_repo");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(target.join(".git")).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target,
            update_existing: true,
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from failed pull");
    }
}
