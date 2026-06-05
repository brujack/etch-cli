use crate::atoms::Outcome;

use super::super::Atom;
use crate::utilities;
use anyhow::anyhow;
use tracing::debug;

#[derive(Default)]
pub struct Exec {
    pub command: String,
    pub arguments: Vec<String>,
    pub working_dir: Option<String>,
    pub environment: Vec<(String, String)>,
    pub privileged: bool,
    pub privilege_provider: String,
    pub streaming: bool,
    pub(crate) status: ExecStatus,
}

#[derive(Default)]
pub(crate) struct ExecStatus {
    code: i32,
    stdout: String,
    stderr: String,
}

#[allow(dead_code)]
pub fn new_run_command(command: String) -> Exec {
    Exec {
        command,
        ..Default::default()
    }
}

impl Exec {
    fn elevate_if_required(&self) -> (String, Vec<String>) {
        // Depending on the priviledged flag and who who the current user is
        // we can determine if we need to prepend sudo to the command

        let privilege_provider = self.privilege_provider.clone();
        let username = whoami::username().unwrap_or_else(|_| String::from("unknown"));

        match (self.privileged, username.as_str()) {
            // Hasn't requested priviledged, so never try to elevate
            (false, _) => (self.command.clone(), self.arguments.clone()),

            // Requested priviledged, but is already root
            (true, "root") => (self.command.clone(), self.arguments.clone()),

            // Requested privileged, but is not root. sudo strips env vars by
            // default, so inject them via `env KEY=VAL` before the command so
            // they reach the child process regardless of sudoers configuration.
            (true, _) => {
                let mut args: Vec<String> = if !self.environment.is_empty() {
                    let mut env_args = vec!["env".to_string()];
                    env_args.extend(self.environment.iter().map(|(k, v)| format!("{k}={v}")));
                    env_args
                } else {
                    vec![]
                };
                args.push(self.command.clone());
                args.extend(self.arguments.clone());
                (privilege_provider, args)
            }
        }
    }

    fn elevate(&mut self) -> anyhow::Result<()> {
        tracing::info!(
            "Privilege elevation required to run `{} {}`. Validating privileges ...",
            &self.command,
            &self.arguments.join(" ")
        );

        let privilege_provider = utilities::get_binary_path(&self.privilege_provider)?;

        match std::process::Command::new(privilege_provider)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .arg("--validate")
            .output()
        {
            Ok(std::process::Output { status, .. }) if status.success() => Ok(()),

            Ok(std::process::Output { stderr, .. }) => Err(anyhow!(
                "Command requires privilege escalation, but couldn't elevate privileges: {}",
                String::from_utf8(stderr)?
            )),

            Err(err) => Err(anyhow!(
                "Command requires privilege escalation, but couldn't elevate privileges: {err}"
            )),
        }
    }
}

impl std::fmt::Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CommandExec with: privileged={}: {} {}",
            self.privileged,
            self.command,
            self.arguments.join(" ")
        )
    }
}

impl Atom for Exec {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            // Commands may have side-effects, but none that can be "known"
            // without some sandboxed operations to detect filesystem and network
            // affects.
            // Maybe we'll look into this one day?
            side_effects: vec![],
            // Commands should always run, we have no cache-key based
            // determinism atm the moment.
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let (command, arguments) = self.elevate_if_required();

        let command = utilities::get_binary_path(&command)
            .map_err(|_| anyhow!("Command `{command}` not found in path"))?;

        // If we require root, we need to use sudo with inherited IO
        // to ensure the user can respond if prompted for a password
        if command.eq("doas") || command.eq("sudo") || command.eq("run0") {
            match self.elevate() {
                Ok(_) => (),
                Err(err) => {
                    return Err(anyhow!(err));
                }
            }
        }

        if self.streaming {
            let mut child = std::process::Command::new(&command)
                .envs(self.environment.clone())
                .args(&arguments)
                .current_dir(self.working_dir.clone().unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|current_dir| current_dir.display().to_string())
                        .expect("Failed to get current directory")
                }))
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .map_err(|err| anyhow!(err))?;

            match child.wait() {
                Ok(status) if status.success() => Ok(()),
                Ok(status) => Err(anyhow!(
                    "Command failed with exit code: {}",
                    status.code().unwrap_or(1)
                )),
                Err(err) => Err(anyhow!(err)),
            }
        } else {
            match std::process::Command::new(&command)
                .envs(self.environment.clone())
                .args(&arguments)
                .current_dir(self.working_dir.clone().unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|current_dir| current_dir.display().to_string())
                        .expect("Failed to get current directory")
                }))
                .output()
            {
                Ok(output) if output.status.success() => {
                    self.status.stdout = String::from_utf8(output.stdout)?;
                    self.status.stderr = String::from_utf8(output.stderr)?;

                    debug!("stdout: {}", &self.status.stdout);

                    Ok(())
                }

                Ok(output) => {
                    self.status.code = output.status.code().unwrap_or(1);
                    self.status.stdout = String::from_utf8(output.stdout)?;
                    self.status.stderr = String::from_utf8(output.stderr)?;

                    debug!("exit code: {}", &self.status.code);
                    debug!("stdout: {}", &self.status.stdout);
                    debug!("stderr: {}", &self.status.stderr);

                    Err(anyhow!(
                        "Command failed with exit code: {}",
                        self.status.code
                    ))
                }

                Err(err) => Err(anyhow!(err)),
            }
        }
    }

    fn output_string(&self) -> String {
        self.status.stdout.clone()
    }

    fn error_message(&self) -> String {
        self.status.stderr.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::privilege::Privilege;
    use pretty_assertions::assert_eq;
    use serial_test::serial;

    #[test]
    fn defaults() {
        let command_run = Exec {
            ..Default::default()
        };

        assert_eq!(String::from(""), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(false, command_run.privileged);
        assert_eq!(false, command_run.streaming);

        let command_run = new_run_command(String::from("echo"));

        assert_eq!(String::from("echo"), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(false, command_run.privileged);
        assert_eq!(false, command_run.streaming);
    }

    #[test]
    fn elevate() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Sudo.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("sudo"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    #[test]
    fn elevate_doas() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Doas.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("doas"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }
    #[test]
    fn elevate_run0() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Run0.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("run0"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    #[test]
    fn elevate_with_env_injects_env_prefix() {
        let mut command_run = new_run_command(String::from("apt"));
        command_run.arguments = vec![String::from("install"), String::from("--yes")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Sudo.to_string();
        command_run.environment = vec![(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        )];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!("sudo", command);
        assert_eq!(
            vec![
                "env",
                "DEBIAN_FRONTEND=noninteractive",
                "apt",
                "install",
                "--yes"
            ],
            args
        );
    }

    #[test]
    fn elevate_with_empty_env_unchanged() {
        let mut command_run = new_run_command(String::from("apt"));
        command_run.arguments = vec![String::from("install"), String::from("--yes")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Sudo.to_string();
        // no environment set
        let (command, args) = command_run.elevate_if_required();

        assert_eq!("sudo", command);
        assert_eq!(vec!["apt", "install", "--yes"], args);
    }

    #[test]
    fn error_propagation() {
        let mut command_run = new_run_command(String::from("non-existant-command"));
        command_run.execute().expect_err("Command should fail");
    }

    #[test]
    #[serial]
    fn execute_succeeds_for_echo() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            ..Default::default()
        };
        assert!(exec.execute().is_ok());
    }

    #[test]
    #[serial]
    fn execute_fails_for_false_command() {
        let mut exec = Exec {
            command: String::from("false"),
            ..Default::default()
        };
        assert!(exec.execute().is_err());
    }

    #[test]
    #[serial]
    fn execute_with_working_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            working_dir: Some(tmp.path().display().to_string()),
            ..Default::default()
        };
        assert!(exec.execute().is_ok());
    }

    #[test]
    fn plan_always_returns_should_run_true() {
        let exec = Exec {
            command: String::from("echo"),
            ..Default::default()
        };
        assert!(exec.plan().unwrap().should_run);
    }

    #[test]
    #[serial]
    fn output_string_after_execute() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello-output")],
            ..Default::default()
        };
        exec.execute().unwrap();
        assert!(exec.output_string().contains("hello-output"));
    }

    #[test]
    fn output_string_empty_before_execute() {
        let exec = Exec {
            command: String::from("echo"),
            ..Default::default()
        };
        assert_eq!(exec.output_string(), "");
    }

    #[test]
    #[serial]
    fn error_message_empty_after_successful_execute() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hi")],
            ..Default::default()
        };
        exec.execute().unwrap();
        assert_eq!(exec.error_message(), "");
    }

    #[test]
    fn display_format() {
        let exec = Exec {
            command: String::from("ls"),
            arguments: vec![String::from("-la")],
            privileged: true,
            ..Default::default()
        };
        let display = format!("{exec}");
        assert!(display.contains("ls"));
        assert!(display.contains("-la"));
        assert!(display.contains("privileged=true"));
    }

    #[test]
    #[serial]
    fn execute_with_environment() {
        let mut exec = Exec {
            command: String::from("env"),
            environment: vec![(String::from("MY_TEST_VAR"), String::from("test_value"))],
            ..Default::default()
        };
        let result = exec.execute();
        assert!(result.is_ok());
        assert!(exec.output_string().contains("MY_TEST_VAR=test_value"));
    }

    #[test]
    #[serial]
    fn streaming_false_captures_output() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            streaming: false,
            ..Default::default()
        };
        exec.execute().unwrap();
        assert!(exec.output_string().contains("hello"));
    }

    #[test]
    #[serial]
    fn streaming_true_output_string_empty() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            streaming: true,
            ..Default::default()
        };
        exec.execute().unwrap();
        assert_eq!(exec.output_string(), "");
        assert_eq!(exec.error_message(), "");
    }

    #[test]
    #[serial]
    fn streaming_true_succeeds() {
        let mut exec = Exec {
            command: String::from("true"),
            streaming: true,
            ..Default::default()
        };
        assert!(exec.execute().is_ok());
    }

    #[test]
    #[serial]
    fn streaming_true_fails_on_nonzero() {
        let mut exec = Exec {
            command: String::from("false"),
            streaming: true,
            ..Default::default()
        };
        let err = exec.execute().unwrap_err();
        assert!(
            err.to_string().contains("exit code"),
            "expected exit code in error, got: {err}"
        );
    }
}
