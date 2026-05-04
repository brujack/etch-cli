use super::{ManifestProvider, ManifestProviderError};

use gix;
use gix::interrupt;
use gix::progress::Discard;

use dirs_next;

use tracing::info;

#[derive(Debug)]
pub struct GitManifestProvider;

#[derive(Debug, PartialEq)]
pub(crate) struct GitConfig {
    repository: String,
    branch: Option<String>,
    path: Option<String>,
}

impl ManifestProvider for GitManifestProvider {
    fn looks_familiar(&self, url: &str) -> bool {
        use regex::Regex;

        if let Ok(regex) = Regex::new(r"^(https|git|ssh)://") {
            regex.is_match(url)
        } else {
            false
        }
    }

    fn resolve(
        &self,
        url: &str,
    ) -> anyhow::Result<std::path::PathBuf, super::ManifestProviderError> {
        let config = self.parse_config_url(url);
        let clean_url = self.clean_git_url(&config.repository);
        let cache_path = dirs_next::cache_dir()
            .ok_or(ManifestProviderError::NoResolution)?
            .join("etch")
            .join("manifests")
            .join("git")
            .join(clean_url);
        if !cache_path.exists() {
            self.fetch_and_clone(&cache_path, &config)?;
        }

        Ok(cache_path)
    }
}

impl GitManifestProvider {
    fn fetch_and_clone(
        &self,
        cache_path: &std::path::Path,
        config: &GitConfig,
    ) -> anyhow::Result<(), super::ManifestProviderError> {
        info!("Preparing to fetch and clone manifests.");
        let r = std::fs::create_dir_all(cache_path);
        if r.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = gix::url::parse(config.repository.clone().as_str().into());

        if url.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = url.unwrap();

        unsafe {
            let handler = interrupt::init_handler(1, || {});
            if handler.is_err() {
                return Err(ManifestProviderError::NoResolution);
            }
        };

        let prepare_clone = gix::prepare_clone(url.clone(), cache_path);
        if prepare_clone.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_clone = prepare_clone.unwrap();

        let prepare_checkout =
            prepare_clone.fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED);
        if prepare_checkout.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_checkout = prepare_checkout.unwrap().0;

        let repo = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED);
        if repo.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let repo = repo.unwrap().0;

        let success = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .expect("always present after clone");
        if success.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        info!("Finished fetch and clone operation.");

        Ok(())
    }

    fn parse_config_url(&self, uri: &str) -> GitConfig {
        let (repository, parts) = match uri.split_once('#') {
            Some(parts) => parts,
            None => {
                return GitConfig {
                    repository: String::from(uri),
                    branch: None,
                    path: None,
                };
            }
        };

        let (reference, path) = match parts.split_once(':') {
            Some(("", path)) => (None, Some(path.to_string())),
            Some((reference, "")) => (Some(reference.to_string()), None),
            Some((reference, path)) => (Some(reference.to_string()), Some(path.to_string())),
            None => {
                return GitConfig {
                    repository: String::from(repository),
                    branch: Some(parts.to_string()),
                    path: None,
                }
            }
        };

        GitConfig {
            repository: String::from(repository),
            branch: reference,
            path,
        }
    }

    fn clean_git_url(&self, uri: &str) -> String {
        uri.to_string()
            .replace("https", "")
            .replace("http", "")
            .replace([':', '.', '/'], "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::providers::ManifestProvider;

    fn provider() -> GitManifestProvider {
        GitManifestProvider
    }

    // --- looks_familiar ---

    #[test]
    fn looks_familiar_https_url_returns_true() {
        assert!(provider().looks_familiar("https://github.com/user/repo"));
    }

    #[test]
    fn looks_familiar_git_url_returns_true() {
        assert!(provider().looks_familiar("git://github.com/user/repo"));
    }

    #[test]
    fn looks_familiar_ssh_url_returns_true() {
        assert!(provider().looks_familiar("ssh://git@github.com/user/repo"));
    }

    #[test]
    fn looks_familiar_local_path_returns_false() {
        assert!(!provider().looks_familiar("/local/path/to/manifests"));
    }

    #[test]
    fn looks_familiar_relative_path_returns_false() {
        assert!(!provider().looks_familiar("../relative/path"));
    }

    #[test]
    fn looks_familiar_empty_string_returns_false() {
        assert!(!provider().looks_familiar(""));
    }

    #[test]
    fn looks_familiar_scp_style_returns_false() {
        // git@github.com:user/repo has no :// so is not recognized
        assert!(!provider().looks_familiar("git@github.com:user/repo"));
    }

    // --- parse_config_url ---

    #[test]
    fn parse_config_url_no_fragment_returns_repository_only() {
        let config = provider().parse_config_url("https://github.com/user/repo");
        assert_eq!(
            config,
            GitConfig {
                repository: "https://github.com/user/repo".into(),
                branch: None,
                path: None,
            }
        );
    }

    #[test]
    fn parse_config_url_fragment_with_branch_only() {
        let config = provider().parse_config_url("https://github.com/user/repo#main");
        assert_eq!(
            config,
            GitConfig {
                repository: "https://github.com/user/repo".into(),
                branch: Some("main".into()),
                path: None,
            }
        );
    }

    #[test]
    fn parse_config_url_fragment_with_path_only() {
        // #:manifests → empty branch, path present
        let config = provider().parse_config_url("https://github.com/user/repo#:manifests");
        assert_eq!(
            config,
            GitConfig {
                repository: "https://github.com/user/repo".into(),
                branch: None,
                path: Some("manifests".into()),
            }
        );
    }

    #[test]
    fn parse_config_url_fragment_with_branch_and_path() {
        let config = provider().parse_config_url("https://github.com/user/repo#main:manifests");
        assert_eq!(
            config,
            GitConfig {
                repository: "https://github.com/user/repo".into(),
                branch: Some("main".into()),
                path: Some("manifests".into()),
            }
        );
    }

    #[test]
    fn parse_config_url_fragment_with_branch_and_empty_path() {
        // #main: → branch present, empty path becomes None
        let config = provider().parse_config_url("https://github.com/user/repo#main:");
        assert_eq!(
            config,
            GitConfig {
                repository: "https://github.com/user/repo".into(),
                branch: Some("main".into()),
                path: None,
            }
        );
    }

    // --- clean_git_url ---

    #[test]
    fn clean_git_url_strips_https_and_special_chars() {
        let result = provider().clean_git_url("https://github.com/user/repo");
        assert_eq!(result, "githubcomuserrepo");
    }

    #[test]
    fn clean_git_url_strips_git_protocol_and_special_chars() {
        let result = provider().clean_git_url("git://github.com/user/repo");
        assert_eq!(result, "gitgithubcomuserrepo");
    }

    #[test]
    fn clean_git_url_strips_ssh_protocol_and_special_chars() {
        let result = provider().clean_git_url("ssh://github.com/user/repo");
        assert_eq!(result, "sshgithubcomuserrepo");
    }

    // --- resolve (cache-hit path — no network required) ---

    #[test]
    fn resolve_returns_cache_path_when_already_cloned() {
        let p = provider();
        let url = "https://github.com/user/repo-cache-hit-test";
        let clean = p.clean_git_url("https://github.com/user/repo-cache-hit-test");
        let cache_path = dirs_next::cache_dir()
            .unwrap()
            .join("etch")
            .join("manifests")
            .join("git")
            .join(&clean);

        std::fs::create_dir_all(&cache_path).unwrap();

        let result = p.resolve(url);

        std::fs::remove_dir_all(&cache_path).unwrap();

        assert_eq!(result.unwrap(), cache_path);
    }
}
