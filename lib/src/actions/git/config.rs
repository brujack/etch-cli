use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitConfigScope {
    #[default]
    Global,
    Local,
    System,
}

#[allow(dead_code)]
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    pub scope: GitConfigScope,
    pub key: Option<String>,
    pub value: Option<String>,
    pub unset: Option<bool>,
    pub settings: Option<IndexMap<String, String>>,
    pub directory: Option<String>,
}

impl Action for GitConfig {
    fn summarize(&self) -> String {
        todo!()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}
