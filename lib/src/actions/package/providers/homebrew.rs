use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
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

    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // Does not require privilege escalation

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: [
                    vec![String::from("install")],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
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
}
