use super::super::Atom;
use crate::atoms::Outcome;
use std::path::PathBuf;
use tracing::instrument;

pub struct Pull {
    pub repository: String,
    pub directory: PathBuf,
}

impl std::fmt::Display for Pull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GitPull {} into {}",
            self.repository,
            self.directory.display()
        )
    }
}

impl Atom for Pull {
    #[instrument(name = "git.pull.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "git.pull.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        if !self.directory.exists() {
            let status = std::process::Command::new("git")
                .args(["clone", &self.repository, &self.directory.to_string_lossy()])
                .status()?;
            if !status.success() {
                anyhow::bail!(
                    "git clone {} {} failed with {}",
                    self.repository,
                    self.directory.display(),
                    status
                );
            }
        } else {
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
        }
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
    fn plan_always_should_run_when_directory_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("not_yet");
        let atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: missing,
        };
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn plan_always_should_run_when_directory_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: tmp.path().to_path_buf(),
        };
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn display_format() {
        let atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: PathBuf::from("/tmp/myrepo"),
        };
        let s = format!("{atom}");
        assert!(
            s.contains("https://github.com/example/repo.git"),
            "missing repo URL in: {s}"
        );
        assert!(s.contains("/tmp/myrepo"), "missing directory in: {s}");
    }

    #[test]
    #[serial]
    fn execute_clones_when_directory_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 0);

        let target = tmp.path().join("new_repo");
        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: target.clone(),
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("clone"),
            "expected 'clone' in git calls log, got: {log}"
        );
        assert!(
            log.contains("https://github.com/example/repo.git"),
            "expected repo URL in git calls log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_pulls_when_directory_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 0);

        // Directory already exists
        let target = tmp.path().join("existing_repo");
        std::fs::create_dir_all(&target).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: target.clone(),
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("pull"),
            "expected 'pull' in git calls log, got: {log}"
        );
        assert!(
            log.contains("-C"),
            "expected '-C' flag in git calls log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_propagates_clone_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 1);

        let target = tmp.path().join("new_repo");
        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: target,
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from failed clone, got Ok");
    }

    #[test]
    #[serial]
    fn execute_propagates_pull_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_git(mock_dir.path(), &calls_file, 1);

        // Directory already exists — triggers pull path
        let target = tmp.path().join("existing_repo");
        std::fs::create_dir_all(&target).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = Pull {
            repository: "https://github.com/example/repo.git".into(),
            directory: target,
        };
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from failed pull, got Ok");
    }
}
