use super::super::Atom;
use crate::atoms::command::Exec;
use crate::atoms::Outcome;

#[derive(Debug)]
pub struct GitConfigUnset {
    /// Args inserted between `git` and `--get`/`--unset`:
    /// global → ["config", "--global"]
    /// local  → ["-C", "/path", "config", "--local"]
    /// system → ["config", "--system"]
    pub config_args: Vec<String>,
    pub key: String,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for GitConfigUnset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitConfigUnset key={}", self.key)
    }
}

impl Atom for GitConfigUnset {
    fn plan(&self) -> anyhow::Result<Outcome> {
        let mut args = self.config_args.clone();
        args.push("--get".into());
        args.push(self.key.clone());
        let status = std::process::Command::new("git")
            .args(&args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()?
            .status;
        Ok(Outcome {
            side_effects: vec![],
            should_run: status.success(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let mut args = self.config_args.clone();
        args.push("--unset".into());
        args.push(self.key.clone());

        if self.privileged {
            // Privileged path (system scope) — GIT_DIR does not affect /etc/gitconfig.
            // Delegate to Exec so sudo/doas/run0 wrapping is handled correctly.
            let mut exec = Exec {
                command: "git".into(),
                arguments: args,
                privileged: true,
                privilege_provider: self.privilege_provider.clone(),
                ..Default::default()
            };
            exec.execute()
        } else {
            // Non-privileged path (global/local scope) — clear git hook env vars so
            // that `git -C <dir> config --local --unset` targets the intended repo,
            // not the repo that launched the hook (GIT_DIR override takes precedence
            // over -C when both are present).
            let status = std::process::Command::new("git")
                .args(&args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "git config --unset failed: exit code {}",
                    status.code().unwrap_or(1)
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::process::Command;

    fn setup_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["-C", tmp.path().to_str().unwrap(), "init"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        tmp
    }

    fn local_config_args(path: &str) -> Vec<String> {
        vec!["-C".into(), path.into(), "config".into(), "--local".into()]
    }

    #[test]
    #[serial]
    fn plan_should_run_false_when_key_absent() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        let atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.email".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    #[serial]
    fn plan_should_run_true_when_key_present() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        Command::new("git")
            .args([
                "-C",
                path,
                "config",
                "--local",
                "user.email",
                "test@example.com",
            ])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        let atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.email".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    #[serial]
    fn execute_removes_the_key() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        // Set the key first
        Command::new("git")
            .args(["-C", path, "config", "--local", "user.name", "Test User"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        // Verify it exists
        let before = Command::new("git")
            .args(["-C", path, "config", "--local", "--get", "user.name"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        assert!(before.success());

        let mut atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.name".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        atom.execute().unwrap();

        // Key should be gone
        let after = Command::new("git")
            .args(["-C", path, "config", "--local", "--get", "user.name"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        assert!(!after.success());
    }

    // Regression test: execute() must target the correct repo even when GIT_DIR
    // is set (as it is inside a git pre-push hook). Without env_remove in execute(),
    // `git -C <tmp> config --local --unset key` would silently target the hook's
    // repo instead of <tmp>, potentially corrupting its gitconfig or failing if
    // the key does not exist there.
    #[test]
    #[serial]
    fn execute_targets_correct_repo_when_git_dir_is_set() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();

        // Set the key in the temp repo (env_remove so it goes to the right place)
        Command::new("git")
            .args(["-C", path, "config", "--local", "user.name", "Hook Test"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();

        let mut atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.name".into(),
            privileged: false,
            privilege_provider: String::new(),
        };

        // Simulate a git hook environment by setting GIT_DIR to a non-existent path.
        // execute() must still target `path`, not whatever GIT_DIR points to.
        let result = {
            // Safety: std::env is process-global; serial_test ensures no concurrent tests.
            unsafe {
                std::env::set_var("GIT_DIR", "/nonexistent/.git");
            }
            let r = atom.execute();
            unsafe {
                std::env::remove_var("GIT_DIR");
            }
            r
        };

        assert!(result.is_ok(), "execute() failed: {:?}", result.err());

        // Confirm the key is gone from the temp repo
        let after = Command::new("git")
            .args(["-C", path, "config", "--local", "--get", "user.name"])
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .unwrap();
        assert!(
            !after.success(),
            "key should have been removed from the temp repo"
        );
    }

    #[test]
    fn display_includes_key() {
        let atom = GitConfigUnset {
            config_args: vec!["config".into(), "--global".into()],
            key: "credential.helper".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(format!("{atom}").contains("credential.helper"));
    }
}
