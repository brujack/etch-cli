use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewUpgrade {
    #[serde(default = "get_false")]
    pub greedy: bool,
}

fn get_false() -> bool {
    false
}

impl Action for BrewUpgrade {
    fn summarize(&self) -> String {
        if self.greedy {
            String::from("Upgrading Homebrew packages (greedy)")
        } else {
            String::from("Upgrading Homebrew packages")
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("upgrade")];
        if self.greedy {
            args.push(String::from("--greedy"));
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
- action: brew.upgrade
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewUpgrade(action)) => {
                assert!(!action.action.greedy);
            }
            _ => panic!("BrewUpgrade didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewUpgrade;
        use crate::actions::Action;
        let action = BrewUpgrade { greedy: false };
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
            display.contains("upgrade"),
            "expected 'upgrade' in: {display}"
        );
    }

    #[test]
    fn plan_includes_greedy_flag() {
        use super::BrewUpgrade;
        use crate::actions::Action;
        let action = BrewUpgrade { greedy: true };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--greedy"),
            "expected '--greedy' in: {display}"
        );
    }
}
