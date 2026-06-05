use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "package.autoremove")]
pub struct PackageAutoremove {}

impl Action for PackageAutoremove {
    fn summarize(&self) -> String {
        String::from("Removing orphaned packages")
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(context).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("apt"),
                arguments: vec![String::from("autoremove"), String::from("--yes")],
                environment: vec![(
                    String::from("DEBIAN_FRONTEND"),
                    String::from("noninteractive"),
                )],
                privileged: true,
                privilege_provider,
                streaming: true,
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

    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: package.autoremove
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageAutoremove(_)) => {}
            _ => panic!("PackageAutoremove didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_returns_non_empty_string() {
        let action = PackageAutoremove {};
        assert!(!action.summarize().is_empty());
    }

    #[test]
    fn plan_returns_one_step() {
        let action = PackageAutoremove {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_step_runs_apt_autoremove() {
        let action = PackageAutoremove {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(display.contains("apt"), "expected 'apt' in: {display}");
        assert!(
            display.contains("autoremove"),
            "expected 'autoremove' in: {display}"
        );
    }

    #[test]
    fn plan_step_is_privileged() {
        let action = PackageAutoremove {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("privileged=true"),
            "expected privileged=true in: {display}"
        );
    }
}
