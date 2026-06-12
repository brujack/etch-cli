use super::providers::PackageProviders;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tracing::span;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageRepository {
    #[serde(alias = "url")]
    pub name: String,

    pub key: Option<RepositoryKey>,

    #[serde(default)]
    pub provider: PackageProviders,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositoryKey {
    pub url: String,
    pub name: Option<String>,
    pub key: Option<String>,
    pub fingerprint: Option<String>,
}

impl Action for PackageRepository {
    fn summarize(&self) -> String {
        format!("Adding repository {}", self.name)
    }

    fn state_key(&self) -> String {
        self.name.clone()
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let box_provider = self.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.repository",
            provider = provider.name()
        )
        .entered();

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() {
            if provider.bootstrap(context).is_empty() {
                return Err(anyhow!(
                    "Package Provider, {}, isn't available. Skipping action",
                    provider.name()
                ));
            }

            atoms.append(&mut provider.bootstrap(context));
        }

        if !provider.has_repository(self) {
            atoms.append(&mut provider.add_repository(self, context)?);
        }

        span.exit();

        Ok(atoms)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn package_repository_can_be_deserialized() {
        let yaml = r#"
- action: package.repository
  name: https://download.docker.com/linux/ubuntu focal stable
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageRepository(action)) => {
                assert!(!action.action.name.is_empty());
            }
            _ => panic!("Expected PackageRepository"),
        }
    }

    #[test]
    fn package_repository_with_key() {
        let yaml = r#"
- action: package.repository
  name: deb https://example.com stable main
  key:
    url: https://example.com/key.gpg
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageRepository(action)) => {
                assert!(action.action.key.is_some());
            }
            _ => panic!("Expected PackageRepository"),
        }
    }

    #[test]
    fn package_repository_summarize() {
        use super::PackageRepository;
        use crate::actions::Action;

        let repo = PackageRepository {
            name: "myrepo".to_string(),
            ..Default::default()
        };
        assert!(repo.summarize().contains("myrepo"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_includes_bootstrap_steps_when_provider_unavailable() {
        use super::PackageRepository;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        // Snapcraft is not available on macOS → bootstrap branch runs
        // has_repository always returns false → add_repository branch runs (returns empty for Snapcraft)
        let repo = PackageRepository {
            name: String::from("stable"),
            provider: PackageProviders::Snapcraft,
            key: None,
        };
        let steps = repo
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // bootstrap step (1) + add_repository (0 for Snapcraft) = at least 1
        assert!(!steps.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_with_available_provider_adds_repository() {
        use super::PackageRepository;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        // Homebrew is available on macOS; has_repository always returns false for Homebrew
        // so this exercises: provider available (no bootstrap) → add_repository
        let repo = PackageRepository {
            name: String::from("homebrew/cask"),
            provider: PackageProviders::Homebrew,
            key: None,
        };
        let steps = repo
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // Homebrew available → no bootstrap; add_repository returns 1 step (brew tap)
        assert_eq!(1, steps.len());
    }
}
