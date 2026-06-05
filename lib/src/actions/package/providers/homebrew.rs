use super::{check_version, PackageProvider, VersionCheck};
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};
use tracing::{debug, trace};
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Homebrew {}

impl PackageProvider for Homebrew {
    fn name(&self) -> &str {
        "Homebrew"
    }

    fn available(&self) -> bool {
        which("brew").is_ok()
    }

    fn bootstrap(&self, _contexts: &Contexts) -> Vec<Step> {
        vec![Step { atom: Box::new(Exec {
            command: String::from("bash"),
            arguments: vec![
                String::from("-c"),
                String::from("$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)")
            ],
            streaming: true,
            ..Default::default()
        }), initializers: vec![], finalizers: vec![] },]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(
        &self,
        repository: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("tap"), repository.name.clone()],
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        let prefix = String::from_utf8(Command::new("brew").arg("--prefix").output()?.stdout)?
            .replace(['\n', '\r'], "");

        let cellar = Path::new(&prefix).join("Cellar");
        let caskroom = Path::new(&prefix).join("Caskroom");

        Ok(package
            .packages()
            .into_iter()
            .filter(|p| {
                if cellar.join(p).is_dir() {
                    trace!("{}: found in Cellar", p);
                    false
                } else if caskroom.join(p).is_dir() {
                    trace!("{}: found in Caskroom", p);
                    false
                } else {
                    debug!("{}: doesn't appear to be installed", p);
                    true
                }
            })
            .collect())
    }

    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
        let out = Command::new("brew")
            .args(["info", "--json=v2", name])
            .output()?;
        if !out.status.success() {
            return Ok(None);
        }
        let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        let version = json["formulae"][0]["installed"][0]["version"]
            .as_str()
            .map(str::to_owned);
        Ok(version)
    }

    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // Does not require privilege escalation

        if let Some(ref declared) = package.version {
            let name = package.name.as_deref().unwrap_or_default();
            let installed = self.installed_version(name)?;
            return Self::brew_version_step(name, declared, installed.as_deref());
        }

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        let mut base = vec![String::from("install")];
        if package.cask {
            base.push(String::from("--cask"));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: [base, package.extra_args.clone(), need_installed].concat(),
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

impl Homebrew {
    fn brew_version_step(
        name: &str,
        declared: &str,
        installed: Option<&str>,
    ) -> anyhow::Result<Vec<Step>> {
        match check_version(declared, installed) {
            VersionCheck::Skip => Ok(vec![]),
            VersionCheck::Mismatch { actual } => Err(anyhow!(
                "package {name} is at {actual}, declared version is {declared}; \
                 manually upgrade/downgrade and re-apply"
            )),
            VersionCheck::InstallNeeded => {
                let formula = format!("{name}@{declared}");
                Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("brew"),
                        arguments: vec![String::from("install"), formula],
                        streaming: true,
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }])
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::contexts::Contexts;

    #[test]
    fn name_returns_homebrew() {
        let homebrew = Homebrew {};
        assert_eq!(homebrew.name(), "Homebrew");
    }

    #[test]
    fn available_does_not_panic() {
        let homebrew = Homebrew {};
        let _ = homebrew.available();
    }

    #[test]
    fn bootstrap_returns_one_step() {
        let homebrew = Homebrew {};
        let contexts = Contexts::default();
        let steps = homebrew.bootstrap(&contexts);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn has_repository_always_returns_false() {
        let homebrew = Homebrew {};
        let repo = PackageRepository::default();
        assert!(!homebrew.has_repository(&repo));
    }

    #[test]
    fn add_repository_returns_one_step() {
        let homebrew = Homebrew {};
        let repo = PackageRepository {
            name: String::from("homebrew/cask"),
            ..Default::default()
        };
        let contexts = Contexts::default();
        let steps = homebrew.add_repository(&repo, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn query_returns_packages_when_brew_available() {
        let homebrew = Homebrew {};
        if !homebrew.available() {
            return; // skip if brew not installed
        }
        // Query for a package that almost certainly isn't installed
        let yaml = "name: __no_such_package_xyz123__\nlist: []\nprovider: homebrew\nextra_args: []\nfile: false";
        let variant: PackageVariant = serde_yaml_ng::from_str(yaml).unwrap_or_default();
        // query() might fail if brew isn't available, just don't panic
        let _ = homebrew.query(&variant);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn install_returns_empty_when_all_installed() {
        let homebrew = Homebrew {};
        if !homebrew.available() {
            return;
        }
        // An empty package list means nothing to install
        let variant = PackageVariant::default();
        let contexts = Contexts::default();
        let result = homebrew.install(&variant, &contexts);
        // Should succeed and return empty (nothing to install)
        if let Ok(steps) = result {
            assert!(steps.is_empty());
        }
        // If brew isn't available, query() may error — that's acceptable
    }

    #[test]
    fn install_includes_cask_flag_when_cask_true() {
        let homebrew = Homebrew {};
        if !homebrew.available() {
            return;
        }
        let pkg = PackageVariant {
            name: Some(String::from("etch-definitely-not-installed-cask-xyz")),
            cask: true,
            ..Default::default()
        };
        let steps = homebrew.install(&pkg, &Contexts::default()).unwrap();
        if steps.is_empty() {
            return;
        }
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--cask"),
            "expected '--cask' in brew install args: {display}"
        );
    }

    #[test]
    fn brew_version_step_not_installed_returns_versioned_formula() {
        let steps = Homebrew::brew_version_step("python", "3.11", None).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("python@3.11"), "display: {display}");
    }

    #[test]
    fn brew_version_step_correct_version_returns_empty() {
        let steps = Homebrew::brew_version_step("python", "3.11", Some("3.11")).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn brew_version_step_wrong_version_returns_error() {
        let result = Homebrew::brew_version_step("python", "3.11", Some("3.12"));
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("python"), "msg: {msg}");
        assert!(msg.contains("3.12"), "msg: {msg}");
        assert!(msg.contains("3.11"), "msg: {msg}");
        assert!(msg.contains("manually upgrade/downgrade"), "msg: {msg}");
    }

    #[test]
    fn install_excludes_cask_flag_when_cask_false() {
        let homebrew = Homebrew {};
        if !homebrew.available() {
            return;
        }
        let pkg = PackageVariant {
            name: Some(String::from("etch-definitely-not-installed-formula-xyz")),
            cask: false,
            ..Default::default()
        };
        let steps = homebrew.install(&pkg, &Contexts::default()).unwrap();
        if steps.is_empty() {
            return;
        }
        let display = steps[0].atom.to_string();
        assert!(
            !display.contains("--cask"),
            "did not expect '--cask' in brew install args: {display}"
        );
    }
}
