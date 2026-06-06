use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudePluginUpdate {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
}

impl ClaudePluginUpdate {
    fn plugin_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }
}

impl Action for ClaudePluginUpdate {
    fn summarize(&self) -> String {
        let names = self.plugin_names();
        if names.is_empty() {
            return String::from("Updating Claude plugins");
        }
        format!("Updating Claude plugin(s): {}", names.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let names = self.plugin_names();
        if names.is_empty() {
            bail!("claude.plugin.update requires either 'name' or 'list'");
        }

        // No idempotency check — update always runs. Caller is responsible for
        // ensuring the plugin is installed (use claude.install for that).

        let steps = names
            .into_iter()
            .map(|name| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![String::from("plugins"), String::from("update"), name],
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect();

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized_name() {
        let yaml = "- action: claude.plugin.update\n  name: superpowers\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudePluginUpdate(a)) => {
                assert_eq!(a.action.name.as_deref(), Some("superpowers"));
            }
            _ => panic!("expected ClaudePluginUpdate"),
        }
    }

    #[test]
    fn it_can_be_deserialized_list() {
        let yaml = concat!(
            "- action: claude.plugin.update\n",
            "  list:\n",
            "    - superpowers\n",
            "    - context7\n",
        );
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudePluginUpdate(a)) => {
                assert_eq!(a.action.list, vec!["superpowers", "context7"]);
            }
            _ => panic!("expected ClaudePluginUpdate"),
        }
    }

    #[test]
    fn summarize_includes_plugin_name() {
        let action = ClaudePluginUpdate {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "got: {s}");
    }

    #[test]
    fn summarize_includes_all_list_plugins() {
        let action = ClaudePluginUpdate {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "got: {s}");
        assert!(s.contains("context7"), "got: {s}");
    }

    #[test]
    fn summarize_with_no_plugins_returns_generic() {
        let s = ClaudePluginUpdate::default().summarize();
        assert!(s.to_lowercase().contains("claude"), "got: {s}");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let result = ClaudePluginUpdate::default().plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("name") || msg.contains("list"), "got: {msg}");
    }

    #[test]
    fn plan_returns_exec_for_name() {
        let action = ClaudePluginUpdate {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("claude"), "got: {display}");
        assert!(display.contains("plugins"), "got: {display}");
        assert!(display.contains("update"), "got: {display}");
        assert!(display.contains("superpowers"), "got: {display}");
    }

    #[test]
    fn plan_returns_exec_for_list() {
        let action = ClaudePluginUpdate {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
        let d0 = steps[0].atom.to_string();
        let d1 = steps[1].atom.to_string();
        assert!(d0.contains("superpowers"), "got: {d0}");
        assert!(d1.contains("context7"), "got: {d1}");
        assert!(d0.contains("plugins"), "got: {d0}");
        assert!(d1.contains("plugins"), "got: {d1}");
        assert!(d0.contains("update"), "got: {d0}");
        assert!(d1.contains("update"), "got: {d1}");
    }
}
