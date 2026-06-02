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
}

#[cfg(test)]
mod tests {
    use super::*;

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
