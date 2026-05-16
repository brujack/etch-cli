use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// BrewBundle is registered in the Actions enum in actions/mod.rs (Task 2).
// The allow attribute is removed once the enum variant is added.
#[allow(dead_code)]
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewBundle {
    pub file: String,

    #[serde(default = "get_false")]
    pub no_upgrade: bool,

    #[serde(default = "get_false")]
    pub cleanup: bool,
}

#[allow(dead_code)]
fn get_false() -> bool {
    false
}

impl Action for BrewBundle {
    fn summarize(&self) -> String {
        format!("Installing Homebrew bundle from {}", self.file)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![
            String::from("bundle"),
            String::from("install"),
            format!("--file={}", self.file),
        ];
        if self.no_upgrade {
            args.push(String::from("--no-upgrade"));
        }
        if self.cleanup {
            args.push(String::from("--cleanup"));
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
    // NOTE: it_can_be_deserialized requires Actions::BrewBundle from the enum.
    // Added in Task 2 after the enum variant is registered.

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: false,
            cleanup: false,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("brew"), "expected 'brew' in: {display}");
    }

    #[test]
    fn plan_includes_no_upgrade_flag() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: true,
            cleanup: false,
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
            display.contains("--no-upgrade"),
            "expected '--no-upgrade' in: {display}"
        );
    }

    #[test]
    fn plan_includes_cleanup_flag() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: false,
            cleanup: true,
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
            display.contains("--cleanup"),
            "expected '--cleanup' in: {display}"
        );
    }
}
