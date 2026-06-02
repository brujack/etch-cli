use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::providers::{apt_upgrade, snap_upgrade};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageUpgrade {
    pub provider: String,
    pub name: Option<String>,
    pub list: Option<Vec<String>>,
}

impl PackageUpgrade {
    pub fn upgradeable(&self) -> bool {
        true
    }
}

impl Action for PackageUpgrade {
    fn summarize(&self) -> String {
        match (&self.name, &self.list) {
            (Some(name), _) => format!("Upgrading {}: {name}", self.provider),
            (_, Some(list)) if !list.is_empty() => {
                format!("Upgrading {} packages: {}", self.provider, list.join(", "))
            }
            _ => format!("Upgrading all {} packages", self.provider),
        }
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        if matches!(self.provider.as_str(), "homebrew" | "brew") {
            bail!("use brew.upgrade directly for Homebrew upgrades");
        }
        if self.name.is_some() && self.list.is_some() {
            bail!("name and list are mutually exclusive");
        }
        match self.provider.as_str() {
            "apt" | "aptitude" | "apt-get" => {
                apt_upgrade::plan(self.name.as_deref(), self.list.as_deref(), context)
            }
            "snap" | "snapcraft" => snap_upgrade::plan(self.name.as_deref(), context),
            p => bail!("unknown provider: {p}; supported: apt, snap"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized_with_provider_only() {
        let yaml = r#"
- action: package.upgrade
  provider: apt
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageUpgrade(a)) => {
                assert_eq!("apt", a.action.provider);
                assert!(a.action.name.is_none());
                assert!(a.action.list.is_none());
            }
            _ => panic!("PackageUpgrade didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_name() {
        let yaml = r#"
- action: package.upgrade
  provider: apt
  name: git
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageUpgrade(a)) => {
                assert_eq!(Some("git".to_string()), a.action.name);
                assert!(a.action.list.is_none());
            }
            _ => panic!("PackageUpgrade didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = r#"
- action: package.upgrade
  provider: apt
  list: [git, curl, vim]
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageUpgrade(a)) => {
                assert!(a.action.name.is_none());
                assert_eq!(
                    Some(vec![
                        "git".to_string(),
                        "curl".to_string(),
                        "vim".to_string()
                    ]),
                    a.action.list
                );
            }
            _ => panic!("PackageUpgrade didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn upgradeable_returns_true() {
        let action = PackageUpgrade {
            provider: "apt".to_string(),
            ..Default::default()
        };
        assert!(action.upgradeable());
    }

    #[test]
    fn plan_errors_for_homebrew_provider() {
        let action = PackageUpgrade {
            provider: "homebrew".to_string(),
            ..Default::default()
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("brew.upgrade"),
            "expected 'brew.upgrade' in error: {msg}"
        );
    }

    #[test]
    fn plan_errors_for_brew_alias() {
        let action = PackageUpgrade {
            provider: "brew".to_string(),
            ..Default::default()
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("brew.upgrade"),
            "expected 'brew.upgrade': {msg}"
        );
    }

    #[test]
    fn plan_errors_when_name_and_list_both_set() {
        let action = PackageUpgrade {
            provider: "apt".to_string(),
            name: Some("git".to_string()),
            list: Some(vec!["curl".to_string()]),
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("mutually exclusive"),
            "expected 'mutually exclusive': {msg}"
        );
    }

    #[test]
    fn plan_errors_for_unknown_provider() {
        let action = PackageUpgrade {
            provider: "winget".to_string(),
            ..Default::default()
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("unknown provider"),
            "expected 'unknown provider': {msg}"
        );
        assert!(msg.contains("winget"), "expected provider name: {msg}");
    }

    #[test]
    fn summarize_with_name() {
        let action = PackageUpgrade {
            provider: "apt".to_string(),
            name: Some("git".to_string()),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("git"), "expected 'git': {s}");
    }

    #[test]
    fn summarize_with_list() {
        let action = PackageUpgrade {
            provider: "apt".to_string(),
            list: Some(vec!["git".to_string(), "curl".to_string()]),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("git"), "expected 'git': {s}");
        assert!(s.contains("curl"), "expected 'curl': {s}");
    }

    #[test]
    fn summarize_no_filter_includes_provider() {
        let action = PackageUpgrade {
            provider: "apt".to_string(),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("apt"), "expected 'apt': {s}");
    }
}
