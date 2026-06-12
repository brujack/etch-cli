use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasApp {
    pub name: String,
    pub id: u64,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasInstall {
    /// Single app name. Mutually exclusive with `list`.
    pub name: Option<String>,
    /// Single app ID. Mutually exclusive with `list`.
    pub id: Option<u64>,
    /// List of apps to install. Mutually exclusive with `name` + `id`.
    #[serde(default)]
    pub list: Vec<MasApp>,
}

impl MasInstall {
    fn app_list(&self) -> Vec<MasApp> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let (Some(name), Some(id)) = (&self.name, self.id) {
            vec![MasApp {
                name: name.clone(),
                id,
            }]
        } else {
            vec![]
        }
    }
}

impl Action for MasInstall {
    fn state_key(&self) -> String {
        self.id
            .map(|id| id.to_string())
            .or_else(|| self.list.first().map(|a| a.id.to_string()))
            .unwrap_or_default()
    }

    fn summarize(&self) -> String {
        let apps = self.app_list();
        match apps.len() {
            0 => String::from("Installing Mac App Store apps"),
            1 => format!("Installing {} from the Mac App Store", apps[0].name),
            _ => {
                let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
                format!(
                    "Installing {} apps from the Mac App Store: {}",
                    apps.len(),
                    names.join(", ")
                )
            }
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let apps = self.app_list();
        if apps.is_empty() {
            bail!("mas.install requires either 'name' + 'id' or 'list' to be specified");
        }

        Ok(apps
            .into_iter()
            .map(|app| Step {
                atom: Box::new(Exec {
                    command: String::from("mas"),
                    arguments: vec![String::from("install"), app.id.to_string()],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect())
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
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasInstall(action)) => {
                assert_eq!(Some("Better Rename 9".to_string()), action.action.name);
                assert_eq!(Some(414209656u64), action.action.id);
                assert!(action.action.list.is_empty());
            }
            _ => panic!("MasInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = r#"
- action: mas.install
  list:
    - name: "Better Rename 9"
      id: 414209656
    - name: Flycut
      id: 442160987
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasInstall(action)) => {
                assert_eq!(2, action.action.list.len());
                assert_eq!("Better Rename 9", action.action.list[0].name);
                assert_eq!(414209656u64, action.action.list[0].id);
                assert_eq!("Flycut", action.action.list[1].name);
                assert_eq!(442160987u64, action.action.list[1].id);
                assert!(action.action.name.is_none());
                assert!(action.action.id.is_none());
            }
            _ => panic!("MasInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_returns_one_step_for_single_app() {
        let action = MasInstall {
            name: Some(String::from("Better Rename 9")),
            id: Some(414209656),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("mas"), "expected 'mas' in: {display}");
        assert!(
            display.contains("414209656"),
            "expected app ID in: {display}"
        );
    }

    #[test]
    fn plan_returns_one_step_per_app_in_list() {
        let action = MasInstall {
            name: None,
            id: None,
            list: vec![
                MasApp {
                    name: String::from("Better Rename 9"),
                    id: 414209656,
                },
                MasApp {
                    name: String::from("Flycut"),
                    id: 442160987,
                },
            ],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
        assert!(steps[0].atom.to_string().contains("414209656"));
        assert!(steps[1].atom.to_string().contains("442160987"));
    }

    #[test]
    fn plan_errors_without_name_id_or_list() {
        let action = MasInstall::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when no name/id or list");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("name") || msg.contains("list"),
            "expected helpful error: {msg}"
        );
    }

    #[test]
    fn plan_includes_correct_id() {
        let action = MasInstall {
            name: Some(String::from("Flycut")),
            id: Some(442160987),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.to_string().contains("442160987"));
    }

    #[test]
    fn summarize_single_app_includes_name() {
        let action = MasInstall {
            name: Some(String::from("Better Rename 9")),
            id: Some(414209656),
            list: vec![],
        };
        let s = action.summarize();
        assert!(s.contains("Better Rename 9"), "expected app name in: {s}");
    }

    #[test]
    fn summarize_list_includes_all_names() {
        let action = MasInstall {
            name: None,
            id: None,
            list: vec![
                MasApp {
                    name: String::from("Better Rename 9"),
                    id: 414209656,
                },
                MasApp {
                    name: String::from("Flycut"),
                    id: 442160987,
                },
            ],
        };
        let s = action.summarize();
        assert!(s.contains("Better Rename 9"), "expected first app in: {s}");
        assert!(s.contains("Flycut"), "expected second app in: {s}");
    }

    #[test]
    fn summarize_with_no_apps_returns_generic_message() {
        let action = MasInstall::default();
        let s = action.summarize();
        assert!(
            s.contains("Mac App Store"),
            "expected 'Mac App Store' in: {s}"
        );
    }

    #[test]
    fn app_list_prefers_list_when_both_set() {
        let action = MasInstall {
            name: Some(String::from("ignored")),
            id: Some(999),
            list: vec![MasApp {
                name: String::from("Real"),
                id: 111,
            }],
        };
        let apps = action.app_list();
        assert_eq!(1, apps.len());
        assert_eq!("Real", apps[0].name);
    }
}
