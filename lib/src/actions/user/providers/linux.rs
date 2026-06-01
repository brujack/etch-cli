use super::UserProvider;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{
    actions::user::add_group::UserAddGroup, actions::user::UserVariant, atoms::command::Exec,
    utilities,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUserProvider {}

impl UserProvider for LinuxUserProvider {
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let mut args: Vec<String> = vec![];
        let cli = match which("useradd") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper user add tool");
                return Ok(vec![]);
            }
        };

        // is a user name isn't provided, cant create a new user
        if user.username.is_empty() {
            warn!(message = "Unable to create user without a username");
            return Ok(vec![]);
        }

        args.push(user.username.clone());

        if !user.home_dir.is_empty() {
            args.push(String::from("-m"));
            args.push(String::from("-d"));
            args.push(user.home_dir.clone());
        }

        if !user.shell.is_empty() {
            args.push(String::from("-s"));
            args.push(user.shell.clone());
        }

        if !user.fullname.is_empty() {
            args.push(String::from("-c"));
            args.push(user.fullname.clone());
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        let mut steps: Vec<Step> = vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
                arguments: vec![].into_iter().chain(args.clone()).collect(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if !user.group.is_empty() {
            let user_groups = UserAddGroup {
                username: user.username.clone(),
                group: user.group.clone(),
                provider: user.provider.clone(),
            };
            for group in self.add_to_group(&user_groups, contexts)? {
                steps.push(group);
            }
        }

        Ok(steps)
    }

    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let cli = match which("usermod") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper user add tool");
                return Ok(vec![]);
            }
        };

        if user.group.is_empty() {
            warn!(message = "No groups listed to add user to");
            return Ok(vec![]);
        }

        if user.username.is_empty() {
            warn!(message = "No user specified to add to group(s)");
            return Ok(vec![]);
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        let mut steps: Vec<Step> = vec![];

        for group in user.group.iter() {
            if Self::user_in_group(&user.username, group) {
                continue;
            }
            steps.push(Step {
                atom: Box::new(Exec {
                    command: cli.display().to_string(),
                    arguments: vec![
                        String::from("-a"),
                        String::from("-G"),
                        String::from(group),
                        user.username.clone(),
                    ],
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
}

impl LinuxUserProvider {
    fn user_in_group(username: &str, group: &str) -> bool {
        std::process::Command::new("id")
            .args(["-nG", username])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .split_whitespace()
                        .any(|g| g == group)
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod test {
    use crate::actions::user::providers::{LinuxUserProvider, UserProvider};
    use crate::actions::user::{add_group::UserAddGroup, UserVariant};
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
    fn test_add_user() {
        with_mock_bins(&["useradd", "usermod"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider
                .add_user(
                    &UserVariant {
                        username: String::from("test"),
                        shell: String::from("sh"),
                        home_dir: String::from("/home/test"),
                        fullname: String::from("Test User"),
                        group: vec![],
                        ..Default::default()
                    },
                    &contexts,
                )
                .unwrap();
            assert_eq!(1, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_user_no_username() {
        with_mock_bins(&["useradd"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider
                .add_user(
                    &UserVariant {
                        username: String::from(""),
                        shell: String::from("sh"),
                        home_dir: String::from("/home/test"),
                        fullname: String::from("Test User"),
                        group: vec![],
                        ..Default::default()
                    },
                    &contexts,
                )
                .unwrap();
            assert_eq!(0, steps.len());
        });
    }

    fn write_id_mock(dir: &std::path::Path, output: &str) {
        let path = dir.join("id");
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' '{}'\nexit 0\n", output),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    #[serial]
    fn test_add_to_group_skips_already_member() {
        let tmp = tempfile::tempdir().unwrap();
        write_mock_bin(tmp.path(), "usermod");
        write_id_mock(tmp.path(), "staff wheel testgroup");
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));

        let user_provider = LinuxUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider
            .add_to_group(
                &UserAddGroup {
                    username: String::from("test"),
                    group: vec![String::from("testgroup")],
                    ..Default::default()
                },
                &contexts,
            )
            .unwrap();

        std::env::set_var("PATH", old_path);
        assert_eq!(0, steps.len(), "already a member — should skip");
    }

    #[test]
    #[serial]
    fn test_add_to_group_skips_already_member_in_list() {
        let tmp = tempfile::tempdir().unwrap();
        write_mock_bin(tmp.path(), "usermod");
        write_id_mock(tmp.path(), "staff wheel testgroup");
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));

        let user_provider = LinuxUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider
            .add_to_group(
                &UserAddGroup {
                    username: String::from("test"),
                    group: vec![
                        String::from("testgroup"),
                        String::from("wheel"),
                        String::from("docker"),
                    ],
                    ..Default::default()
                },
                &contexts,
            )
            .unwrap();

        std::env::set_var("PATH", old_path);
        assert_eq!(1, steps.len(), "only docker is not in the member list");
    }

    #[test]
    #[serial]
    fn test_add_to_group_generates_step_when_id_fails() {
        let tmp = tempfile::tempdir().unwrap();
        write_mock_bin(tmp.path(), "usermod");
        // id exits 1 — user not found yet (new user being created)
        let id_path = tmp.path().join("id");
        std::fs::write(&id_path, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&id_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));

        let user_provider = LinuxUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider
            .add_to_group(
                &UserAddGroup {
                    username: String::from("newuser"),
                    group: vec![String::from("docker")],
                    ..Default::default()
                },
                &contexts,
            )
            .unwrap();

        std::env::set_var("PATH", old_path);
        assert_eq!(1, steps.len(), "id failure should not skip the step");
    }

    #[test]
    #[serial]
    fn test_add_to_group() {
        with_mock_bins(&["usermod", "id"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider
                .add_to_group(
                    &UserAddGroup {
                        username: String::from("test"),
                        group: vec![String::from("testgroup"), String::from("wheel")],
                        ..Default::default()
                    },
                    &contexts,
                )
                .unwrap();
            assert_eq!(2, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_create_user_add_to_group() {
        with_mock_bins(&["useradd", "usermod", "id"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider
                .add_user(
                    &UserVariant {
                        username: String::from("test"),
                        shell: String::from(""),
                        home_dir: String::from(""),
                        fullname: String::from(""),
                        group: vec![String::from("testgroup")],
                        ..Default::default()
                    },
                    &contexts,
                )
                .unwrap();
            assert_eq!(2, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_user_returns_empty_when_useradd_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", tmp.path().display().to_string());

        let user_provider = LinuxUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider
            .add_user(
                &UserVariant {
                    username: String::from("test"),
                    ..Default::default()
                },
                &contexts,
            )
            .unwrap();

        std::env::set_var("PATH", old_path);
        assert_eq!(0, steps.len());
    }
}
