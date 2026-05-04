use super::User;
use super::UserVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;

use tracing::debug;

pub type UserAdd = User;

impl Action for UserAdd {
    fn summarize(&self) -> String {
        format!("Adding user: {}", self.username)
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: UserVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        if variant.username.is_empty() {
            return Ok(atoms);
        }

        match uzers::get_user_by_name(&variant.username) {
            Some(_user) => debug!(message = "User already exists", username = ?variant.username),
            None => atoms.append(&mut provider.add_user(&variant, context)?),
        }

        Ok(atoms)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;
    use crate::config::Config;
    use crate::contexts::build_contexts;
    use crate::manifests::Manifest;

    #[test]
    fn user_add_can_be_deserialized() {
        let yaml = r#"
- action: user.add
  username: testuser
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserAdd(action)) => {
                assert_eq!(action.action.username, "testuser");
            }
            _ => panic!("Expected UserAdd"),
        }
    }

    #[test]
    fn user_add_summarize() {
        use super::UserAdd;
        use crate::actions::Action;
        let user = UserAdd {
            username: "alice".to_string(),
            ..Default::default()
        };
        assert!(user.summarize().contains("alice"));
    }

    #[test]
    fn user_add_plan_with_empty_username_returns_empty() {
        use super::UserAdd;
        use crate::actions::Action;
        let user = UserAdd::default(); // empty username
        let manifest = Manifest::default();
        let contexts = build_contexts(&Config::default());
        let steps = user.plan(&manifest, &contexts).unwrap();
        assert!(steps.is_empty());
    }
}
