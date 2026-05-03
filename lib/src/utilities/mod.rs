pub mod lua;
use std::{borrow::Cow, ops::Deref, path::PathBuf};

use crate::contexts::Contexts;

use camino::Utf8PathBuf;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use which::which;

pub fn get_binary_path(binary: &str) -> Result<String, anyhow::Error> {
    let binary = which(binary)?.to_string_lossy().to_string();

    Ok(binary)
}

pub fn get_privilege_provider(contexts: &Contexts) -> Option<String> {
    let privilege_provider = contexts.get("privilege").and_then(|s| s.first_key_value());

    if let Some(privilege_provider) = privilege_provider {
        return Some(privilege_provider.1.to_string());
    }

    None
}

#[derive(Eq, PartialEq, Debug, Clone, Default, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CustomPathBuf(pub Utf8PathBuf);

impl JsonSchema for CustomPathBuf {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("CustomPathBuf")
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        gen.subschema_for::<PathBuf>()
    }
}

impl Deref for CustomPathBuf {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::contexts::Contexts;
    use std::collections::BTreeMap;

    #[test]
    fn get_binary_path_for_existing_command() {
        let result = get_binary_path("echo");
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn get_binary_path_for_nonexistent_command() {
        let result = get_binary_path("this-command-does-not-exist-12345");
        assert!(result.is_err());
    }

    #[test]
    fn get_privilege_provider_when_set() {
        use crate::contexts::build_contexts;

        let config = Config::default();
        let contexts = build_contexts(&config);
        // privilege context should be present
        let result = get_privilege_provider(&contexts);
        // It should return Some with default (sudo)
        assert!(result.is_some());
    }

    #[test]
    fn get_privilege_provider_when_absent() {
        let contexts: Contexts = BTreeMap::new();
        let result = get_privilege_provider(&contexts);
        assert!(result.is_none());
    }

    #[test]
    fn custom_path_buf_deref() {
        let cpb = CustomPathBuf(Utf8PathBuf::from("/tmp/test"));
        let _ = cpb.deref();
    }

    #[test]
    fn custom_path_buf_json_schema() {
        let name = CustomPathBuf::schema_name();
        assert_eq!(name, "CustomPathBuf");
    }
}
