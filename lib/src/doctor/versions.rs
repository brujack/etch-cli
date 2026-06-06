use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct VersionsCheck;

impl DoctorCheck for VersionsCheck {
    fn name(&self) -> &'static str {
        "Versions"
    }

    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        vec![]
    }
}
