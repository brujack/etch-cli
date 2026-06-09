use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "macos.softwareupdate")]
pub struct MacOSSoftwareUpdate {}

impl Action for MacOSSoftwareUpdate {
    fn summarize(&self) -> String {
        String::from("Installing macOS software updates")
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(context).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("softwareupdate"),
                arguments: vec![String::from("--install"), String::from("--all")],
                privileged: true,
                privilege_provider,
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
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.softwareupdate
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSSoftwareUpdate(_)) => {}
            _ => panic!("MacOSSoftwareUpdate didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_returns_non_empty_string() {
        let action = MacOSSoftwareUpdate {};
        assert!(!action.summarize().is_empty());
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_step_runs_softwareupdate_install_all() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("softwareupdate"),
            "expected 'softwareupdate' in: {display}"
        );
        assert!(
            display.contains("--install"),
            "expected '--install' in: {display}"
        );
        assert!(display.contains("--all"), "expected '--all' in: {display}");
    }

    #[test]
    fn plan_step_is_privileged() {
        let action = MacOSSoftwareUpdate {};
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
    fn plan_step_has_no_initializers() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            steps[0].initializers.len(),
            0,
            "expected no initializers — softwareupdate is self-idempotent"
        );
    }
}
