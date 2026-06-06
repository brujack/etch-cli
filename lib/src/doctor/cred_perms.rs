use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct CredPermsCheck;

impl DoctorCheck for CredPermsCheck {
    fn name(&self) -> &'static str {
        "Credential dirs"
    }

    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        vec![]
    }
}
