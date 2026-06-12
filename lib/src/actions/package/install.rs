use super::providers::PackageProviders;
use super::Package;
use super::PackageVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use std::ops::Deref;
use tracing::debug;
use tracing::span;

pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn summarize(&self) -> String {
        let pkgs: Vec<&str> = if let Some(ref name) = self.name {
            vec![name.as_str()]
        } else {
            self.list.iter().map(|s| s.as_str()).collect()
        };
        if pkgs.is_empty() {
            "Installing packages".to_string()
        } else {
            format!("Installing: {}", pkgs.join(", "))
        }
    }

    fn state_key(&self) -> String {
        if let Some(ref name) = self.name {
            name.clone()
        } else {
            self.list.first().cloned().unwrap_or_default()
        }
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: PackageVariant = self.into();

        if variant.version.is_some() && !variant.list.is_empty() {
            return Err(anyhow!(
                "package.install: 'version' cannot be used with 'list'; \
                 pin one package at a time using 'name'"
            ));
        }

        if variant.version.is_some() && variant.cask {
            return Err(anyhow!(
                "package.install: 'version' is not supported with 'cask: true'; \
                 Homebrew casks do not have versioned taps"
            ));
        }

        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.install",
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

            if variant.file {
                match variant.provider {
                    PackageProviders::Aptitude => {
                        debug!("Will attempt to install from local file.")
                    }
                    _ => {
                        return Err(anyhow!(
                        "Package Provider, {}, isn't capabale of local file installs. Skipping action.",
                        provider.name()
                    ));
                    }
                }
            }

            atoms.append(&mut provider.bootstrap(context));
        }

        atoms.append(&mut provider.install(&variant, context)?);

        span.exit();

        Ok(atoms)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: package.install
  name: curl

- action: package.install
  list:
    - bash
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!(vec!["bash"], action.action.list);
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!("curl", action.action.name.unwrap());
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_be_deserialized_with_cask() {
        use crate::actions::Actions;
        let yaml = r#"
- action: package.install
  name: alfred
  provider: homebrew
  cask: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!("alfred", action.action.name.clone().unwrap());
                assert!(action.action.cask);
            }
            _ => panic!("PackageInstall didn't deserialize correctly"),
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_includes_bootstrap_steps_when_provider_unavailable() {
        use super::PackageInstall;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        // Snapcraft is not available on macOS, so this exercises the bootstrap branch
        let pkg = PackageInstall {
            name: Some(String::from("htop")),
            provider: PackageProviders::Snapcraft,
            ..Default::default()
        };
        let steps = pkg
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // bootstrap (1 step) + install (1 step) = 2 steps
        assert!(steps.len() >= 2);
    }

    #[test]
    fn summarize_with_name() {
        use super::PackageInstall;
        use crate::actions::Action;
        let pkg = PackageInstall {
            name: Some(String::from("vim")),
            ..Default::default()
        };
        let s = pkg.summarize();
        assert!(s.contains("vim"), "summary: {s}");
    }

    #[test]
    fn summarize_with_list() {
        use super::PackageInstall;
        use crate::actions::Action;
        let pkg = PackageInstall {
            list: vec![String::from("curl"), String::from("git")],
            ..Default::default()
        };
        let s = pkg.summarize();
        assert!(s.contains("curl"), "summary: {s}");
        assert!(s.contains("git"), "summary: {s}");
    }

    #[test]
    fn summarize_empty_falls_back() {
        use super::PackageInstall;
        use crate::actions::Action;
        let pkg = PackageInstall {
            ..Default::default()
        };
        // no packages set — should not panic
        let s = pkg.summarize();
        assert!(!s.is_empty());
    }

    #[test]
    fn plan_rejects_version_with_list() {
        use super::PackageInstall;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        let pkg = PackageInstall {
            list: vec!["git".to_string(), "curl".to_string()],
            provider: PackageProviders::Homebrew,
            version: Some("2.43".to_string()),
            ..Default::default()
        };
        let result = pkg.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("'version' cannot be used with 'list'"),
            "msg: {msg}"
        );
    }

    #[test]
    fn plan_rejects_version_with_cask() {
        use super::PackageInstall;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        let pkg = PackageInstall {
            name: Some("firefox".to_string()),
            provider: PackageProviders::Homebrew,
            cask: true,
            version: Some("120.0".to_string()),
            ..Default::default()
        };
        let result = pkg.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("'version' is not supported with 'cask: true'"),
            "msg: {msg}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_errors_for_file_install_with_non_aptitude_provider() {
        use super::PackageInstall;
        use crate::actions::package::providers::PackageProviders;
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;

        // Snapcraft is unavailable on macOS; file=true with non-Aptitude should error
        let pkg = PackageInstall {
            name: Some(String::from("htop")),
            provider: PackageProviders::Snapcraft,
            file: true,
            ..Default::default()
        };
        let result = pkg.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
    }
}
