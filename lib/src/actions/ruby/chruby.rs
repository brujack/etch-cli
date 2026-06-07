use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyChruby {
    /// Ruby version to set as default in ~/.ruby-version.
    /// Verbatim string written to the file (e.g. "ruby-3.3.0").
    /// If omitted, ~/.ruby-version is not written.
    pub default_version: Option<String>,
}

impl Action for RubyChruby {
    fn summarize(&self) -> String {
        match &self.default_version {
            Some(v) => format!("Installing chruby and setting default ruby to {v}"),
            None => String::from("Installing chruby"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;
        use crate::atoms::file::SetContents;

        let mut steps = vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("install"), String::from("chruby")],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Some(version) = &self.default_version {
            let path = PathBuf::from(shellexpand::tilde("~/.ruby-version").into_owned());
            steps.push(Step {
                atom: Box::new(SetContents {
                    path,
                    contents: format!("{version}\n").into_bytes(),
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
    fn it_can_be_deserialized_without_default_version() {
        let yaml = r#"
actions:
  - action: ruby.chruby
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::RubyChruby(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(None, action.default_version);
    }

    #[test]
    fn it_can_be_deserialized_with_default_version() {
        let yaml = r#"
actions:
  - action: ruby.chruby
    default_version: "ruby-3.3.0"
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::RubyChruby(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(Some(String::from("ruby-3.3.0")), action.default_version);
    }

    #[test]
    fn plan_without_default_version_emits_one_step() {
        let action = RubyChruby {
            default_version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_default_version_emits_two_steps() {
        let action = RubyChruby {
            default_version: Some(String::from("ruby-3.3.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
    }
}
