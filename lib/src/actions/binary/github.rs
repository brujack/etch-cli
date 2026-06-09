use crate::actions::Action;
use crate::atoms::file::{Chmod, SetContents};
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tracing::debug;

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    tag: String,
    fetched_at: u64,
}

fn read_cache(cache_path: &Path) -> Option<String> {
    let data = fs::read_to_string(cache_path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(entry.fetched_at) < 3600 {
        Some(entry.tag)
    } else {
        None
    }
}

fn write_cache(cache_path: &Path, tag: &str) -> anyhow::Result<()> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let entry = CacheEntry {
        tag: tag.to_string(),
        fetched_at: now,
    };
    fs::write(cache_path, serde_json::to_string(&entry)?)?;
    Ok(())
}

fn normalize_version(v: &str) -> &str {
    v.strip_prefix('v').unwrap_or(v)
}

pub(crate) struct BinaryGitHubStatus {
    pub name: String,
    pub directory: String,
    pub owner: String,
    pub repo: String,
    pub version: String,
    /// Override the cache directory. `None` uses `~/.cache/etch/github-versions/`.
    /// Tests inject a tempdir here to avoid hitting the real GitHub API.
    pub cache_dir: Option<PathBuf>,
}

impl BinaryGitHubStatus {
    fn cache_path(&self) -> PathBuf {
        match &self.cache_dir {
            Some(dir) => dir.join(format!("{}-{}.json", self.owner, self.repo)),
            None => PathBuf::from(
                shellexpand::tilde(&format!(
                    "~/.cache/etch/github-versions/{}-{}.json",
                    self.owner, self.repo
                ))
                .into_owned(),
            ),
        }
    }

    fn fetch_latest_tag(&self) -> anyhow::Result<String> {
        let cache_path = self.cache_path();
        if let Some(cached) = read_cache(&cache_path) {
            return Ok(cached);
        }
        let async_runtime =
            Runtime::new().map_err(|e| anyhow::anyhow!("Failed to create async runtime: {e}"))?;
        let octocrab = async_runtime.block_on(async { octocrab::instance() });
        let repos = octocrab.repos(&self.owner, &self.repo);
        let releases = repos.releases();
        let release = async_runtime
            .block_on(releases.get_latest())
            .map_err(|e| anyhow::anyhow!("Failed to fetch latest release: {e}"))?;
        let tag = release.tag_name;
        let _ = write_cache(&cache_path, &tag);
        Ok(tag)
    }
}

impl crate::atoms::Atom for BinaryGitHubStatus {
    fn plan(&self) -> anyhow::Result<crate::atoms::Outcome> {
        Ok(crate::atoms::Outcome {
            side_effects: vec![],
            should_run: false,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn status(&self) -> anyhow::Result<crate::atoms::AtomStatus> {
        use crate::atoms::AtomStatus;
        let sidecar = PathBuf::from(format!("{}/.{}.version", self.directory, self.name));
        if !sidecar.exists() {
            return Ok(AtomStatus::Unchecked);
        }
        let installed = fs::read_to_string(&sidecar)?.trim().to_string();
        if normalize_version(&installed) != normalize_version(&self.version) {
            return Ok(AtomStatus::Drifted {
                expected: self.version.clone(),
                actual: installed,
            });
        }
        let latest = match self.fetch_latest_tag() {
            Ok(tag) => tag,
            Err(e) => {
                tracing::warn!("binary.github: failed to fetch latest tag: {e}");
                return Ok(AtomStatus::Unchecked);
            }
        };
        if normalize_version(&latest) != normalize_version(&self.version) {
            return Ok(AtomStatus::Drifted {
                expected: format!("{latest} (latest)"),
                actual: format!("{} (pinned)", self.version),
            });
        }
        Ok(AtomStatus::Ok)
    }
}

impl std::fmt::Display for BinaryGitHubStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "binary.github version check: {}/{}@{}",
            self.owner, self.repo, self.version
        )
    }
}

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryGitHub {
    pub name: String,
    pub directory: String,
    pub repository: String,
    pub version: Option<String>,
}

struct GitHubAsset {
    pub url: String,
    pub score: i32,
}

impl Action for BinaryGitHub {
    fn summarize(&self) -> String {
        format!(
            "Downloading binary from {} to {}",
            self.repository, self.directory
        )
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let binary_path = PathBuf::from(format!("{}/{}", self.directory, self.name));
        let binary_exists = binary_path.exists();
        let is_pinned = matches!(&self.version, Some(v) if v != "latest");

        // Early exit: no version pinned + binary exists = nothing to do
        if !is_pinned && binary_exists {
            return Ok(vec![]);
        }

        let (owner, repo) = self.repository.split_once('/').ok_or_else(|| {
            anyhow!(
                "Failed to parse repository name: {}",
                self.repository.as_str()
            )
        })?;

        let mut steps: Vec<Step> = Vec::new();

        // Version drift check atom — always present when version is pinned
        if is_pinned {
            steps.push(Step {
                atom: Box::new(BinaryGitHubStatus {
                    name: self.name.clone(),
                    directory: self.directory.clone(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    version: self.version.as_ref().unwrap().clone(),
                    cache_dir: None,
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        // Download steps — only when binary is absent
        if !binary_exists {
            let async_runtime = match Runtime::new() {
                Ok(runtime) => runtime,
                Err(e) => {
                    return Err(anyhow!("Failed to create async runtime: {e}"));
                }
            };

            let octocrab = async_runtime.block_on(async { octocrab::instance() });
            let repos = octocrab.repos(owner, repo);
            let releases = repos.releases();

            let result = match &self.version {
                Some(version) if version == "latest" => {
                    async_runtime.block_on(releases.get_latest())
                }
                Some(version) => async_runtime.block_on(releases.get_by_tag(version.as_str())),
                None => async_runtime.block_on(releases.get_latest()),
            };

            let release = match result {
                Ok(release) => release,
                Err(e) => {
                    return Err(anyhow!("Failed to find a release: {e}"));
                }
            };

            let asset: Option<GitHubAsset> = release.assets.into_iter().fold(None, |acc, asset| {
                let mut score = 0;

                let mut score_terms = vec![
                    std::env::consts::OS.to_lowercase(),
                    std::env::consts::ARCH.to_lowercase(),
                ];

                let os = os_info::get();
                if os.os_type() == os_info::Type::Macos {
                    score_terms.push(String::from("darwin"));
                    score_terms.push(String::from("apple"));
                } else {
                    score_terms.push(os.os_type().to_string());
                };

                if std::env::consts::ARCH == "aarch64" {
                    score_terms.push("arm".to_string());
                    score_terms.push("aarch".to_string());
                } else {
                    score_terms.push("unknown".to_string());
                };

                match os.bitness() {
                    os_info::Bitness::X32 => score_terms.push("32".to_string()),
                    os_info::Bitness::X64 => score_terms.push("64".to_string()),
                    _ => (),
                }

                score_terms.iter().for_each(|term| {
                    if asset.name.to_lowercase().contains(term.as_str()) {
                        score += 1;
                    }
                });

                match acc {
                    Some(ass) => {
                        if score > ass.score {
                            Some(GitHubAsset {
                                url: asset.browser_download_url.into(),
                                score,
                            })
                        } else {
                            Some(ass)
                        }
                    }
                    None => Some(GitHubAsset {
                        url: asset.browser_download_url.into(),
                        score,
                    }),
                }
            });

            let asset = match asset {
                Some(asset) => {
                    debug!("Downloading {:?}", asset.url);
                    asset
                }
                None => {
                    return Err(anyhow!("Failed to find a downloadable asset"));
                }
            };

            let to_path = PathBuf::from(format!("{}/{}", self.directory, self.name));

            steps.push(Step {
                atom: Box::new(Download {
                    url: asset.url,
                    to: to_path.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
            steps.push(Step {
                atom: Box::new(Chmod {
                    path: to_path,
                    mode: 0o755,
                }),
                initializers: vec![],
                finalizers: vec![],
            });

            // Write sidecar version file so future `etch status` can detect drift
            if is_pinned {
                let sidecar = PathBuf::from(format!("{}/.{}.version", self.directory, self.name));
                steps.push(Step {
                    atom: Box::new(SetContents {
                        path: sidecar,
                        contents: self.version.as_ref().unwrap().as_bytes().to_vec(),
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                });
            }
        }

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::atoms::Atom;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: binary.github
  name: gitleaks
  directory: /usr/local/bin
  repository: gitleaks/gitleaks
  version: latest
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BinaryGitHub(action)) => {
                assert_eq!("gitleaks", action.action.name);
                assert_eq!("/usr/local/bin", action.action.directory);
                assert_eq!("gitleaks/gitleaks", action.action.repository);
                assert_eq!(Some(String::from("latest")), action.action.version);
            }
            _ => panic!("BinaryGitHub didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_empty_when_binary_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("owner/repo"),
            version: None,
        };

        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_errors_on_invalid_repository_format() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("no-slash-here"),
            version: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn plan_with_pinned_version_and_binary_present_emits_one_step() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("owner/repo"),
            version: Some(String::from("v1.5.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len(), "expected 1 status-only step");
        assert!(
            steps[0].atom.to_string().contains("version check"),
            "expected status atom, got: {}",
            steps[0].atom
        );
    }

    #[test]
    fn plan_with_no_version_and_binary_present_emits_zero_steps() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("owner/repo"),
            version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_with_latest_version_and_binary_present_emits_zero_steps() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("owner/repo"),
            version: Some(String::from("latest")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_with_pinned_version_and_invalid_repo_errors() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("no-slash-here"),
            version: Some(String::from("v1.5.0")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn binary_github_status_unchecked_when_no_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: Some(cache_dir.path().to_path_buf()),
        };
        assert_eq!(atom.status().unwrap(), crate::atoms::AtomStatus::Unchecked);
    }

    #[test]
    fn binary_github_status_drifted_when_sidecar_mismatches_pinned() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".mytool.version"), "v1.4.0").unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: Some(cache_dir.path().to_path_buf()),
        };
        assert_eq!(
            atom.status().unwrap(),
            crate::atoms::AtomStatus::Drifted {
                expected: String::from("v1.5.0"),
                actual: String::from("v1.4.0"),
            }
        );
    }

    #[test]
    fn binary_github_status_ok_when_sidecar_matches_and_cache_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".mytool.version"), "v1.5.0").unwrap();
        write_cache(&cache_dir.path().join("owner-repo.json"), "v1.5.0").unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: Some(cache_dir.path().to_path_buf()),
        };
        assert_eq!(atom.status().unwrap(), crate::atoms::AtomStatus::Ok);
    }

    #[test]
    fn binary_github_status_drifted_when_pinned_behind_latest() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".mytool.version"), "v1.5.0").unwrap();
        write_cache(&cache_dir.path().join("owner-repo.json"), "v1.7.0").unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: Some(cache_dir.path().to_path_buf()),
        };
        assert_eq!(
            atom.status().unwrap(),
            crate::atoms::AtomStatus::Drifted {
                expected: String::from("v1.7.0 (latest)"),
                actual: String::from("v1.5.0 (pinned)"),
            }
        );
    }

    #[test]
    fn binary_github_status_ok_normalizes_v_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".mytool.version"), "1.5.0").unwrap();
        write_cache(&cache_dir.path().join("owner-repo.json"), "v1.5.0").unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: Some(cache_dir.path().to_path_buf()),
        };
        assert_eq!(atom.status().unwrap(), crate::atoms::AtomStatus::Ok);
    }

    #[test]
    fn binary_github_status_display() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = BinaryGitHubStatus {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            owner: String::from("owner"),
            repo: String::from("repo"),
            version: String::from("v1.5.0"),
            cache_dir: None,
        };
        assert_eq!(
            format!("{atom}"),
            "binary.github version check: owner/repo@v1.5.0"
        );
    }

    #[test]
    fn read_cache_returns_none_when_file_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        assert!(read_cache(&path).is_none());
    }

    #[test]
    fn read_cache_returns_none_for_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        std::fs::write(&path, b"not json").unwrap();
        assert!(read_cache(&path).is_none());
    }

    #[test]
    fn read_cache_returns_none_when_expired() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        std::fs::write(&path, r#"{"tag":"v1.5.0","fetched_at":0}"#).unwrap();
        assert!(read_cache(&path).is_none());
    }

    #[test]
    fn write_and_read_cache_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        write_cache(&path, "v1.7.0").unwrap();
        let tag = read_cache(&path).unwrap();
        assert_eq!(tag, "v1.7.0");
    }

    #[test]
    fn write_cache_creates_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a/b/c/cache.json");
        write_cache(&path, "v1.0.0").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn normalize_version_strips_leading_v() {
        assert_eq!(normalize_version("v1.5.0"), "1.5.0");
    }

    #[test]
    fn normalize_version_no_change_when_no_v() {
        assert_eq!(normalize_version("1.5.0"), "1.5.0");
    }

    #[test]
    fn normalize_version_only_strips_lowercase_v() {
        assert_eq!(normalize_version("V1.5.0"), "V1.5.0");
    }

    #[test]
    #[ignore]
    fn plan_downloads_real_github_release() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryGitHub {
            name: String::from("gitleaks"),
            directory: tmp.path().display().to_string(),
            repository: String::from("gitleaks/gitleaks"),
            version: Some(String::from("v8.30.1")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
    }
}
