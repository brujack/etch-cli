use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{FileAction, FileActionConfig};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

fn parse_mode(mode: &str) -> anyhow::Result<u32> {
    let stripped = mode.strip_prefix("0o").unwrap_or(mode);
    u32::from_str_radix(stripped, 8).map_err(|_| anyhow!("invalid mode: {}", mode))
}

impl FileAction for FileChmod {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileChmod {
    fn summarize(&self) -> String {
        format!("Set permissions {} on {}", self.mode, self.path)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.config.privileged {
            use crate::atoms::command::Exec;
            let privilege_provider =
                utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
            return Ok(vec![Step {
                atom: Box::new(Exec {
                    command: "chmod".into(),
                    arguments: vec![self.mode.clone(), self.path.clone()],
                    privileged: true,
                    privilege_provider,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        let mode = parse_mode(&self.mode)?;
        Ok(vec![Step {
            atom: Box::new(Chmod {
                path: self.path.clone().parse()?,
                mode,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.chmod
  path: /tmp/testdir
  mode: "700"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChmod(action)) => {
                assert_eq!("/tmp/testdir", action.action.path);
                assert_eq!("700", action.action.mode);
                assert!(!action.action.config.privileged);
            }
            _ => panic!("FileChmod didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_chmod_step() {
        use super::FileChmod;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("700"),
            config: FileActionConfig { privileged: false },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.to_string().contains("need to be set"));
    }

    #[test]
    fn plan_errors_on_invalid_mode() {
        use super::FileChmod;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("xyz"),
            config: FileActionConfig { privileged: false },
        };
        let result = action.plan(
            &crate::manifests::Manifest::default(),
            &crate::contexts::Contexts::default(),
        );
        match result {
            Err(e) => assert!(e.to_string().contains("invalid mode")),
            Ok(_) => panic!("expected an error for invalid mode"),
        }
    }

    #[test]
    fn plan_accepts_0o_prefixed_mode() {
        use super::FileChmod;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("0o700"),
            config: FileActionConfig { privileged: false },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.to_string().contains("need to be set"));
    }

    #[test]
    fn plan_returns_exec_step_when_privileged() {
        use super::FileChmod;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("700"),
            config: FileActionConfig { privileged: true },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(!steps[0].atom.to_string().contains("need to be set"));
    }

    #[test]
    fn summarize_includes_path_and_mode() {
        use super::FileChmod;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("755"),
            config: FileActionConfig { privileged: false },
        };
        let summary = action.summarize();
        assert!(summary.contains("/tmp/testdir"));
        assert!(summary.contains("755"));
    }
}
