use crate::contexts::privilege::Privilege;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub manifest_paths: Vec<String>,

    #[serde(default)]
    pub variables: BTreeMap<String, String>,

    #[serde(default)]
    pub include_variables: Option<Vec<String>>,

    #[serde(default)]
    pub disable_update_check: bool,

    #[serde(default)]
    pub privilege: Privilege,

    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UpdateConfig {
    pub git_tools: Option<GitToolsConfig>,
    pub claude: Option<ClaudeUpdateConfig>,
    pub log_path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GitToolsConfig {
    pub ai_config: Option<String>,
    pub dotfiles: Option<String>,
    pub oh_my_zsh: Option<bool>,
    pub tpm: Option<bool>,
    pub tfenv: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUpdateConfig {
    #[serde(default)]
    pub npm_globals: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_config_default_has_none_fields() {
        let c = UpdateConfig::default();
        assert!(c.git_tools.is_none());
        assert!(c.claude.is_none());
        assert!(c.log_path.is_none());
    }

    #[test]
    fn claude_update_config_default_has_empty_vecs() {
        let c = ClaudeUpdateConfig::default();
        assert!(c.npm_globals.is_empty());
    }

    #[test]
    fn update_config_deserialize_full_yaml() {
        let yaml = r#"
update:
    git_tools:
        ai_config: "git@github.com:user/ai-config.git"
        oh_my_zsh: true
        tpm: true
        tfenv: false
    claude:
        npm_globals:
            - firecrawl-cli
    log_path: "/tmp/etch-update.log"
"#;
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        let update = &config.update;
        let git = update.git_tools.as_ref().unwrap();
        assert_eq!(
            git.ai_config.as_deref(),
            Some("git@github.com:user/ai-config.git")
        );
        assert_eq!(git.oh_my_zsh, Some(true));
        assert_eq!(git.tpm, Some(true));
        assert_eq!(git.tfenv, Some(false));
        let claude = update.claude.as_ref().unwrap();
        assert_eq!(claude.npm_globals, vec!["firecrawl-cli"]);
        assert_eq!(update.log_path.as_deref(), Some("/tmp/etch-update.log"));
    }

    #[test]
    fn claude_update_config_accepts_unknown_plugins_field() {
        // Existing etch.yaml files may have plugins: — should not fail deserialization
        // since ClaudeUpdateConfig does not use deny_unknown_fields.
        let yaml = r#"
update:
    claude:
        plugins:
            - superpowers@claude-plugins-official
        npm_globals:
            - firecrawl-cli
"#;
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        let claude = config.update.claude.as_ref().unwrap();
        assert_eq!(claude.npm_globals, vec!["firecrawl-cli"]);
    }

    #[test]
    fn update_config_absent_defaults_to_empty() {
        let yaml = "manifest_paths: []\n";
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.update.git_tools.is_none());
        assert!(config.update.claude.is_none());
    }

    #[test]
    fn git_tools_config_all_fields_optional() {
        let yaml = r#"
update:
    git_tools: {}
"#;
        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        let git = config.update.git_tools.as_ref().unwrap();
        assert!(git.ai_config.is_none());
        assert!(git.dotfiles.is_none());
        assert!(git.oh_my_zsh.is_none());
    }
}
