use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZshOhMyZsh {
    /// Git URLs of oh-my-zsh community plugins to install.
    /// Each URL is cloned into ~/.oh-my-zsh/custom/plugins/<repo-name>.
    /// The repo name is the last path segment of the URL, with any trailing .git stripped.
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Override the oh-my-zsh install directory. Defaults to ~/.oh-my-zsh.
    /// Used in tests to inject a temp directory; not normally set in manifests.
    #[serde(default = "default_omz_dir", skip_serializing)]
    #[schemars(skip)]
    pub omz_dir: String,
}

fn default_omz_dir() -> String {
    shellexpand::tilde("~/.oh-my-zsh").into_owned()
}

impl Default for ZshOhMyZsh {
    fn default() -> Self {
        Self {
            plugins: vec![],
            omz_dir: default_omz_dir(),
        }
    }
}

impl Action for ZshOhMyZsh {
    fn summarize(&self) -> String {
        if self.plugins.is_empty() {
            String::from("Installing oh-my-zsh")
        } else {
            format!("Installing oh-my-zsh with {} plugin(s)", self.plugins.len())
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::git::Clone;
        use std::path::PathBuf;

        let omz_path = PathBuf::from(&self.omz_dir);
        let plugins_base = omz_path.join("custom/plugins");

        let mut steps: Vec<Step> = vec![];

        if !omz_path.exists() {
            let url = gix::url::parse("https://github.com/ohmyzsh/ohmyzsh".into())?;
            steps.push(Step {
                atom: Box::new(Clone {
                    repository: url,
                    directory: omz_path.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        for plugin_url in &self.plugins {
            let name = plugin_name_from_url(plugin_url).ok_or_else(|| {
                anyhow::anyhow!("cannot extract plugin name from URL: {plugin_url}")
            })?;
            let plugin_dir = plugins_base.join(&name);
            if !plugin_dir.exists() {
                let url = gix::url::parse(plugin_url.as_str().into())?;
                steps.push(Step {
                    atom: Box::new(Clone {
                        repository: url,
                        directory: plugin_dir,
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                });
            }
        }

        Ok(steps)
    }
}

pub(crate) fn plugin_name_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let segments: Vec<&str> = without_scheme
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let name = segments.last()?.trim_end_matches(".git");
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use tempfile::TempDir;

    // ── plugin_name_from_url ──────────────────────────────────────────────

    #[test]
    fn plugin_name_from_https_url() {
        assert_eq!(
            Some(String::from("zsh-autosuggestions")),
            plugin_name_from_url("https://github.com/zsh-users/zsh-autosuggestions")
        );
    }

    #[test]
    fn plugin_name_strips_git_suffix() {
        assert_eq!(
            Some(String::from("bar")),
            plugin_name_from_url("https://github.com/foo/bar.git")
        );
    }

    #[test]
    fn plugin_name_strips_trailing_slash() {
        assert_eq!(
            Some(String::from("bar")),
            plugin_name_from_url("https://github.com/foo/bar/")
        );
    }

    #[test]
    fn plugin_name_returns_none_for_no_path() {
        assert_eq!(None, plugin_name_from_url("https://example.com"));
    }

    // ── plan() ───────────────────────────────────────────────────────────

    #[test]
    fn plan_no_plugins_omz_absent_emits_one_step() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let action = ZshOhMyZsh {
            plugins: vec![],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_no_plugins_omz_present_emits_zero_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        std::fs::create_dir_all(&omz_dir).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_two_plugins_nothing_installed_emits_three_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len());
    }

    #[test]
    fn plan_two_plugins_omz_exists_plugins_absent_emits_two_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        std::fs::create_dir_all(&omz_dir).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
    }

    #[test]
    fn plan_everything_installed_emits_zero_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let plugins_dir = omz_dir.join("custom/plugins");
        std::fs::create_dir_all(plugins_dir.join("zsh-autosuggestions")).unwrap();
        std::fs::create_dir_all(plugins_dir.join("zsh-syntax-highlighting")).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_malformed_url_returns_err() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let action = ZshOhMyZsh {
            plugins: vec![String::from("https://example.com")],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    // ── deserialization ───────────────────────────────────────────────────

    #[test]
    fn it_can_be_deserialized_without_plugins() {
        let yaml = r#"
actions:
  - action: zsh.oh-my-zsh
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::ZshOhMyZsh(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert!(action.plugins.is_empty());
    }

    #[test]
    fn it_can_be_deserialized_with_plugins() {
        let yaml = r#"
actions:
  - action: zsh.oh-my-zsh
    plugins:
      - "https://github.com/zsh-users/zsh-autosuggestions"
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::ZshOhMyZsh(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(1, action.plugins.len());
        assert_eq!(
            "https://github.com/zsh-users/zsh-autosuggestions",
            action.plugins[0]
        );
    }
}
