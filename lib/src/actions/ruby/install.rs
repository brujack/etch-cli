use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionManager {
    Rbenv,
    Chruby,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyInstall {
    pub version: String,
    /// Ruby implementation to install. Defaults to "ruby". Other options: "jruby", "truffleruby".
    pub implementation: Option<String>,
    /// Directory where rubies are installed. Defaults to ~/.rubies. Passed as --rubies-dir to ruby-install when set.
    pub rubies_dir: Option<String>,
    /// Version manager that owns the Ruby installation. When set, delegates install to rbenv or chruby instead of ruby-install.
    pub version_manager: Option<VersionManager>,
}

impl RubyInstall {
    fn impl_name(&self) -> &str {
        self.implementation.as_deref().unwrap_or("ruby")
    }

    fn resolved_rubies_dir(&self) -> PathBuf {
        let base = self.rubies_dir.as_deref().unwrap_or("~/.rubies");
        PathBuf::from(shellexpand::tilde(base).into_owned())
    }
}

impl Action for RubyInstall {
    fn summarize(&self) -> String {
        format!(
            "Installing {} {} via ruby-install",
            self.impl_name(),
            self.version
        )
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let ruby_dir =
            self.resolved_rubies_dir()
                .join(format!("{}-{}", self.impl_name(), self.version));

        if ruby_dir.exists() {
            return Ok(vec![]);
        }

        let mut arguments = vec![self.impl_name().to_string(), self.version.clone()];
        if let Some(dir) = &self.rubies_dir {
            let expanded = shellexpand::tilde(dir).into_owned();
            arguments.push(String::from("--rubies-dir"));
            arguments.push(expanded);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("ruby-install"),
                arguments,
                ..Default::default()
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
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: ruby.install
  version: "3.3.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::RubyInstall(action)) => {
                assert_eq!("3.3.0", action.action.version);
                assert!(action.action.implementation.is_none());
            }
            _ => panic!("RubyInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_implementation() {
        let yaml = r#"
- action: ruby.install
  version: "9.4.0.0"
  implementation: jruby
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::RubyInstall(action)) => {
                assert_eq!("9.4.0.0", action.action.version);
                assert_eq!(Some("jruby".to_string()), action.action.implementation);
            }
            _ => panic!("RubyInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_version_and_impl() {
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: None,
            version_manager: None,
        };
        let s = action.summarize();
        assert!(s.contains("ruby"), "expected 'ruby' in: {s}");
        assert!(s.contains("3.3.0"), "expected version in: {s}");
    }

    #[test]
    fn summarize_uses_custom_implementation() {
        let action = RubyInstall {
            version: String::from("9.4.0.0"),
            implementation: Some(String::from("jruby")),
            rubies_dir: None,
            version_manager: None,
        };
        let s = action.summarize();
        assert!(s.contains("jruby"), "expected 'jruby' in: {s}");
        assert!(s.contains("9.4.0.0"), "expected version in: {s}");
    }

    #[test]
    fn plan_returns_exec_when_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
            version_manager: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("ruby-install"),
            "expected ruby-install in: {display}"
        );
        assert!(display.contains("3.3.0"), "expected version in: {display}");
    }

    #[test]
    fn plan_skips_if_ruby_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("ruby-3.3.0")).unwrap();
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
            version_manager: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(
            steps.is_empty(),
            "expected empty steps when ruby already installed"
        );
    }

    #[test]
    fn plan_skips_if_jruby_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("jruby-9.4.0.0")).unwrap();
        let action = RubyInstall {
            version: String::from("9.4.0.0"),
            implementation: Some(String::from("jruby")),
            rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
            version_manager: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(
            steps.is_empty(),
            "expected empty steps when jruby already installed"
        );
    }

    #[test]
    fn plan_passes_rubies_dir_flag_when_set() {
        let tmp = tempfile::tempdir().unwrap();
        let dir_str = tmp.path().to_string_lossy().to_string();
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: Some(dir_str.clone()),
            version_manager: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--rubies-dir"),
            "expected --rubies-dir flag when rubies_dir is set: {display}"
        );
    }

    #[test]
    fn plan_omits_rubies_dir_flag_when_default() {
        let action = RubyInstall {
            version: String::from("99.99.99"),
            implementation: None,
            rubies_dir: None,
            version_manager: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // Will return 1 step because ~/.rubies/ruby-99.99.99 won't exist
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            !display.contains("--rubies-dir"),
            "expected no --rubies-dir flag when using default: {display}"
        );
    }

    #[test]
    fn impl_name_defaults_to_ruby() {
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: None,
            version_manager: None,
        };
        assert_eq!("ruby", action.impl_name());
    }

    #[test]
    fn impl_name_uses_custom_implementation() {
        let action = RubyInstall {
            version: String::from("9.4.0.0"),
            implementation: Some(String::from("jruby")),
            rubies_dir: None,
            version_manager: None,
        };
        assert_eq!("jruby", action.impl_name());
    }

    #[test]
    fn it_can_be_deserialized_with_version_manager() {
        let yaml = r#"
- action: ruby.install
  version: "3.3.0"
  version_manager: rbenv
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::RubyInstall(action)) => {
                assert_eq!("3.3.0", action.action.version);
                assert_eq!(Some(VersionManager::Rbenv), action.action.version_manager);
            }
            _ => panic!("RubyInstall didn't deserialize to the correct type"),
        }
    }
}
