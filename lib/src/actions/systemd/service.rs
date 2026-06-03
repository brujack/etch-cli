use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemdService {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    #[serde(default)]
    pub privileged: bool,
}

impl Action for SystemdService {
    fn summarize(&self) -> String {
        let mut parts: Vec<&str> = vec![];
        if let Some(e) = self.enabled {
            parts.push(if e { "enable" } else { "disable" });
        }
        if let Some(s) = self.started {
            parts.push(if s { "start" } else { "stop" });
        }
        if parts.is_empty() {
            format!("systemd service {}", self.unit)
        } else {
            format!("{} service {}", parts.join("+"), self.unit)
        }
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.enabled.is_none() && self.started.is_none() {
            anyhow::bail!(
                "systemd.service: at least one of 'enabled' or 'started' must be set for unit {}",
                self.unit
            );
        }
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        Ok(vec![Step {
            atom: Box::new(crate::atoms::systemd::Service {
                unit: self.unit.clone(),
                enabled: self.enabled,
                started: self.started,
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

    #[test]
    fn deserialize_enabled_and_started() {
        let yaml = r#"
unit: sshd.service
enabled: true
started: true
privileged: true
"#;
        let action: SystemdService = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.unit, "sshd.service");
        assert_eq!(action.enabled, Some(true));
        assert_eq!(action.started, Some(true));
        assert!(action.privileged);
    }

    #[test]
    fn deserialize_enabled_only() {
        let yaml = r#"
unit: bluetooth.service
enabled: false
"#;
        let action: SystemdService = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.unit, "bluetooth.service");
        assert_eq!(action.enabled, Some(false));
        assert_eq!(action.started, None);
        assert!(!action.privileged);
    }

    #[test]
    fn deserialize_started_only() {
        let yaml = r#"
unit: cups.service
started: false
"#;
        let action: SystemdService = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.unit, "cups.service");
        assert_eq!(action.enabled, None);
        assert_eq!(action.started, Some(false));
    }

    #[test]
    fn plan_returns_one_step_with_enabled_only() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: Some(true),
            started: None,
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_returns_one_step_with_started_only() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "cups.service".to_string(),
            enabled: None,
            started: Some(false),
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_errors_when_both_none() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: None,
            started: None,
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let result = action.plan(&manifest, &contexts);
        assert!(result.is_err(), "expected Err when both fields are None");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("at least one"),
            "expected helpful error message, got: {msg}"
        );
    }

    #[test]
    fn summarize_enabled_true() {
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: Some(true),
            started: None,
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("enable"), "got: {s}");
        assert!(s.contains("sshd.service"), "got: {s}");
    }

    #[test]
    fn summarize_enabled_false() {
        let action = SystemdService {
            unit: "bluetooth.service".to_string(),
            enabled: Some(false),
            started: None,
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("disable"), "got: {s}");
    }

    #[test]
    fn summarize_started_true() {
        let action = SystemdService {
            unit: "cups.service".to_string(),
            enabled: None,
            started: Some(true),
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("start"), "got: {s}");
        assert!(s.contains("cups.service"), "got: {s}");
    }

    #[test]
    fn summarize_started_false() {
        let action = SystemdService {
            unit: "cups.service".to_string(),
            enabled: None,
            started: Some(false),
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("stop"), "got: {s}");
    }

    #[test]
    fn summarize_both_enabled_and_started() {
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: Some(true),
            started: Some(true),
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("enable"), "got: {s}");
        assert!(s.contains("start"), "got: {s}");
        assert!(s.contains('+'), "expected '+' separator, got: {s}");
    }

    #[test]
    fn summarize_neither_enabled_nor_started() {
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: None,
            started: None,
            privileged: false,
        };
        let s = action.summarize();
        assert!(s.contains("systemd service"), "got: {s}");
        assert!(s.contains("sshd.service"), "got: {s}");
    }
}
