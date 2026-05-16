use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
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
    // NOTE: it_can_be_deserialized requires Actions::MasInstall from the enum.
    // Added in Task 2 after the enum variant is registered.

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
