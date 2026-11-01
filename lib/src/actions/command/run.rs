use crate::contexts::Contexts;
use crate::steps::Step;
use crate::steps::initializers::FileExists;
use crate::{actions::Action, manifests::Manifest, steps, utilities};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,

    /// Working directory. Defaults to the directory etch runs in.
    #[serde(default = "get_cwd")]
    #[schemars(transform = utilities::without_default)]
    pub dir: String,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub skip_if_exists: Option<String>,
}

fn get_false() -> bool {
    false
}

fn get_cwd() -> String {
    std::env::current_dir()
        .map(|current_dir| current_dir.display().to_string())
        .expect("Failed to get current directory")
}

impl Action for RunCommand {
    fn summarize(&self) -> String {
        if self.args.is_empty() {
            format!("Running {}", self.command)
        } else {
            format!("Running {} {}", self.command, self.args.join(" "))
        }
    }

    fn state_key(&self) -> String {
        if let Some(first_arg) = self.args.first() {
            format!("{} {}", self.command, first_arg)
        } else {
            self.command.clone()
        }
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        // The privilege provider would carry env values on its command line
        // (`sudo env K=V ...`), exposing them in ps and sudo logs. Refused for
        // every user so a manifest behaves the same on every machine.
        if self.privileged && !self.env.is_empty() {
            anyhow::bail!(
                "command.run: env: cannot be combined with privileged: true; the privilege provider would expose the values on its command line (ps, sudo logs)"
            );
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        let mut initializers = Vec::new();

        if let Some(path) = &self.skip_if_exists {
            initializers.push(steps::initializers::FlowControl::SkipIf(Box::new(
                FileExists(PathBuf::from(path)),
            )));
        }

        // Sorted so Exec's environment (and any prefix built from it) is deterministic.
        let mut environment: Vec<(String, String)> = self
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        environment.sort();

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                environment,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers,
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn schema_does_not_publish_the_cwd_as_dir_default() {
        let schema = schemars::schema_for!(super::RunCommand);
        let dir = &schema.as_value()["properties"]["dir"];
        assert!(dir.is_object(), "no dir property: {schema:?}");
        assert!(dir.get("default").is_none(), "dir carries a default: {dir}");
    }

    #[test]
    fn it_can_be_deserialize() {
        let yaml = r#"
  - action: command.run
    command: echo
    args:
      - hi
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!("echo", action.action.command);
                assert_eq!("hi", action.action.args.first().unwrap().as_str());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_deserialize_env_vars() {
        let yaml = r#"
  - action: command.run
    command: echo
    args:
      - hi
    env:
        GOROOT: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!("echo", action.action.command);
                assert_eq!("hi", action.action.args.first().unwrap().as_str());

                let value = action.action.env.get("GOROOT");
                assert!(value.is_some());
                assert_eq!("test", value.unwrap());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn plan_returns_one_step_without_initializer_or_finalizer() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            args: vec![String::from("hello")],
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        assert_eq!(0, steps[0].initializers.len());
        assert_eq!(0, steps[0].finalizers.len());
    }

    #[test]
    fn summarize_no_args() {
        use crate::actions::Action;
        let action = super::RunCommand {
            command: String::from("bash"),
            ..Default::default()
        };
        assert_eq!("Running bash", action.summarize());
    }

    #[test]
    fn summarize_with_args() {
        use crate::actions::Action;
        let action = super::RunCommand {
            command: String::from("bash"),
            args: vec![String::from("-c"), String::from("apt-get update")],
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("bash"), "summary: {s}");
        assert!(s.contains("-c"), "summary: {s}");
        assert!(s.contains("apt-get update"), "summary: {s}");
        assert!(!s.contains("command"), "should not say 'command': {s}");
    }

    #[test]
    fn plan_privileged_still_returns_one_step() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            privileged: true,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_env_vars_adds_no_initializer() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let mut env = std::collections::HashMap::new();
        env.insert(String::from("MY_VAR"), String::from("value"));
        let action = super::RunCommand {
            command: String::from("echo"),
            env,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        assert_eq!(0, steps[0].initializers.len());
    }

    #[test]
    fn plan_routes_sorted_env_to_exec_redacted() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        // Eight keys inserted out of order: HashMap iteration order only
        // coincides with sorted order by chance, so dropping `.sort()` fails.
        let env: std::collections::HashMap<String, String> =
            ["H", "C", "F", "A", "E", "B", "G", "D"]
                .iter()
                .map(|k| (k.to_string(), format!("etch-secret-{k}")))
                .collect();
        let action = super::RunCommand {
            command: String::from("echo"),
            env,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].finalizers.is_empty());
        let debug = format!("{:?}", steps[0].atom);
        let env_start = debug.find("environment").expect("environment in Debug");
        let env_debug = &debug[env_start..];
        let order: Vec<&str> = ["A", "B", "C", "D", "E", "F", "G", "H"]
            .iter()
            .copied()
            .filter(|k| env_debug.contains(&format!("\"{k}\"")))
            .collect();
        assert_eq!(8, order.len(), "missing keys: {debug}");
        let mut positions: Vec<usize> = order
            .iter()
            .map(|k| env_debug.find(&format!("\"{k}\"")).unwrap())
            .collect();
        let expected = positions.clone();
        positions.sort_unstable();
        assert_eq!(expected, positions, "keys not sorted A..H: {debug}");
        assert!(!debug.contains("etch-secret-"), "leaked: {debug}");
    }

    #[test]
    fn plan_refuses_env_with_privileged() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            privileged: true,
            env: std::collections::HashMap::from([(String::from("K"), String::from("v"))]),
            ..Default::default()
        };
        let err = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap_err();
        assert!(err.to_string().contains("privileged"), "message: {err}");
    }

    #[test]
    fn plan_allows_privileged_without_env() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            privileged: true,
            ..Default::default()
        };
        assert!(
            action
                .plan(&Manifest::default(), &Contexts::default())
                .is_ok()
        );
    }

    #[test]
    fn plan_allows_unprivileged_with_env() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            env: std::collections::HashMap::from([(String::from("K"), String::from("v"))]),
            ..Default::default()
        };
        assert!(
            action
                .plan(&Manifest::default(), &Contexts::default())
                .is_ok()
        );
    }

    #[test]
    fn it_can_be_deserialized_with_skip_if_exists() {
        let yaml = r#"
- action: command.run
  command: bash
  args:
    - "-c"
    - echo hello
  skip_if_exists: /tmp/test-path
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!(
                    Some(String::from("/tmp/test-path")),
                    action.action.skip_if_exists
                );
            }
            _ => panic!("CommandRun didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_includes_skip_if_initializer_when_set() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            skip_if_exists: Some(String::from("/tmp/test-path")),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        // FileExists (SkipIf) = 1 initializer
        assert_eq!(1, steps[0].initializers.len());
    }
}
