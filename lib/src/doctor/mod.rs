use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub label: String,
    pub passed: bool,
    pub detail: Option<String>,
}

pub trait DoctorCheck {
    fn name(&self) -> &'static str;
    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult>;
}

pub mod cred_perms;
pub mod symlinks;
pub mod tools;
pub mod versions;
