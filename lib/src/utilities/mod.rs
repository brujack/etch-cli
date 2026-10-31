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

/// Resolve `binary` against an explicit `path` (a PATH-style list) instead of
/// the process environment, with relative entries taken from the current dir.
pub fn get_binary_path_in(binary: &str, path: &std::ffi::OsStr) -> Result<String, anyhow::Error> {
    let binary = which::which_in(binary, Some(path), std::env::current_dir()?)?
        .to_string_lossy()
        .to_string();

    Ok(binary)
}

pub fn get_privilege_provider(contexts: &Contexts) -> Option<String> {
    let privilege_provider = contexts.get("privilege").and_then(|s| s.first_key_value());

    if let Some(privilege_provider) = privilege_provider {
        return Some(privilege_provider.1.to_string());
    }

    None
}

/// Drops a field's `default` from the JSON Schema. schemars evaluates a serde
/// default while it builds the schema, so a default computed from the host
/// (its cwd or OS) would otherwise be published as a fixed value that differs
/// by the machine generating the schema.
pub(crate) fn without_default(schema: &mut Schema) {
    schema.remove("default");
}

#[derive(Eq, PartialEq, Debug, Clone, Default, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CustomPathBuf(pub Utf8PathBuf);

impl JsonSchema for CustomPathBuf {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("CustomPathBuf")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<PathBuf>()
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
    fn without_default_removes_only_the_default() {
        let mut schema = schemars::json_schema!({"type": "string", "default": "/home/x"});
        without_default(&mut schema);
        assert_eq!(schema, schemars::json_schema!({"type": "string"}));

        // Idempotent, and a schema without a default is left unchanged.
        without_default(&mut schema);
        assert_eq!(schema, schemars::json_schema!({"type": "string"}));
    }

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

    #[test]
    fn custom_path_buf_json_schema_is_a_string() {
        // An independent literal, not a re-derivation through subschema_for:
        // a path serializes as a JSON string, so that is the schema it must publish.
        let mut generator = SchemaGenerator::default();
        let actual = CustomPathBuf::json_schema(&mut generator);
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::json!({"type": "string"})
        );
    }
}
