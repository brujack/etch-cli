use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacOSDefaultOperation {
    #[default]
    Write,
    ArrayAdd,
    Delete,
}

// I went through all the examples here: https://macos-defaults.com/
// and while arrays and dictionaries are valid values, I couldn't
// find any usable examples. So omitting for now
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    pub domain: String,
    pub key: String,
    #[serde(default)]
    pub operation: MacOSDefaultOperation,
    pub kind: Option<String>,
    pub value: Option<String>,
}

const VALID_KINDS: &[&str] = &[
    "string", "integer", "int", "float", "bool", "boolean", "date", "data", "array", "dict",
];

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn delete_shell_cmd(domain: &str, key: &str) -> String {
    format!(
        "defaults delete {} {} 2>/dev/null || true",
        sh_quote(domain),
        sh_quote(key),
    )
}

// NOTE: grep -qF matches by substring. If the array already contains an element
// that is a prefix of `value` (e.g. "/bin" present when adding "/bin/sh"), the
// check produces a false positive and skips the add. This is an acceptable
// trade-off for the primary use case (macOS plist paths, which are typically
// unique and not prefixes of each other).
fn array_add_shell_cmd(domain: &str, key: &str, kind: &str, value: &str) -> String {
    let domain_q = sh_quote(domain);
    let key_q = sh_quote(key);
    let value_q = sh_quote(value);
    format!(
        "defaults read {domain} {key} 2>/dev/null | grep -qF {value} || defaults write {domain} {key} -array-add -{kind} {value}",
        domain = domain_q,
        key = key_q,
        kind = kind,
        value = value_q,
    )
}

impl Action for MacOSDefault {
    fn summarize(&self) -> String {
        format!(
            "macos.default {} {} = {}",
            self.domain,
            self.key,
            self.value.as_deref().unwrap_or("<delete>")
        )
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        match self.operation {
            MacOSDefaultOperation::Write => {
                let kind = self
                    .kind
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`kind` is required for operation `write`"))?;
                if !VALID_KINDS.contains(&kind) {
                    anyhow::bail!("`kind` must be one of {:?}, got {:?}", VALID_KINDS, kind);
                }
                let value = self
                    .value
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`value` is required for operation `write`"))?;
                Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("defaults"),
                        arguments: vec![
                            String::from("write"),
                            self.domain.clone(),
                            self.key.clone(),
                            format!("-{}", kind),
                            value.to_string(),
                        ],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }])
            }
            MacOSDefaultOperation::Delete => Ok(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("sh"),
                    arguments: vec![
                        String::from("-c"),
                        delete_shell_cmd(&self.domain, &self.key),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]),
            MacOSDefaultOperation::ArrayAdd => {
                let kind = self.kind.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("`kind` is required for operation `array-add`")
                })?;
                if !VALID_KINDS.contains(&kind) {
                    anyhow::bail!("`kind` must be one of {:?}, got {:?}", VALID_KINDS, kind);
                }
                let value = self.value.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("`value` is required for operation `array-add`")
                })?;
                Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("sh"),
                        arguments: vec![
                            String::from("-c"),
                            array_add_shell_cmd(&self.domain, &self.key, kind, value),
                        ],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!("com.apple.dock", action.action.domain);
                assert_eq!("autohide", action.action.key);
                assert_eq!(Some(String::from("bool")), action.action.kind);
                assert_eq!(Some(String::from("true")), action.action.value);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("bool")),
            value: Some(String::from("true")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_integer_kind() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("tilesize"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("integer")),
            value: Some(String::from("48")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_string_kind() {
        let action = MacOSDefault {
            domain: String::from("com.example.app"),
            key: String::from("mykey"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("string")),
            value: Some(String::from("myvalue")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn operation_defaults_to_write() {
        let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::Write, action.action.operation);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn write_missing_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: None,
            value: Some(String::from("true")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn write_missing_value_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("bool")),
            value: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn it_can_deserialize_delete() {
        let yaml = r#"
- action: macos.default
  operation: delete
  domain: com.apple.dock
  key: stale-key
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::Delete, action.action.operation);
                assert_eq!("com.apple.dock", action.action.domain);
                assert_eq!("stale-key", action.action.key);
            }
            _ => panic!("MacOSDefault delete didn't deserialize"),
        }
    }

    #[test]
    fn delete_emits_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("stale-key"),
            operation: MacOSDefaultOperation::Delete,
            kind: None,
            value: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn delete_ignores_kind_and_value() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("stale-key"),
            operation: MacOSDefaultOperation::Delete,
            kind: Some(String::from("bool")),
            value: Some(String::from("true")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        // Verify the shell command template doesn't include kind or value.
        // Use sh_quote form to mirror how arguments would appear if leaked.
        let cmd = delete_shell_cmd(&action.domain, &action.key);
        assert!(
            !cmd.contains("'bool'"),
            "kind leaked into delete command: {cmd}"
        );
        assert!(
            !cmd.contains("'true'"),
            "value leaked into delete command: {cmd}"
        );
    }

    #[test]
    fn delete_shell_cmd_produces_correct_command() {
        let cmd = delete_shell_cmd("com.apple.dock", "tilesize");
        assert_eq!(
            "defaults delete 'com.apple.dock' 'tilesize' 2>/dev/null || true",
            cmd
        );
    }

    #[test]
    fn delete_shell_cmd_escapes_single_quotes() {
        let cmd = delete_shell_cmd("com.apple.it's", "key");
        assert_eq!(
            "defaults delete 'com.apple.it'\\''s' 'key' 2>/dev/null || true",
            cmd
        );
    }

    #[test]
    fn it_can_deserialize_array_add() {
        let yaml = r#"
- action: macos.default
  operation: array-add
  domain: com.apple.systemuiserver
  key: menuExtras
  kind: string
  value: "/System/Library/CoreServices/Menu Extras/Volume.menu"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::ArrayAdd, action.action.operation);
                assert_eq!("com.apple.systemuiserver", action.action.domain);
                assert_eq!("menuExtras", action.action.key);
                assert_eq!(Some(String::from("string")), action.action.kind);
                assert_eq!(
                    Some(String::from(
                        "/System/Library/CoreServices/Menu Extras/Volume.menu"
                    )),
                    action.action.value
                );
            }
            _ => panic!("MacOSDefault array-add didn't deserialize"),
        }
    }

    #[test]
    fn array_add_emits_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: Some(String::from("string")),
            value: Some(String::from(
                "/System/Library/CoreServices/Menu Extras/Volume.menu",
            )),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn array_add_missing_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: None,
            value: Some(String::from(
                "/System/Library/CoreServices/Menu Extras/Volume.menu",
            )),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_missing_value_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: Some(String::from("string")),
            value: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_shell_cmd_produces_correct_command() {
        let cmd = array_add_shell_cmd(
            "com.apple.systemuiserver",
            "menuExtras",
            "string",
            "/System/Library/CoreServices/Menu Extras/Volume.menu",
        );
        assert_eq!(
            "defaults read 'com.apple.systemuiserver' 'menuExtras' 2>/dev/null \
             | grep -qF '/System/Library/CoreServices/Menu Extras/Volume.menu' \
             || defaults write 'com.apple.systemuiserver' 'menuExtras' -array-add \
             -string '/System/Library/CoreServices/Menu Extras/Volume.menu'",
            cmd
        );
    }

    #[test]
    fn write_invalid_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("notakind")),
            value: Some(String::from("true")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_invalid_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: Some(String::from("notakind")),
            value: Some(String::from("/some/path")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_shell_cmd_escapes_single_quotes_in_value() {
        let cmd = array_add_shell_cmd("com.example.app", "key", "string", "it's a value");
        assert_eq!(
            "defaults read 'com.example.app' 'key' 2>/dev/null | grep -qF 'it'\\''s a value' || defaults write 'com.example.app' 'key' -array-add -string 'it'\\''s a value'",
            cmd
        );
    }

    #[test]
    fn summarize_includes_domain_key_and_value() {
        let action = MacOSDefault {
            domain: "com.apple.dock".into(),
            key: "autohide".into(),
            value: Some("true".into()),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("com.apple.dock"), "missing domain: {s}");
        assert!(s.contains("autohide"), "missing key: {s}");
        assert!(s.contains("true"), "missing value: {s}");
    }

    #[test]
    fn summarize_delete_operation_shows_placeholder() {
        let action = MacOSDefault {
            domain: "com.apple.dock".into(),
            key: "autohide".into(),
            value: None,
            operation: MacOSDefaultOperation::Delete,
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("<delete>"), "missing delete placeholder: {s}");
    }
}
