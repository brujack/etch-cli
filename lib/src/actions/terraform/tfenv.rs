use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::initializers::{FileExists, FlowControl};
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "terraform.tfenv")]
pub struct TerraformTfenv {
    /// Terraform version to install and set as default (e.g. "1.9.0").
    /// If omitted, only tfenv itself is installed.
    pub version: Option<String>,
}

impl Action for TerraformTfenv {
    fn summarize(&self) -> String {
        match &self.version {
            Some(v) => format!("Installing tfenv and Terraform {v}"),
            None => String::from("Installing tfenv"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let tfenv_dir = shellexpand::tilde("~/.tfenv").into_owned();
        let tfenv_bin = shellexpand::tilde("~/.tfenv/bin/tfenv").into_owned();

        let mut steps = vec![Step {
            atom: Box::new(Exec {
                command: String::from("git"),
                arguments: vec![
                    String::from("clone"),
                    String::from("https://github.com/tfutils/tfenv.git"),
                    tfenv_dir.clone(),
                ],
                ..Default::default()
            }),
            initializers: vec![FlowControl::SkipIf(Box::new(FileExists(PathBuf::from(
                &tfenv_dir,
            ))))],
            finalizers: vec![],
        }];

        if let Some(version) = &self.version {
            let versions_dir =
                shellexpand::tilde(&format!("~/.tfenv/versions/{version}")).into_owned();

            steps.push(Step {
                atom: Box::new(Exec {
                    command: tfenv_bin.clone(),
                    arguments: vec![String::from("install"), version.clone()],
                    ..Default::default()
                }),
                initializers: vec![FlowControl::SkipIf(Box::new(FileExists(PathBuf::from(
                    versions_dir,
                ))))],
                finalizers: vec![],
            });

            steps.push(Step {
                atom: Box::new(Exec {
                    command: tfenv_bin,
                    arguments: vec![String::from("use"), version.clone()],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized_without_version() {
        let yaml = r#"
- action: terraform.tfenv
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::TerraformTfenv(a)) => assert_eq!(None, a.action.version),
            _ => panic!("TerraformTfenv didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_version() {
        let yaml = r#"
- action: terraform.tfenv
  version: "1.9.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::TerraformTfenv(a)) => {
                assert_eq!(Some(String::from("1.9.0")), a.action.version)
            }
            _ => panic!("TerraformTfenv didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_without_version() {
        let action = TerraformTfenv { version: None };
        assert_eq!("Installing tfenv", action.summarize());
    }

    #[test]
    fn summarize_with_version() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        assert_eq!("Installing tfenv and Terraform 1.9.0", action.summarize());
    }

    #[test]
    fn plan_without_version_emits_one_step() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_version_emits_three_steps() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len());
    }

    #[test]
    fn plan_step1_clones_tfenv() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(display.contains("git"), "expected 'git' in: {display}");
        assert!(display.contains("clone"), "expected 'clone' in: {display}");
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
    }

    #[test]
    fn plan_step1_has_one_initializer() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            1,
            steps[0].initializers.len(),
            "expected 1 SkipIf initializer for idempotency"
        );
    }

    #[test]
    fn plan_step2_runs_tfenv_install() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[1].atom.to_string();
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
        assert!(
            display.contains("install"),
            "expected 'install' in: {display}"
        );
        assert!(display.contains("1.9.0"), "expected '1.9.0' in: {display}");
    }

    #[test]
    fn plan_step2_has_one_initializer() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            1,
            steps[1].initializers.len(),
            "expected 1 SkipIf initializer on install step"
        );
    }

    #[test]
    fn plan_step3_runs_tfenv_use() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[2].atom.to_string();
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
        assert!(display.contains("use"), "expected 'use' in: {display}");
        assert!(display.contains("1.9.0"), "expected '1.9.0' in: {display}");
    }

    #[test]
    fn plan_step3_has_no_initializers() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            0,
            steps[2].initializers.len(),
            "expected no initializers — tfenv use is idempotent"
        );
    }
}
