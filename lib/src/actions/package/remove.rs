use super::providers::PackageProviders;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageRemove {
    pub name: Option<String>,

    #[serde(default)]
    pub list: Vec<String>,

    #[serde(default)]
    pub provider: PackageProviders,

    #[serde(default)]
    pub purge: bool,
}

impl PackageRemove {
    fn packages(&self) -> Vec<String> {
        self.name
            .as_ref()
            .map(|n| vec![n.clone()])
            .unwrap_or_else(|| self.list.clone())
    }
}

impl Action for PackageRemove {
    fn summarize(&self) -> String {
        let pkgs = self.packages();
        if pkgs.is_empty() {
            return String::from("Removing packages");
        }
        format!("Removing package(s): {}", pkgs.join(", "))
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let pkgs = self.packages();
        if pkgs.is_empty() {
            bail!("package.remove requires either 'name' or 'list'");
        }
        let provider = self.provider.clone().get_provider();
        provider.deref().remove(&pkgs, self.purge, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_errors_when_no_name_or_list() {
        let action = PackageRemove::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("requires either 'name' or 'list'"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn plan_skips_package_not_installed_apt() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Aptitude,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled package");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn plan_skips_package_not_installed_snap() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Snapcraft,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled snap");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_skips_package_not_installed_homebrew() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Homebrew,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled formula");
    }

    #[test]
    fn summarize_includes_package_name() {
        let action = PackageRemove {
            name: Some(String::from("nginx")),
            ..Default::default()
        };
        assert!(action.summarize().contains("nginx"));
    }

    #[test]
    fn summarize_includes_list_names() {
        let action = PackageRemove {
            list: vec![String::from("htop"), String::from("curl")],
            ..Default::default()
        };
        let summary = action.summarize();
        assert!(summary.contains("htop"));
        assert!(summary.contains("curl"));
    }

    #[test]
    fn deserialize_name_form() {
        let yaml = "name: htop\nprovider: apt\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.name, Some(String::from("htop")));
        assert!(!action.purge);
    }

    #[test]
    fn deserialize_list_form() {
        let yaml = "list:\n  - htop\n  - curl\nprovider: apt\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.list, vec!["htop", "curl"]);
    }

    #[test]
    fn deserialize_purge_true() {
        let yaml = "name: nginx\nprovider: apt\npurge: true\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(action.purge);
    }
}
