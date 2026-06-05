use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplaceRemove {
    pub name: String,
    pub scope: Option<String>,
}

impl Action for ClaudeMarketplaceRemove {
    fn summarize(&self) -> String {
        format!("Removing Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }
}
