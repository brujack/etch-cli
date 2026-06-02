mod aptitude;
use self::aptitude::Aptitude;
mod homebrew;
use self::homebrew::Homebrew;
mod snapcraft;
use self::snapcraft::Snapcraft;
pub(crate) mod apt_upgrade;
pub(crate) mod snap_upgrade;
use super::{repository::PackageRepository, PackageVariant};
use crate::contexts::Contexts;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(rename = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,

    #[serde(rename = "homebrew", alias = "brew")]
    Homebrew,

    #[serde(rename = "snapcraft", alias = "snap")]
    Snapcraft,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::Homebrew => Box::new(Homebrew {}),
            PackageProviders::Snapcraft => Box::new(Snapcraft {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        tracing::debug!("OS info: {info:?}");

        match info.os_type() {
            os_info::Type::Ubuntu => PackageProviders::Aptitude,
            os_info::Type::Macos => PackageProviders::Homebrew,
            _ => panic!("Unsupported OS. Use provider: apt, snap, or brew explicitly."),
        }
    }
}

pub trait PackageProvider {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step>;
    fn has_repository(&self, package: &PackageRepository) -> bool;
    fn add_repository(
        &self,
        package: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>>;
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>>;
    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
    /// Returns the installed version of a single named package, or None if not installed.
    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>>;
}

/// Outcome of comparing a declared version against what is currently installed.
pub(super) enum VersionCheck {
    /// Installed version matches declared — skip, emit no steps.
    Skip,
    /// Package is not installed — proceed to install at declared version.
    InstallNeeded,
    /// Wrong version installed — operator must resolve manually.
    Mismatch { actual: String },
}

pub(super) fn check_version(declared: &str, installed: Option<&str>) -> VersionCheck {
    match installed {
        None => VersionCheck::InstallNeeded,
        Some(actual) if actual == declared => VersionCheck::Skip,
        Some(actual) => VersionCheck::Mismatch {
            actual: actual.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_version_not_installed_returns_install_needed() {
        assert!(matches!(
            check_version("3.11", None),
            VersionCheck::InstallNeeded
        ));
    }

    #[test]
    fn check_version_correct_returns_skip() {
        assert!(matches!(
            check_version("3.11", Some("3.11")),
            VersionCheck::Skip
        ));
    }

    #[test]
    fn check_version_wrong_returns_mismatch() {
        match check_version("3.11", Some("3.12")) {
            VersionCheck::Mismatch { actual } => assert_eq!(actual, "3.12"),
            _ => panic!("expected Mismatch"),
        }
    }

    #[test]
    fn homebrew_get_provider_returns_homebrew() {
        let provider = PackageProviders::Homebrew.get_provider();
        assert_eq!("Homebrew", provider.name());
    }

    #[test]
    fn snapcraft_get_provider_returns_snapcraft() {
        let provider = PackageProviders::Snapcraft.get_provider();
        assert_eq!("Snapcraft", provider.name());
    }

    #[test]
    fn aptitude_get_provider_returns_aptitude() {
        let provider = PackageProviders::Aptitude.get_provider();
        assert_eq!("Aptitude", provider.name());
    }
}
