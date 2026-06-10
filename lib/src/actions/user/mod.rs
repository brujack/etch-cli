pub mod add;
pub mod add_group;
pub mod default_shell;
pub mod providers;

use providers::UserProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    provider: UserProviders,

    #[serde(default)]
    username: String,

    #[serde(default)]
    home_dir: String,

    #[serde(default)]
    fullname: String,

    #[serde(default)]
    shell: String,

    #[serde(default)]
    group: Vec<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, UserVariant>,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserVariant {
    #[serde(default)]
    provider: UserProviders,

    #[serde(default)]
    username: String,

    #[serde(default)]
    home_dir: String,

    #[serde(default)]
    fullname: String,

    #[serde(default)]
    shell: String,

    #[serde(default)]
    group: Vec<String>,
}

impl From<&User> for UserVariant {
    fn from(user: &User) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = user.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return UserVariant {
                provider: user.provider.clone(),
                username: user.username.clone(),
                home_dir: user.home_dir.clone(),
                fullname: user.fullname.clone(),
                shell: user.shell.clone(),
                group: user.group.clone(),
            };
        };

        // .unwrap() is safe here because we checked for None above
        let variant = variant.unwrap();

        debug!(message = "Built Variant", variant = ?variant);

        let mut user = UserVariant {
            provider: user.provider.clone(),
            username: user.username.clone(),
            home_dir: user.home_dir.clone(),
            fullname: user.fullname.clone(),
            shell: user.shell.clone(),
            group: user.group.clone(),
        };

        user.provider = variant.provider.clone();

        user
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn user_add_can_be_deserialized() {
        let yaml = r#"
- action: user.add
  username: john
  shell: /bin/bash
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserAdd(action)) => {
                assert_eq!(action.action.username, "john");
                assert_eq!(action.action.shell, "/bin/bash");
            }
            _ => panic!("Expected UserAdd action"),
        }
    }

    #[test]
    fn user_variant_from_user_no_variants() {
        use super::{User, UserVariant};

        let user = User {
            username: "alice".to_string(),
            home_dir: "/home/alice".to_string(),
            ..Default::default()
        };
        let variant: UserVariant = (&user).into();
        assert_eq!(variant.username, "alice");
        assert_eq!(variant.home_dir, "/home/alice");
    }

    #[test]
    fn user_variant_from_user_with_matching_os_variant() {
        use super::{User, UserVariant};
        use std::collections::HashMap;

        let os = os_info::get();
        let mut variants = HashMap::new();
        variants.insert(
            os.os_type(),
            UserVariant {
                username: "variant_user".to_string(),
                ..Default::default()
            },
        );

        let user = User {
            username: "base_user".to_string(),
            home_dir: "/home/base".to_string(),
            variants,
            ..Default::default()
        };

        let variant: UserVariant = (&user).into();
        // Base user fields are preserved; only provider is overridden by the variant
        assert_eq!(variant.username, "base_user");
        assert_eq!(variant.home_dir, "/home/base");
    }
}
