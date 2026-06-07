use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyChruby {
    /// Ruby version to set as default in ~/.ruby-version.
    /// Verbatim string written to the file (e.g. "ruby-3.3.0").
    /// If omitted, ~/.ruby-version is not written.
    pub default_version: Option<String>,
}

impl Action for RubyChruby {
    fn summarize(&self) -> String {
        match &self.default_version {
            Some(v) => format!("Installing chruby and setting default ruby to {v}"),
            None => String::from("Installing chruby"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }
}
