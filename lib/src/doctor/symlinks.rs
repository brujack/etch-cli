use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct SymlinkCheck;

impl DoctorCheck for SymlinkCheck {
    fn name(&self) -> &'static str {
        "Symlinks"
    }

    fn run(&self, _config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        for manifest in manifests.values() {
            for action in &manifest.actions {
                let Actions::FileLink(a) = action else {
                    continue;
                };

                let source = a
                    .action
                    .source
                    .as_deref()
                    .or(a.action.from.as_deref())
                    .unwrap_or("(unknown)");
                let target = a.action.target.as_deref().or(a.action.to.as_deref());

                let Some(target) = target else { continue };

                let expanded = shellexpand::tilde(target).into_owned();
                let exists = std::path::Path::new(&expanded).exists();

                results.push(CheckResult {
                    label: format!("{source} → {target}"),
                    passed: exists,
                    detail: if exists {
                        None
                    } else {
                        Some(String::from("target does not exist"))
                    },
                });
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::Manifest;

    fn manifest_with_link(source: &str, target: &str) -> HashMap<String, Manifest> {
        let yaml = format!(
            "actions:\n  - action: file.link\n    source: {source}\n    target: {target}\n"
        );
        let manifest: Manifest = serde_yaml_ng::from_str(&yaml).unwrap();
        let mut map = HashMap::new();
        map.insert("test".to_string(), manifest);
        map
    }

    #[test]
    fn passes_when_target_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source_file");
        let target = tmp.path().join("symlink");
        std::fs::write(&source, b"content").unwrap();
        std::os::unix::fs::symlink(&source, &target).unwrap();

        let manifests =
            manifest_with_link(&source.display().to_string(), &target.display().to_string());
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert_eq!(1, results.len());
        assert!(
            results[0].passed,
            "expected pass, got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn fails_when_target_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source_file");
        let target = tmp.path().join("nonexistent_link");

        let manifests =
            manifest_with_link(&source.display().to_string(), &target.display().to_string());
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0]
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("does not exist"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn returns_empty_for_no_file_link_actions() {
        let yaml = "actions:\n  - action: command.run\n    command: echo hi\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert!(results.is_empty());
    }
}
