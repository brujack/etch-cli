use super::GroupProvider;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec, utilities};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxGroupProvider {}

impl GroupProvider for LinuxGroupProvider {
    fn add_group(&self, group: &GroupVariant, contexts: &Contexts) -> Vec<Step> {
        let cli = match which("groupadd") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper group add tool");
                return vec![];
            }
        };

        if group.group_name.is_empty() {
            warn!(message = "Unable to create group without a group name");
            return vec![];
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
                arguments: vec![group.group_name.clone()],
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}

#[cfg(test)]
mod test {
    use crate::actions::group::providers::{GroupProvider, LinuxGroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Contexts;
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    fn write_mock_bin(dir: &std::path::Path, name: &str) {
        let path = dir.join(name);
        std::fs::write(&path, "#!/usr/bin/env bash\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn with_mock_bins<F: FnOnce()>(bins: &[&str], f: F) {
        let tmp = tempfile::tempdir().unwrap();
        for bin in bins {
            write_mock_bin(tmp.path(), bin);
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));
        f();
        std::env::set_var("PATH", old_path);
    }

    #[test]
    #[serial]
    fn test_add_group() {
        with_mock_bins(&["groupadd"], || {
            let group_provider = LinuxGroupProvider {};
            let contexts = Contexts::default();
            let steps = group_provider.add_group(
                &GroupVariant {
                    group_name: String::from("test"),
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(1, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_group_no_group_name() {
        with_mock_bins(&["groupadd"], || {
            let group_provider = LinuxGroupProvider {};
            let contexts = Contexts::default();
            let steps = group_provider.add_group(
                &GroupVariant {
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(0, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_group_returns_empty_when_groupadd_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", tmp.path().display().to_string());

        let group_provider = LinuxGroupProvider {};
        let contexts = Contexts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                group_name: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        std::env::set_var("PATH", old_path);
        assert_eq!(0, steps.len());
    }
}
