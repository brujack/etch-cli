use super::{check_version, PackageProvider, VersionCheck};
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::process::Command;
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aptitude {}

impl Aptitude {
    fn env(&self) -> Vec<(String, String)> {
        vec![(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        )]
    }
}

impl PackageProvider for Aptitude {
    fn name(&self) -> &str {
        "Aptitude"
    }

    fn available(&self) -> bool {
        match which("apt-add-repository") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "apt-add-repository not available");
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
                    String::from("software-properties-common"),
                    String::from("curl"),
                    String::from("gpg"),
                ],
                environment: self.env(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        repository: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        let mut signed_by = String::from("");

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        if repository.key.is_some() {
            // .unwrap() is safe here because we checked for key.is_some() above
            let key = repository.clone().key.unwrap();

            let key_name = key.name.unwrap_or_else(|| digest(&*key.url));
            let key_path = format!("/usr/share/keyrings/{key_name}.asc");

            signed_by = format!("signed-by={key_path}");

            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("curl"),
                    arguments: vec![String::from("-o"), key_path, key.url],
                    environment: self.env(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        //sudo apt-add-repository "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/<myrepository>-archive-keyring.gpg] https://repository.example.com/debian/ $(lsb_release -cs) stable main "
        steps.extend(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("apt-add-repository"),
                    arguments: vec![
                        String::from("-y"),
                        format!(
                            "deb [arch=$(dpkg --print-architecture) {}] {}",
                            signed_by,
                            repository.name.clone()
                        ),
                    ],
                    environment: self.env(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("apt"),
                    arguments: vec![String::from("update")],
                    environment: self.env(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]);

        Ok(steps)
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
        let out = Command::new("dpkg-query")
            .args(["-W", "-f=${Version}", name])
            .output()?;
        if !out.status.success() {
            return Ok(None);
        }
        let s = String::from_utf8(out.stdout)?.trim().to_owned();
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }

    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let _ = names;
        Ok(vec![]) // stub — full implementation in next task
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        if let Some(ref declared) = package.version {
            let name = package.name.as_deref().unwrap_or_default();
            let installed = self.installed_version(name)?;
            return Self::apt_version_step(
                name,
                declared,
                installed.as_deref(),
                &privilege_provider,
                &self.env(),
            );
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("apt"),
                arguments: vec![String::from("install"), String::from("--yes")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                environment: self.env(),
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

impl Aptitude {
    fn apt_version_step(
        name: &str,
        declared: &str,
        installed: Option<&str>,
        privilege_provider: &str,
        env: &[(String, String)],
    ) -> anyhow::Result<Vec<Step>> {
        match check_version(declared, installed) {
            VersionCheck::Skip => Ok(vec![]),
            VersionCheck::Mismatch { actual } => Err(anyhow!(
                "package {name} is at {actual}, declared version is {declared}; \
                 manually upgrade/downgrade and re-apply"
            )),
            VersionCheck::InstallNeeded => Ok(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("apt-get"),
                    arguments: vec![
                        String::from("install"),
                        String::from("--yes"),
                        format!("{name}={declared}"),
                    ],
                    environment: env.to_vec(),
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
}

#[cfg(test)]
mod test {
    use crate::actions::package::repository::RepositoryKey;
    use crate::contexts::Contexts;

    use super::*;

    // These tests are really weak at the moment, but that's because I'm not
    // sure how to add derive(Debug,Default) to struct Step
    // TODO: Learn how to fix this

    #[test]
    fn test_add_repository_without_key() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_add_repository_with_key() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }

    #[test]
    fn test_add_repository_with_key_and_fingerprint() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    fingerprint: Some(String::from("abc")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }

    #[test]
    fn test_name() {
        let aptitude = Aptitude {};
        assert_eq!(aptitude.name(), "Aptitude");
    }

    #[test]
    fn test_available_returns_bool() {
        let aptitude = Aptitude {};
        // Just verify it doesn't panic — result depends on whether apt-add-repository is installed
        let _ = aptitude.available();
    }

    #[test]
    fn test_bootstrap_returns_one_step() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.bootstrap(&contexts);
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_has_repository_returns_false() {
        let aptitude = Aptitude {};
        let repo = PackageRepository::default();
        assert!(!aptitude.has_repository(&repo));
    }

    #[test]
    fn test_query_returns_packages() {
        use crate::actions::package::PackageVariant;
        let aptitude = Aptitude {};
        let variant = PackageVariant {
            ..Default::default()
        };
        let result = aptitude.query(&variant).unwrap();
        assert!(result.is_empty()); // no packages set
    }

    #[test]
    fn apt_version_step_not_installed_returns_versioned_install() {
        let env = vec![("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string())];
        let steps =
            Aptitude::apt_version_step("git", "1:2.43.0-1ubuntu7", None, "sudo", &env).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("git=1:2.43.0-1ubuntu7"),
            "display: {display}"
        );
    }

    #[test]
    fn apt_version_step_correct_version_returns_empty() {
        let env = vec![];
        let steps = Aptitude::apt_version_step(
            "git",
            "1:2.43.0-1ubuntu7",
            Some("1:2.43.0-1ubuntu7"),
            "sudo",
            &env,
        )
        .unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn apt_version_step_wrong_version_returns_error() {
        let env = vec![];
        let result = Aptitude::apt_version_step(
            "git",
            "1:2.43.0-1ubuntu7",
            Some("1:2.41.0-1ubuntu2"),
            "sudo",
            &env,
        );
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("git"), "msg: {msg}");
        assert!(msg.contains("1:2.41.0-1ubuntu2"), "msg: {msg}");
        assert!(msg.contains("1:2.43.0-1ubuntu7"), "msg: {msg}");
        assert!(msg.contains("manually upgrade/downgrade"), "msg: {msg}");
    }

    #[test]
    fn test_install_returns_one_step() {
        use crate::actions::package::PackageVariant;
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let variant = PackageVariant::default();
        let steps = aptitude.install(&variant, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_regression_share_ring() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    fingerprint: Some(String::from("abc")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        let steps = steps.unwrap_or_default();

        if let Some(step) = steps.first() {
            let exec = step.atom.to_string();
            assert!(exec.contains(" /usr/share/keyrings/"));
        } else {
            panic!("expected at least one step");
        }
    }
}
