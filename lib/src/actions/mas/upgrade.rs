use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasUpgrade {
    pub id: Option<u64>,
}

impl Action for MasUpgrade {
    fn state_key(&self) -> String {
        self.id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "all".to_string())
    }

    fn summarize(&self) -> String {
        match self.id {
            Some(id) => format!("Upgrading App Store app {id}"),
            None => String::from("Upgrading all App Store apps"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("upgrade")];
        if let Some(id) = self.id {
            args.push(id.to_string());
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("mas"),
                arguments: args,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: mas.upgrade
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasUpgrade(action)) => {
                assert!(action.action.id.is_none());
            }
            _ => panic!("MasUpgrade didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_exec_step() {
        use super::MasUpgrade;
        use crate::actions::Action;
        let action = MasUpgrade { id: None };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("mas"), "expected 'mas' in: {display}");
        assert!(
            display.contains("upgrade"),
            "expected 'upgrade' in: {display}"
        );
    }

    #[test]
    fn plan_includes_id_when_set() {
        use super::MasUpgrade;
        use crate::actions::Action;
        let action = MasUpgrade {
            id: Some(414209656),
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("414209656"),
            "expected app ID in: {display}"
        );
    }
}
