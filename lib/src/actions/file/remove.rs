use std::path::PathBuf;

use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{actions::Action, steps::Step};

use super::{FileAction, FileActionConfig};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRemove {
    pub target: String,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

impl FileRemove {}

impl FileAction for FileRemove {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileRemove {
    fn summarize(&self) -> String {
        format!("Removing file {}", self.target)
    }

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        if self.config.privileged {
            return Err(anyhow!("file.remove does not support privileged mode"));
        }

        use crate::atoms::file::Remove as RemoveFile;

        let path = PathBuf::from(&self.target);

        let steps = vec![Step {
            atom: Box::new(RemoveFile { target: path }),
            initializers: vec![],
            finalizers: vec![],
        }];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.remove
  target: a
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileRemove(action)) => {
                assert_eq!("a", action.action.target);
            }
            _ => {
                panic!("FileRemove didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn plan_returns_remove_step() {
        use super::FileRemove;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = FileRemove {
            target: String::from("/tmp/somefile"),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_errors_when_privileged_not_supported() {
        use super::FileRemove;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileRemove {
            target: String::from("/tmp/somefile"),
            config: FileActionConfig { privileged: true },
        };
        assert!(action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default()
            )
            .is_err());
    }
}
