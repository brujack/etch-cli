use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uzers::os::unix::UserExt;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDefaultShell {
    pub shell: String,
    pub username: Option<String>,
}

impl Action for UserDefaultShell {
    fn summarize(&self) -> String {
        match &self.username {
            Some(u) => format!("Setting default shell for {} to {}", u, self.shell),
            None => format!("Setting default shell to {}", self.shell),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.shell.is_empty() {
            anyhow::bail!("user.default_shell requires 'shell' to be specified");
        }

        let current_shell: Option<String> = match &self.username {
            Some(name) => uzers::get_user_by_name(name.as_str())
                .map(|u| u.shell().to_string_lossy().into_owned()),
            None => uzers::get_user_by_uid(uzers::get_current_uid())
                .map(|u| u.shell().to_string_lossy().into_owned()),
        };

        if current_shell.as_deref() == Some(self.shell.as_str()) {
            return Ok(vec![]);
        }

        let mut args = vec![String::from("-s"), self.shell.clone()];
        let privileged = self.username.is_some();
        if let Some(name) = &self.username {
            args.push(name.clone());
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("chsh"),
                arguments: args,
                privileged,
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
    use crate::actions::Action;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn plan_errors_when_shell_empty() {
        let action = UserDefaultShell {
            shell: String::new(),
            username: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn plan_skips_when_shell_matches_current_user() {
        let current_shell = uzers::get_user_by_uid(uzers::get_current_uid())
            .map(|u| u.shell().to_string_lossy().into_owned())
            .unwrap_or_else(|| "/bin/sh".to_string());
        let action = UserDefaultShell {
            shell: current_shell,
            username: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "expected skip when shell already matches");
    }

    #[test]
    fn plan_emits_chsh_when_shell_differs_current_user() {
        let action = UserDefaultShell {
            shell: String::from("/bin/definitely-not-a-shell-xyzzy"),
            username: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let step_display = format!("{}", steps[0].atom);
        assert!(
            step_display.contains("chsh") || step_display.contains("definitely-not"),
            "unexpected step: {step_display}"
        );
    }

    #[test]
    fn plan_emits_privileged_chsh_with_username() {
        let action = UserDefaultShell {
            shell: String::from("/bin/zsh"),
            username: Some(String::from("testuser-xyzzy-nonexistent")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // User doesn't exist → shell won't match → step emitted
        assert_eq!(1, steps.len());
    }

    #[test]
    fn deserialization_with_shell_only() {
        use crate::actions::Actions;
        let yaml = r#"
- action: user.default_shell
  shell: /bin/zsh
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserDefaultShell(action)) => {
                assert_eq!("/bin/zsh", action.action.shell);
                assert!(action.action.username.is_none());
            }
            _ => panic!("expected UserDefaultShell"),
        }
    }

    #[test]
    fn deserialization_with_username() {
        use crate::actions::Actions;
        let yaml = r#"
- action: user.default_shell
  shell: /bin/zsh
  username: alice
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserDefaultShell(action)) => {
                assert_eq!("/bin/zsh", action.action.shell);
                assert_eq!(Some("alice".to_string()), action.action.username);
            }
            _ => panic!("expected UserDefaultShell"),
        }
    }
}
