use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug)]
pub struct ToolsCheck;

fn action_implied_tool(action: &Actions) -> Option<&'static str> {
    match action {
        Actions::BrewBundle(_) | Actions::BrewUpgrade(_) | Actions::BrewCleanup(_) => Some("brew"),
        Actions::GemInstall(_) => Some("gem"),
        Actions::PipInstall(_) => Some("pip"),
        Actions::NpmInstall(_) => Some("npm"),
        Actions::MasInstall(_) | Actions::MasUpgrade(_) => Some("mas"),
        Actions::PyenvInstall(_) | Actions::PyenvVirtualenv(_) => Some("pyenv"),
        Actions::RubyInstall(_) => Some("ruby-install"),
        Actions::ClaudeInstall(_) | Actions::ClaudeUpgrade(_) | Actions::ClaudePluginUpdate(_) => {
            Some("claude")
        }
        _ => None,
    }
}

impl DoctorCheck for ToolsCheck {
    fn name(&self) -> &'static str {
        "Tools"
    }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut tools: BTreeSet<String> = BTreeSet::new();

        for manifest in manifests.values() {
            for action in &manifest.actions {
                if let Some(tool) = action_implied_tool(action) {
                    tools.insert(tool.to_string());
                }
            }
        }

        if let Some(ref doctor) = config.doctor {
            for tool in &doctor.tools {
                tools.insert(tool.clone());
            }
        }

        tools
            .into_iter()
            .map(|tool| {
                let found = which::which(&tool).is_ok();
                CheckResult {
                    label: tool.clone(),
                    passed: found,
                    detail: if found {
                        None
                    } else {
                        Some(String::from("not found in PATH"))
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DoctorConfig;
    use serial_test::serial;

    fn config_with_tools(tools: &[&str]) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                tools: tools.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    #[serial]
    fn passes_for_tool_in_path() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("mytestbinary_xyz");
        std::fs::write(&fake, b"#!/bin/sh").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_tools(&["mytestbinary_xyz"]);
        let results = ToolsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(results[0].passed, "expected pass for tool in PATH");
    }

    #[test]
    fn fails_for_tool_not_in_path() {
        let config = config_with_tools(&["etch_cli_nonexistent_tool_xyz"]);
        let results = ToolsCheck.run(&config, &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0]
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("not found"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn explicit_and_manifest_derived_merged_and_deduped() {
        let yaml = "actions:\n  - action: brew.bundle\n    file: /tmp/Brewfile\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);

        let config = config_with_tools(&["brew"]);
        let results = ToolsCheck.run(&config, &manifests);

        let brew_results: Vec<_> = results.iter().filter(|r| r.label == "brew").collect();
        assert_eq!(1, brew_results.len(), "expected brew deduped to one entry");
    }

    #[test]
    fn gem_install_implies_gem() {
        let yaml = "actions:\n  - action: gem.install\n    name: bundler\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);
        let results = ToolsCheck.run(&Config::default(), &manifests);
        assert!(
            results.iter().any(|r| r.label == "gem"),
            "expected gem in results"
        );
    }

    #[test]
    fn empty_manifests_and_empty_config_returns_empty() {
        let results = ToolsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
