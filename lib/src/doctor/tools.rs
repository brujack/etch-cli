use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct ToolsCheck;

impl DoctorCheck for ToolsCheck {
    fn name(&self) -> &'static str {
        "Tools"
    }

    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        vec![]
    }
}
