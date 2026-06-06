use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;
use std::process::Command;

pub struct VersionsCheck;

/// Run a shell command string (for explicit config pins — user-authored in etch.yaml).
fn run_shell_command(cmd: &str) -> Option<String> {
    let output = Command::new("sh").args(["-c", cmd]).output().ok()?;
    capture_output(output)
}

/// Run a binary directly without shell interpretation (for manifest-derived binary atoms).
/// Avoids command injection from manifest-controlled `name` fields.
fn run_binary_version(binary_name: &str) -> Option<String> {
    let output = Command::new(binary_name).arg("--version").output().ok()?;
    capture_output(output)
}

fn capture_output(output: std::process::Output) -> Option<String> {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let trimmed = combined.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn version_result(label: String, output: Option<String>, expected: &str) -> CheckResult {
    match output {
        Some(output) => {
            let passed = output.contains(expected);
            let first_line = output.lines().next().unwrap_or(&output).to_string();
            CheckResult {
                label,
                passed,
                detail: if passed {
                    None
                } else {
                    Some(format!("got \"{first_line}\", expected \"{expected}\""))
                },
            }
        }
        None => CheckResult {
            label,
            passed: false,
            detail: Some(String::from("command not found")),
        },
    }
}

impl DoctorCheck for VersionsCheck {
    fn name(&self) -> &'static str {
        "Versions"
    }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        for manifest in manifests.values() {
            for action in &manifest.actions {
                let (binary_name, version) = match action {
                    Actions::BinaryGitHub(a) => {
                        (a.action.name.as_str(), a.action.version.as_deref())
                    }
                    Actions::BinaryUrl(a) => (a.action.name.as_str(), a.action.version.as_deref()),
                    _ => continue,
                };

                let Some(version) = version else { continue };
                if version == "latest" {
                    continue;
                }

                let label = format!("{binary_name} {version}");
                let output = run_binary_version(binary_name);
                results.push(version_result(label, output, version));
            }
        }

        if let Some(ref doctor) = config.doctor {
            for pin in &doctor.versions {
                let label = format!("{} {}", pin.tool, pin.expected);
                let output = run_shell_command(&pin.command);
                results.push(version_result(label, output, &pin.expected));
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DoctorConfig, VersionPin};
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    fn make_fake_binary(dir: &std::path::Path, name: &str, output: &str) -> std::path::PathBuf {
        let bin = dir.join(name);
        std::fs::write(&bin, format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", output)).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    fn config_with_pin(tool: &str, command: &str, expected: &str) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                versions: vec![VersionPin {
                    tool: tool.to_string(),
                    command: command.to_string(),
                    expected: expected.to_string(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    #[serial]
    fn passes_when_output_contains_expected() {
        let tmp = tempfile::tempdir().unwrap();
        let name = "fake_version_tool_pass_xyz";
        make_fake_binary(tmp.path(), name, "mytool 1.2.3");

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_pin("mytool", name, "1.2.3");
        let results = VersionsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(
            results[0].passed,
            "expected pass, got: {:?}",
            results[0].detail
        );
    }

    #[test]
    #[serial]
    fn fails_when_output_does_not_contain_expected() {
        let tmp = tempfile::tempdir().unwrap();
        let name = "fake_version_tool_fail_xyz";
        make_fake_binary(tmp.path(), name, "mytool 9.9.9");

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_pin("mytool", name, "1.2.3");
        let results = VersionsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0].detail.as_deref().unwrap_or("").contains("got"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn fails_when_command_not_found() {
        let config = config_with_pin(
            "nonexistent_tool",
            "etch_cli_nonexistent_tool_xyz --version",
            "1.0.0",
        );
        let results = VersionsCheck.run(&config, &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0]
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("not found"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn skips_binary_github_with_latest_version() {
        let yaml = concat!(
            "actions:\n",
            "  - action: binary.github\n",
            "    name: fake_bin_latest_xyz\n",
            "    directory: /tmp\n",
            "    repository: owner/repo\n",
            "    version: latest\n",
        );
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);

        let results = VersionsCheck.run(&Config::default(), &manifests);
        assert!(
            results.is_empty(),
            "version: latest should produce no check"
        );
    }

    #[test]
    fn returns_empty_with_no_config_and_no_binary_atoms() {
        let results = VersionsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
