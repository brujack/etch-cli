use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::atoms::git::GitConfigUnset;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitConfigScope {
    #[default]
    Global,
    Local,
    System,
}

#[allow(dead_code)]
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    pub scope: GitConfigScope,
    pub key: Option<String>,
    pub value: Option<String>,
    pub unset: Option<bool>,
    pub settings: Option<IndexMap<String, String>>,
    pub directory: Option<String>,
}

impl GitConfig {
    fn config_args(&self) -> Vec<String> {
        match &self.scope {
            GitConfigScope::Global => vec!["config".into(), "--global".into()],
            GitConfigScope::Local => {
                let dir = self.directory.as_deref().unwrap_or(".");
                vec!["-C".into(), dir.into(), "config".into(), "--local".into()]
            }
            GitConfigScope::System => vec!["config".into(), "--system".into()],
        }
    }
}

impl Action for GitConfig {
    fn summarize(&self) -> String {
        let scope = match self.scope {
            GitConfigScope::Global => "global",
            GitConfigScope::Local => "local",
            GitConfigScope::System => "system",
        };
        if let Some(ref key) = self.key {
            if self.unset == Some(true) {
                return format!("Unset git.{scope} {key}");
            }
            let val = self.value.as_deref().unwrap_or("(from settings)");
            return format!("Set git.{scope} {key} = {val}");
        }
        let count = self.settings.as_ref().map_or(0, |s| s.len());
        format!("Set {count} git.{scope} config values")
    }

    fn plan(&self, _manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        use anyhow::anyhow;

        if self.key.is_some() && self.settings.is_some() {
            return Err(anyhow!(
                "git.config: 'key' and 'settings' are mutually exclusive"
            ));
        }
        if self.key.is_none() && self.settings.is_none() {
            return Err(anyhow!(
                "git.config: one of 'key' or 'settings' is required"
            ));
        }
        if self.unset == Some(true) && self.settings.is_some() {
            return Err(anyhow!(
                "git.config: 'unset' cannot be used with 'settings'"
            ));
        }
        if self.unset == Some(true) && self.value.is_some() {
            return Err(anyhow!(
                "git.config: 'unset' and 'value' are mutually exclusive"
            ));
        }
        if matches!(self.scope, GitConfigScope::Local) && self.directory.is_none() {
            return Err(anyhow!(
                "git.config: 'directory' is required for scope 'local'"
            ));
        }
        if self.key.is_some() && self.value.is_none() && self.unset != Some(true) {
            return Err(anyhow!(
                "git.config: 'key' requires either 'value' (to set) or 'unset: true'"
            ));
        }

        let config_args = self.config_args();
        let privileged = matches!(self.scope, GitConfigScope::System);
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        // Single key set
        if let (Some(key), Some(value)) = (&self.key, &self.value) {
            let mut args = config_args;
            args.push(key.clone());
            args.push(value.clone());
            return Ok(vec![Step {
                atom: Box::new(Exec {
                    command: "git".into(),
                    arguments: args,
                    privileged,
                    privilege_provider,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        // Single key unset
        if self.unset == Some(true) {
            let key = self.key.as_ref().unwrap().clone();
            return Ok(vec![Step {
                atom: Box::new(GitConfigUnset {
                    config_args,
                    key,
                    privileged,
                    privilege_provider,
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        // Bulk settings map
        if let Some(ref settings) = self.settings {
            let steps = settings
                .iter()
                .map(|(key, value)| {
                    let mut args = config_args.clone();
                    args.push(key.clone());
                    args.push(value.clone());
                    Step {
                        atom: Box::new(Exec {
                            command: "git".into(),
                            arguments: args,
                            privileged,
                            privilege_provider: privilege_provider.clone(),
                            ..Default::default()
                        }),
                        initializers: vec![],
                        finalizers: vec![],
                    }
                })
                .collect();
            return Ok(steps);
        }

        // Unreachable: validation ensures one of key/settings is present.
        unreachable!("git.config: unhandled field combination (validation missed a case)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    fn plan(action: GitConfig) -> anyhow::Result<Vec<Step>> {
        action.plan(&Manifest::default(), &Contexts::default())
    }

    // TODO: uncomment after registration in Task 9
    // #[test]
    // fn deserialize_single_key_value() {
    //     let yaml = r#"
    // - action: git.config
    //   scope: global
    //   key: user.email
    //   value: test@example.com
    // "#;
    //     let mut actions: Vec<crate::actions::Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    //     match actions.pop() {
    //         Some(crate::actions::Actions::GitConfig(a)) => {
    //             assert_eq!(a.action.key, Some("user.email".into()));
    //             assert_eq!(a.action.value, Some("test@example.com".into()));
    //             assert!(matches!(a.action.scope, GitConfigScope::Global));
    //         }
    //         _ => panic!("wrong variant"),
    //     }
    // }

    // #[test]
    // fn deserialize_unset() {
    //     let yaml = r#"
    // - action: git.config
    //   scope: global
    //   key: credential.helper
    //   unset: true
    // "#;
    //     let mut actions: Vec<crate::actions::Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    //     match actions.pop() {
    //         Some(crate::actions::Actions::GitConfig(a)) => {
    //             assert_eq!(a.action.key, Some("credential.helper".into()));
    //             assert_eq!(a.action.unset, Some(true));
    //         }
    //         _ => panic!("wrong variant"),
    //     }
    // }

    // #[test]
    // fn deserialize_settings_map() {
    //     let yaml = r#"
    // - action: git.config
    //   scope: global
    //   settings:
    //     user.name: Bruce
    //     user.email: bruce@example.com
    // "#;
    //     let mut actions: Vec<crate::actions::Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    //     match actions.pop() {
    //         Some(crate::actions::Actions::GitConfig(a)) => {
    //             let s = a.action.settings.unwrap();
    //             assert_eq!(s.len(), 2);
    //             assert_eq!(s["user.name"], "Bruce");
    //         }
    //         _ => panic!("wrong variant"),
    //     }
    // }

    // #[test]
    // fn deserialize_local_scope() {
    //     let yaml = r#"
    // - action: git.config
    //   scope: local
    //   directory: /tmp/repo
    //   key: user.email
    //   value: local@example.com
    // "#;
    //     let mut actions: Vec<crate::actions::Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    //     match actions.pop() {
    //         Some(crate::actions::Actions::GitConfig(a)) => {
    //             assert!(matches!(a.action.scope, GitConfigScope::Local));
    //             assert_eq!(a.action.directory, Some("/tmp/repo".into()));
    //         }
    //         _ => panic!("wrong variant"),
    //     }
    // }

    #[test]
    fn error_key_and_settings_both_present() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            settings: Some({
                let mut m = IndexMap::new();
                m.insert("user.name".into(), "Foo".into());
                m
            }),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_neither_key_nor_settings() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_unset_with_settings() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            unset: Some(true),
            settings: Some({
                let mut m = IndexMap::new();
                m.insert("user.email".into(), "foo@bar.com".into());
                m
            }),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_unset_with_value() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            unset: Some(true),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_local_scope_without_directory() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_key_without_value_or_unset() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn plan_global_set_emits_one_exec_step() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("test@example.com".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("config"), "display: {display}");
        assert!(display.contains("--global"), "display: {display}");
        assert!(display.contains("user.email"), "display: {display}");
        assert!(display.contains("test@example.com"), "display: {display}");
        assert!(display.contains("privileged=false"), "display: {display}");
    }

    #[test]
    fn plan_local_set_includes_dash_c_and_local_flag() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            key: Some("user.email".into()),
            value: Some("local@example.com".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("-C"), "display: {display}");
        assert!(display.contains("/tmp/repo"), "display: {display}");
        assert!(display.contains("--local"), "display: {display}");
    }

    #[test]
    fn plan_system_set_is_privileged() {
        let action = GitConfig {
            scope: GitConfigScope::System,
            key: Some("credential.helper".into()),
            value: Some("osxkeychain".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("privileged=true"), "display: {display}");
        assert!(display.contains("--system"), "display: {display}");
    }

    #[test]
    fn plan_unset_emits_git_config_unset_step() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("credential.helper".into()),
            unset: Some(true),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("GitConfigUnset"), "display: {display}");
        assert!(display.contains("credential.helper"), "display: {display}");
    }

    #[test]
    fn plan_local_unset_includes_dir_in_config_args() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            key: Some("user.email".into()),
            unset: Some(true),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("GitConfigUnset"), "display: {display}");
        assert!(display.contains("user.email"), "display: {display}");
    }

    #[test]
    fn plan_settings_map_emits_one_step_per_key() {
        let mut settings = IndexMap::new();
        settings.insert("user.name".into(), "Bruce".into());
        settings.insert("user.email".into(), "bruce@example.com".into());
        settings.insert("core.autocrlf".into(), "false".into());
        let action = GitConfig {
            scope: GitConfigScope::Global,
            settings: Some(settings),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 3);
        // Verify insertion order preserved — user.name first
        assert!(steps[0].atom.to_string().contains("user.name"));
        assert!(steps[1].atom.to_string().contains("user.email"));
        assert!(steps[2].atom.to_string().contains("core.autocrlf"));
    }

    #[test]
    fn summarize_single_set() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("global"), "summary: {s}");
        assert!(s.contains("user.email"), "summary: {s}");
        assert!(s.contains("foo@bar.com"), "summary: {s}");
    }

    #[test]
    fn summarize_unset() {
        let action = GitConfig {
            scope: GitConfigScope::System,
            key: Some("credential.helper".into()),
            unset: Some(true),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("Unset"), "summary: {s}");
        assert!(s.contains("system"), "summary: {s}");
        assert!(s.contains("credential.helper"), "summary: {s}");
    }

    #[test]
    fn summarize_settings_map() {
        let mut settings = IndexMap::new();
        settings.insert("user.name".into(), "Bruce".into());
        settings.insert("user.email".into(), "bruce@example.com".into());
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            settings: Some(settings),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("2"), "summary: {s}");
        assert!(s.contains("local"), "summary: {s}");
    }
}
