use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplaceRemove {
    pub name: String,
    pub scope: Option<String>,
}

impl ClaudeMarketplaceRemove {
    fn installed_marketplaces() -> Vec<String> {
        std::process::Command::new("claude")
            .args(["plugins", "marketplace", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_marketplace_list(&stdout)
            })
            .unwrap_or_default()
    }

    fn build_step(name: &str, scope: Option<&str>) -> Step {
        use crate::atoms::command::Exec;
        let mut args = vec![
            String::from("plugins"),
            String::from("marketplace"),
            String::from("remove"),
            name.to_string(),
        ];
        if let Some(s) = scope {
            args.push(String::from("--scope"));
            args.push(s.to_string());
        }
        Step {
            atom: Box::new(Exec {
                command: String::from("claude"),
                arguments: args,
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
}

impl Action for ClaudeMarketplaceRemove {
    fn summarize(&self) -> String {
        format!("Removing Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let installed = Self::installed_marketplaces();
        if !installed.contains(&self.name) {
            return Ok(vec![]);
        }
        Ok(vec![Self::build_step(&self.name, self.scope.as_deref())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_step_contains_remove_and_name() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", None);
        let display = step.atom.to_string();
        assert!(display.contains("marketplace"), "got: {display}");
        assert!(display.contains("remove"), "got: {display}");
        assert!(display.contains("caveman"), "got: {display}");
    }

    #[test]
    fn build_step_omits_scope_when_none() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", None);
        let display = step.atom.to_string();
        assert!(!display.contains("--scope"), "got: {display}");
    }

    #[test]
    fn build_step_includes_scope_when_set() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", Some("user"));
        let display = step.atom.to_string();
        assert!(display.contains("--scope"), "got: {display}");
        assert!(display.contains("user"), "got: {display}");
    }

    #[test]
    fn summarize_includes_name() {
        let r = ClaudeMarketplaceRemove {
            name: String::from("caveman"),
            scope: None,
        };
        assert!(r.summarize().contains("caveman"));
    }

    #[test]
    fn deserialize_minimal() {
        let yaml = "name: caveman\n";
        let r: ClaudeMarketplaceRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.name, "caveman");
        assert!(r.scope.is_none());
    }

    #[test]
    fn deserialize_with_scope() {
        let yaml = "name: caveman\nscope: user\n";
        let r: ClaudeMarketplaceRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.scope, Some(String::from("user")));
    }
}
