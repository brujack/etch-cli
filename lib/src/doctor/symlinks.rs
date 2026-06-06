use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct SymlinkCheck;

impl DoctorCheck for SymlinkCheck {
    fn name(&self) -> &'static str {
        "Symlinks"
    }

    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        vec![]
    }
}
