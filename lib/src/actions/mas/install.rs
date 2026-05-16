use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasInstall {
    pub name: String,
    pub id: u64,
}

impl Action for MasInstall {
    fn summarize(&self) -> String {
        format!("Installing {} from the Mac App Store", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("mas"),
                arguments: vec![String::from("install"), self.id.to_string()],
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
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasInstall(action)) => {
                assert_eq!("Better Rename 9", action.action.name);
                assert_eq!(414209656u64, action.action.id);
            }
            _ => panic!("MasInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_exec_step() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Better Rename 9"),
            id: 414209656,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Exec Display: "CommandExec with: privileged=false: mas install 414209656"
        let display = steps[0].atom.to_string();
        assert!(display.contains("mas"), "expected 'mas' in: {display}");
        assert!(
            display.contains("414209656"),
            "expected app ID in: {display}"
        );
    }

    #[test]
    fn plan_includes_correct_id() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Flycut"),
            id: 442160987,
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
            display.contains("442160987"),
            "expected correct ID in: {display}"
        );
    }

    #[test]
    fn summarize_includes_name() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Better Rename 9"),
            id: 414209656,
        };
        let summary = action.summarize();
        assert!(
            summary.contains("Better Rename 9"),
            "expected app name in: {summary}"
        );
    }
}
