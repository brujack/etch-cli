use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct CredPermsCheck;

impl DoctorCheck for CredPermsCheck {
    fn name(&self) -> &'static str {
        "Credential dirs"
    }

    fn run(&self, config: &Config, _manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        use std::os::unix::fs::PermissionsExt;

        let dirs = match &config.doctor {
            Some(d) if !d.credential_dirs.is_empty() => &d.credential_dirs,
            _ => return vec![],
        };

        dirs.iter()
            .filter_map(|dir| {
                let expanded = shellexpand::tilde(dir).into_owned();
                let path = std::path::Path::new(&expanded);

                if !path.exists() {
                    return None;
                }

                match std::fs::metadata(path) {
                    Ok(meta) => {
                        let mode = meta.permissions().mode() & 0o777;
                        let passed = mode == 0o700;
                        Some(CheckResult {
                            label: format!("{dir} ({mode:03o})"),
                            passed,
                            detail: if passed {
                                None
                            } else {
                                Some(format!("mode {mode:03o}, expected 700"))
                            },
                        })
                    }
                    Err(e) => Some(CheckResult {
                        label: dir.clone(),
                        passed: false,
                        detail: Some(format!("cannot read metadata: {e}")),
                    }),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DoctorConfig;
    use std::os::unix::fs::PermissionsExt;

    fn config_with_dirs(dirs: &[&str]) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                credential_dirs: dirs.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn passes_for_dir_with_mode_700() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let dir = tmp.path().display().to_string();

        let results = CredPermsCheck.run(&config_with_dirs(&[&dir]), &HashMap::new());
        assert_eq!(1, results.len());
        assert!(
            results[0].passed,
            "expected pass for 700, got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn fails_for_dir_with_wrong_mode() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        let dir = tmp.path().display().to_string();

        let results = CredPermsCheck.run(&config_with_dirs(&[&dir]), &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        let detail = results[0].detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("755"),
            "expected '755' in detail, got: {detail}"
        );
        assert!(
            detail.contains("700"),
            "expected '700' in detail, got: {detail}"
        );
    }

    #[test]
    fn skips_nonexistent_dir() {
        let results = CredPermsCheck.run(
            &config_with_dirs(&["/tmp/etch_nonexistent_cred_dir_xyz"]),
            &HashMap::new(),
        );
        assert!(
            results.is_empty(),
            "nonexistent dir should produce no result"
        );
    }

    #[test]
    fn returns_empty_when_no_credential_dirs_configured() {
        let results = CredPermsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
