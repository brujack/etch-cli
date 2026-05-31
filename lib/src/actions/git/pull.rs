use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPull {
    pub repo_url: String,
    pub directory: String,
    /// Skip this action entirely if the given path does not exist.
    pub skip_if_not_exists: Option<String>,
}

impl Action for GitPull {
    fn summarize(&self) -> String {
        format!(
            "Pulling repository {} into {}",
            self.repo_url, self.directory
        )
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if let Some(path) = &self.skip_if_not_exists {
            if !PathBuf::from(path).exists() {
                return Ok(vec![]);
            }
        }
        let _ = gix::url::parse(self.repo_url.as_str().into())?;
        Ok(vec![Step {
            atom: Box::new(crate::atoms::git::Pull {
                repository: self.repo_url.clone(),
                directory: PathBuf::from(self.directory.clone()),
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn plan_returns_one_step_for_valid_url() {
        let action = GitPull {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: String::from("/tmp/repo"),
            skip_if_not_exists: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_errors_on_invalid_url() {
        let action = GitPull {
            repo_url: String::from("not a url ://"),
            directory: String::from("/tmp/repo"),
            skip_if_not_exists: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn plan_skips_when_skip_if_not_exists_path_absent() {
        let action = GitPull {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: String::from("/tmp/repo"),
            skip_if_not_exists: Some(String::from("/tmp/this-path-does-not-exist-etch-test")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn plan_runs_when_skip_if_not_exists_path_present() {
        let tmp = tempfile::tempdir().unwrap();
        let action = GitPull {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: String::from("/tmp/repo"),
            skip_if_not_exists: Some(tmp.path().to_string_lossy().to_string()),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }
}
