use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitClone {
    pub repo_url: String,
    pub directory: String,
    #[serde(default)]
    pub update_existing: bool,
}

impl Action for GitClone {
    fn summarize(&self) -> String {
        format!("Cloning repository {} to {}", self.repo_url, self.directory)
    }

    fn state_key(&self) -> String {
        self.directory.clone()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let url = gix::url::parse(self.repo_url.as_str().into())?;
        Ok(vec![Step {
            atom: Box::new(crate::atoms::git::Clone {
                repository: url.clone(),
                directory: PathBuf::from(self.directory.clone()),
                update_existing: self.update_existing,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitClone(action)) => {
                assert_eq!(
                    "https://github.com/example/repo.git",
                    action.action.repo_url
                );
                assert_eq!("/tmp/repo", action.action.directory);
            }
            _ => panic!("GitClone didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_one_step_for_valid_url() {
        let action = GitClone {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: String::from("/tmp/repo"),
            update_existing: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_errors_on_invalid_url() {
        let action = GitClone {
            repo_url: String::from("not a url ://"),
            directory: String::from("/tmp/repo"),
            update_existing: false,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn deserialization_with_update_existing_true() {
        let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
  update_existing: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitClone(action)) => {
                assert!(action.action.update_existing);
            }
            _ => panic!("GitClone didn't deserialize"),
        }
    }

    #[test]
    fn deserialization_defaults_update_existing_false() {
        let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitClone(action)) => {
                assert!(!action.action.update_existing);
            }
            _ => panic!("GitClone didn't deserialize"),
        }
    }

    #[test]
    fn plan_passes_update_existing_true_to_atom() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let action = GitClone {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: tmp.path().to_string_lossy().into_owned(),
            update_existing: true,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.plan().unwrap().should_run);
    }
}
