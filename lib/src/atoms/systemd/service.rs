use super::super::Atom;
use crate::atoms::Outcome;
use tracing::instrument;

pub struct Service {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<&str> = vec![];
        if let Some(e) = self.enabled {
            parts.push(if e { "enable" } else { "disable" });
        }
        if let Some(s) = self.started {
            parts.push(if s { "start" } else { "stop" });
        }
        write!(f, "SystemdService {} {}", parts.join("+"), self.unit)
    }
}

impl Service {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        let output = std::process::Command::new("systemctl")
            .args(["is-enabled", &self.unit])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout == "enabled")
    }

    fn is_active(&self) -> bool {
        let status = std::process::Command::new("systemctl")
            .args(["is-active", &self.unit])
            .status();
        matches!(status, Ok(s) if s.success())
    }

    fn run_systemctl(&self, subcommand: &str) -> anyhow::Result<()> {
        let (cmd, args) = if self.privileged {
            (
                self.privilege_provider.clone(),
                vec![
                    "systemctl".to_string(),
                    subcommand.to_string(),
                    self.unit.clone(),
                ],
            )
        } else {
            (
                "systemctl".to_string(),
                vec![subcommand.to_string(), self.unit.clone()],
            )
        };
        let status = std::process::Command::new(&cmd).args(&args).status()?;
        if !status.success() {
            anyhow::bail!(
                "systemctl {} {} failed with {}",
                subcommand,
                self.unit,
                status
            );
        }
        Ok(())
    }
}

impl Atom for Service {
    #[instrument(name = "systemd.service.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "systemd.service.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        if let Some(want_enabled) = self.enabled {
            let currently_enabled = self.is_enabled()?;
            if currently_enabled != want_enabled {
                let subcommand = if want_enabled { "enable" } else { "disable" };
                self.run_systemctl(subcommand)?;
            } else {
                tracing::info!(
                    unit = %self.unit,
                    enabled = want_enabled,
                    "systemd.service: enabled already in desired state, skipping"
                );
            }
        }

        if let Some(want_started) = self.started {
            let currently_active = self.is_active();
            if currently_active != want_started {
                let subcommand = if want_started { "start" } else { "stop" };
                self.run_systemctl(subcommand)?;
            } else {
                tracing::info!(
                    unit = %self.unit,
                    started = want_started,
                    "systemd.service: started already in desired state, skipping"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn write_mock_systemctl(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        is_enabled_output: &str,
        is_active_exit: i32,
        action_exit: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("systemctl");
        let content = format!(
            "#!/usr/bin/env bash\n\
             printf 'systemctl %s\\n' \"$*\" >> '{}'\n\
             if [[ \"$1\" == \"is-enabled\" ]]; then\n\
               printf '{}\\n'\n\
               exit 0\n\
             fi\n\
             if [[ \"$1\" == \"is-active\" ]]; then\n\
               exit {}\n\
             fi\n\
             exit {}\n",
            calls_file.display(),
            is_enabled_output,
            is_active_exit,
            action_exit
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn make_service(unit: &str, enabled: Option<bool>, started: Option<bool>) -> Service {
        Service {
            unit: unit.to_string(),
            enabled,
            started,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        }
    }

    #[test]
    fn plan_returns_should_run_true() {
        let atom = make_service("sshd.service", Some(true), None);
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "enabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl enable"),
            "expected no 'enable' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("bluetooth.service", Some(false), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl disable"),
            "expected no 'disable' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_enables_when_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl enable sshd.service"),
            "expected 'enable sshd.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_disables_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "enabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("bluetooth.service", Some(false), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl disable bluetooth.service"),
            "expected 'disable bluetooth.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_started() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 0, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl start"),
            "expected no 'start' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("cups.service", None, Some(false));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl stop"),
            "expected no 'stop' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_starts_when_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl start sshd.service"),
            "expected 'start sshd.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_stops_when_started() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 0, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("cups.service", None, Some(false));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl stop cups.service"),
            "expected 'stop cups.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_handles_enabled_and_started_together() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl enable sshd.service"),
            "expected 'enable' in log, got: {log}"
        );
        assert!(
            log.contains("systemctl start sshd.service"),
            "expected 'start' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_errors_if_enable_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when enable fails, got Ok");
    }

    #[test]
    #[serial]
    fn execute_errors_if_start_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when start fails, got Ok");
    }

    #[test]
    fn display_enable_format() {
        let atom = make_service("sshd.service", Some(true), None);
        let s = format!("{atom}");
        assert!(s.contains("enable"), "expected 'enable' in: {s}");
        assert!(s.contains("sshd.service"), "expected unit in: {s}");
    }

    #[test]
    fn display_enable_start_format() {
        let atom = make_service("sshd.service", Some(true), Some(true));
        let s = format!("{atom}");
        assert!(s.contains("enable"), "expected 'enable' in: {s}");
        assert!(s.contains("start"), "expected 'start' in: {s}");
    }
}
