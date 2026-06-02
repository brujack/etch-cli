use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::initializers::{FileExists, FlowControl};
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "macos.rosetta")]
pub struct MacOSRosetta {}

impl Action for MacOSRosetta {
    fn summarize(&self) -> String {
        String::from("Ensuring Rosetta 2 is installed")
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(context).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("softwareupdate"),
                arguments: vec![
                    String::from("--install-rosetta"),
                    String::from("--agree-to-license"),
                ],
                privileged: true,
                privilege_provider,
                ..Default::default()
            }),
            initializers: vec![FlowControl::SkipIf(Box::new(FileExists(PathBuf::from(
                "/Library/Apple/usr/share/rosetta/rosetta",
            ))))],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.rosetta
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSRosetta(_)) => {}
            _ => panic!("MacOSRosetta didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_returns_non_empty_string() {
        let action = MacOSRosetta {};
        assert!(!action.summarize().is_empty());
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSRosetta {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_step_runs_softwareupdate_install_rosetta() {
        let action = MacOSRosetta {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("softwareupdate"),
            "expected 'softwareupdate' in: {display}"
        );
        assert!(
            display.contains("install-rosetta"),
            "expected 'install-rosetta' in: {display}"
        );
    }

    #[test]
    fn plan_step_is_privileged() {
        let action = MacOSRosetta {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("privileged=true"),
            "expected privileged=true in: {display}"
        );
    }

    #[test]
    fn plan_step_has_one_initializer() {
        let action = MacOSRosetta {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            steps[0].initializers.len(),
            1,
            "expected 1 SkipIf initializer for idempotency"
        );
    }
}
