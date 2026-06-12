use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewCleanup {
    pub prune: Option<u32>,
}

impl Action for BrewCleanup {
    fn summarize(&self) -> String {
        String::from("Cleaning up Homebrew cache")
    }

    fn state_key(&self) -> String {
        "brew.cleanup".to_string()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("cleanup")];
        if let Some(days) = self.prune {
            args.push(format!("--prune={days}"));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
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
- action: brew.cleanup
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewCleanup(action)) => {
                assert!(action.action.prune.is_none());
            }
            _ => panic!("BrewCleanup didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewCleanup;
        use crate::actions::Action;
        let action = BrewCleanup { prune: None };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("brew"), "expected 'brew' in: {display}");
        assert!(
            display.contains("cleanup"),
            "expected 'cleanup' in: {display}"
        );
    }

    #[test]
    fn plan_includes_prune_flag() {
        use super::BrewCleanup;
        use crate::actions::Action;
        let action = BrewCleanup { prune: Some(30) };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--prune=30"),
            "expected '--prune=30' in: {display}"
        );
    }
}
