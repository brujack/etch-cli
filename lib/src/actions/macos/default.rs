use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// I went through all the examples here: https://macos-defaults.com/
// and while arrays and dictionaries are valid values, I couldn't
// find any usable examples. So omitting for now
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    pub domain: String,
    pub key: String,
    pub kind: String,
    pub value: String,
}

impl Action for MacOSDefault {
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("defaults"),
                arguments: vec![
                    String::from("write"),
                    self.domain.clone(),
                    self.key.clone(),
                    format!("-{}", self.kind),
                    self.value.clone(),
                ],
                ..Default::default()
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
                assert_eq!("bool", action.action.kind);
                assert_eq!("true", action.action.value);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            kind: String::from("bool"),
            value: String::from("true"),
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
            kind: String::from("integer"),
            value: String::from("48"),
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
            kind: String::from("string"),
            value: String::from("myvalue"),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }
}
