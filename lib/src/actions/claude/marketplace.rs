use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplace {
    pub name: String,
    pub source: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub sparse: Vec<String>,
}

impl ClaudeMarketplace {
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

    fn build_step(source: &str, scope: Option<&str>, sparse: &[String]) -> Step {
        use crate::atoms::command::Exec;
        let mut args = vec![
            String::from("plugins"),
            String::from("marketplace"),
            String::from("add"),
            source.to_string(),
        ];
        if let Some(s) = scope {
            args.push(String::from("--scope"));
            args.push(s.to_string());
        }
        if !sparse.is_empty() {
            args.push(String::from("--sparse"));
            args.extend(sparse.iter().cloned());
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

impl Action for ClaudeMarketplace {
    fn summarize(&self) -> String {
        format!("Adding Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.name.is_empty() || self.source.is_empty() {
            bail!("claude.marketplace requires 'name' and 'source'");
        }
        let installed = Self::installed_marketplaces();
        if installed.contains(&self.name) {
            return Ok(vec![]);
        }
        Ok(vec![Self::build_step(
            &self.source,
            self.scope.as_deref(),
            &self.sparse,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marketplace(name: &str, source: &str) -> ClaudeMarketplace {
        ClaudeMarketplace {
            name: name.to_string(),
            source: source.to_string(),
            scope: None,
            sparse: vec![],
        }
    }

    #[test]
    fn plan_skips_when_marketplace_already_present() {
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(display.contains("marketplace"), "got: {display}");
        assert!(display.contains("add"), "got: {display}");
        assert!(display.contains("owner/repo"), "got: {display}");
    }

    #[test]
    fn build_step_omits_scope_when_none() {
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(!display.contains("--scope"), "got: {display}");
    }

    #[test]
    fn build_step_includes_scope_when_set() {
        let step = ClaudeMarketplace::build_step("owner/repo", Some("user"), &[]);
        let display = step.atom.to_string();
        assert!(display.contains("--scope"), "got: {display}");
        assert!(display.contains("user"), "got: {display}");
    }

    #[test]
    fn build_step_includes_sparse_when_set() {
        let step =
            ClaudeMarketplace::build_step("owner/repo", None, &[String::from(".claude-plugin")]);
        let display = step.atom.to_string();
        assert!(display.contains("--sparse"), "got: {display}");
        assert!(display.contains(".claude-plugin"), "got: {display}");
    }

    #[test]
    fn build_step_omits_sparse_when_empty() {
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(!display.contains("--sparse"), "got: {display}");
    }

    #[test]
    fn summarize_includes_name() {
        let m = marketplace("caveman", "juliusbrussee/caveman");
        assert!(m.summarize().contains("caveman"));
    }

    #[test]
    fn deserialize_minimal() {
        let yaml = "name: caveman\nsource: juliusbrussee/caveman\n";
        let m: ClaudeMarketplace = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.name, "caveman");
        assert_eq!(m.source, "juliusbrussee/caveman");
        assert!(m.scope.is_none());
        assert!(m.sparse.is_empty());
    }

    #[test]
    fn deserialize_with_scope_and_sparse() {
        let yaml = "name: caveman\nsource: juliusbrussee/caveman\nscope: user\nsparse:\n  - .claude-plugin\n";
        let m: ClaudeMarketplace = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.scope, Some(String::from("user")));
        assert_eq!(m.sparse, vec![".claude-plugin"]);
    }
}
