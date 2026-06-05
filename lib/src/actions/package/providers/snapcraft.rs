use super::{check_version, PackageProvider, VersionCheck};
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec, utilities};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::process::Command;
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapcraft {}

impl PackageProvider for Snapcraft {
    fn name(&self) -> &str {
        "Snapcraft"
    }

    fn available(&self) -> bool {
        match which("snap") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "snap is not available");
                false
            }
        }
    }

    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        vec![Step {
            atom: Box::new(Exec {
                command: String::from("apt"),
                arguments: vec![
                    String::from("install"),
                    String::from("--yes"),
                    String::from("snapd"),
                ],
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }

    fn has_repository(&self, _package: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        _package: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
        let out = Command::new("snap").args(["list", name]).output()?;
        if !out.status.success() {
            return Ok(None);
        }
        let text = String::from_utf8(out.stdout)?;
        Ok(Self::parse_snap_tracking(&text))
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        if let Some(ref declared) = package.version {
            let name = package.name.as_deref().unwrap_or_default();
            let installed = self.installed_version(name)?;
            return Self::snap_version_step(
                name,
                declared,
                installed.as_deref(),
                &privilege_provider,
            );
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("snap"),
                arguments: vec![String::from("install"), String::from("--yes")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

impl Snapcraft {
    fn snap_version_step(
        name: &str,
        declared: &str,
        installed: Option<&str>,
        privilege_provider: &str,
    ) -> anyhow::Result<Vec<Step>> {
        match check_version(declared, installed) {
            VersionCheck::Skip => Ok(vec![]),
            VersionCheck::Mismatch { actual } => Err(anyhow!(
                "package {name} is tracking channel {actual}, declared version is {declared}; \
                 manually upgrade/downgrade and re-apply"
            )),
            VersionCheck::InstallNeeded => Ok(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("snap"),
                    arguments: vec![
                        String::from("install"),
                        format!("--channel={declared}"),
                        name.to_string(),
                    ],
                    privileged: true,
                    privilege_provider: privilege_provider.to_string(),
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]),
        }
    }

    /// Parse the Tracking column from `snap list <name>` output.
    /// Exposed for unit testing the output parsing logic.
    pub(crate) fn parse_snap_tracking(output: &str) -> Option<String> {
        output
            .lines()
            .nth(1)
            .and_then(|l| l.split_whitespace().nth(3))
            .map(str::to_owned)
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::providers::PackageProviders;
    use crate::contexts::Contexts;

    use super::*;

    #[test]
    fn test_install() {
        let snapcraft = Snapcraft {};
        let contexts = Contexts::default();
        let steps = snapcraft.install(
            &PackageVariant {
                name: Some(String::from("")),
                list: vec![],
                extra_args: vec![],
                provider: PackageProviders::Snapcraft,
                file: false,
                cask: false,
                version: None,
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn available_does_not_panic() {
        let snapcraft = Snapcraft {};
        let _ = snapcraft.available();
    }

    #[test]
    fn bootstrap_returns_one_step() {
        let snapcraft = Snapcraft {};
        let contexts = Contexts::default();
        let steps = snapcraft.bootstrap(&contexts);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn has_repository_always_returns_false() {
        let snapcraft = Snapcraft {};
        let repo = crate::actions::package::repository::PackageRepository::default();
        assert!(!snapcraft.has_repository(&repo));
    }

    #[test]
    fn name_returns_snapcraft() {
        let snapcraft = Snapcraft {};
        assert_eq!(snapcraft.name(), "Snapcraft");
    }

    #[test]
    fn query_returns_all_packages() {
        let snapcraft = Snapcraft {};
        let package = PackageVariant {
            name: Some(String::from("htop")),
            list: vec![],
            extra_args: vec![],
            provider: PackageProviders::Snapcraft,
            file: false,
            cask: false,
            version: None,
        };
        let packages = snapcraft.query(&package).unwrap();
        assert_eq!(packages.len(), 1);
    }

    #[test]
    fn parse_snap_tracking_extracts_channel_column() {
        let output = "Name  Version  Rev  Tracking        Publisher  Notes\n\
                      code  1.89.0   172  latest/stable   vscode     -\n";
        let channel = Snapcraft::parse_snap_tracking(output);
        assert_eq!(channel, Some("latest/stable".to_string()));
    }

    #[test]
    fn parse_snap_tracking_returns_none_for_header_only() {
        let output = "Name  Version  Rev  Tracking  Publisher  Notes\n";
        let channel = Snapcraft::parse_snap_tracking(output);
        assert_eq!(channel, None);
    }

    #[test]
    fn snap_version_step_not_installed_returns_channel_install() {
        let steps = Snapcraft::snap_version_step("code", "latest/stable", None, "sudo").unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--channel=latest/stable"),
            "display: {display}"
        );
        assert!(display.contains("code"), "display: {display}");
    }

    #[test]
    fn snap_version_step_correct_channel_returns_empty() {
        let steps =
            Snapcraft::snap_version_step("code", "latest/stable", Some("latest/stable"), "sudo")
                .unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn snap_version_step_wrong_channel_returns_error() {
        let result =
            Snapcraft::snap_version_step("code", "latest/stable", Some("latest/edge"), "sudo");
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("code"), "msg: {msg}");
        assert!(msg.contains("latest/edge"), "msg: {msg}");
        assert!(msg.contains("latest/stable"), "msg: {msg}");
        assert!(msg.contains("manually upgrade/downgrade"), "msg: {msg}");
    }
}
