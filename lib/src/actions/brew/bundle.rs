use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewBundle {
    pub file: String,

    #[serde(default = "get_false")]
    pub no_upgrade: bool,

    #[serde(default = "get_false")]
    pub cleanup: bool,
}

fn get_false() -> bool {
    false
}

impl Action for BrewBundle {
    fn summarize(&self) -> String {
        format!("Installing Homebrew bundle from {}", self.file)
    }

    fn state_key(&self) -> String {
        self.file.clone()
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
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: brew.bundle
  file: /tmp/Brewfile
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewBundle(action)) => {
                assert_eq!("/tmp/Brewfile", action.action.file);
                assert!(!action.action.no_upgrade);
                assert!(!action.action.cleanup);
            }
            _ => panic!("BrewBundle didn't deserialize to the correct type"),
        }
    }

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
