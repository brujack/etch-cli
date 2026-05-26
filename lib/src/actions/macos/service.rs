use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacOSServiceState {
    #[default]
    Loaded,
    Unloaded,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSService {
    pub plist: String,
    pub label: Option<String>,
    pub state: MacOSServiceState,
    #[serde(default)]
    pub privileged: bool,
}

impl Action for MacOSService {
    fn summarize(&self) -> String {
        let action = match self.state {
            MacOSServiceState::Loaded => "load",
            MacOSServiceState::Unloaded => "unload",
        };
        format!("{} service {}", action, self.plist)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let expanded = shellexpand::tilde(&self.plist).to_string();
        let path = PathBuf::from(&expanded);
        if !path.exists() {
            anyhow::bail!("plist file does not exist: {}", expanded);
        }
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        let load = matches!(self.state, MacOSServiceState::Loaded);
        Ok(vec![Step {
            atom: Box::new(crate::atoms::macos::Service {
                plist: path,
                label: self.label.clone(),
                load,
                privileged: self.privileged,
                privilege_provider,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized_loaded() {
        let yaml = r#"
- action: macos.service
  plist: /System/Library/LaunchDaemons/com.apple.ssh.plist
  state: loaded
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(
                    "/System/Library/LaunchDaemons/com.apple.ssh.plist",
                    action.action.plist
                );
                assert_eq!(MacOSServiceState::Loaded, action.action.state);
                assert!(action.action.privileged);
                assert!(action.action.label.is_none());
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn it_can_be_deserialized_unloaded() {
        let yaml = r#"
- action: macos.service
  plist: ~/Library/LaunchAgents/com.myapp.agent.plist
  state: unloaded
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(
                    "~/Library/LaunchAgents/com.myapp.agent.plist",
                    action.action.plist
                );
                assert_eq!(MacOSServiceState::Unloaded, action.action.state);
                assert!(!action.action.privileged);
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_explicit_label() {
        let yaml = r#"
- action: macos.service
  plist: /Library/LaunchDaemons/com.myapp.daemon.plist
  label: com.myapp.daemon
  state: loaded
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(Some("com.myapp.daemon".to_string()), action.action.label);
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn privileged_defaults_to_false() {
        let yaml = r#"
- action: macos.service
  plist: /tmp/test.plist
  state: loaded
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert!(!action.action.privileged);
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn plan_errors_if_plist_missing() {
        let action = MacOSService {
            plist: "/nonexistent/path/test.plist".to_string(),
            label: None,
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn plan_returns_one_step_when_plist_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(&plist, "").unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_step_display_contains_load() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(&plist, "").unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("load"),
            "expected 'load' in atom display: {display}"
        );
    }

    #[test]
    fn plan_step_display_contains_unload() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(&plist, "").unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Unloaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("unload"),
            "expected 'unload' in atom display: {display}"
        );
    }
}
