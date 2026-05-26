use super::super::Atom;
use crate::atoms::Outcome;

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
            .output()?
            .status;
        Ok(Outcome {
            side_effects: vec![],
            should_run: status.success(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn setup_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["-C", tmp.path().to_str().unwrap(), "init"])
            .status()
            .unwrap();
        tmp
    }

    fn local_config_args(path: &str) -> Vec<String> {
        vec!["-C".into(), path.into(), "config".into(), "--local".into()]
    }

    #[test]
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
}
