use super::super::Atom;
use crate::atoms::Outcome;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Debug)]
pub struct Service {
    pub plist: PathBuf,
    pub label: Option<String>,
    pub load: bool,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let action = if self.load { "load" } else { "unload" };
        write!(f, "MacOSService {} {}", action, self.plist.display())
    }
}

impl Service {
    fn resolve_label(&self) -> anyhow::Result<String> {
        if let Some(ref label) = self.label {
            return Ok(label.clone());
        }
        let output = std::process::Command::new("defaults")
            .args(["read", &self.plist.to_string_lossy(), "Label"])
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "defaults read {} Label failed with {}",
                self.plist.display(),
                output.status
            );
        }
        let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if label.is_empty() {
            anyhow::bail!(
                "defaults read returned empty label for {}",
                self.plist.display()
            );
        }
        Ok(label)
    }

    fn is_loaded(&self, label: &str) -> bool {
        let status = std::process::Command::new("launchctl")
            .args(["list", label])
            .status();
        matches!(status, Ok(s) if s.success())
    }

    fn run_launchctl(&self, subcommand: &str) -> anyhow::Result<()> {
        let plist_str = self.plist.to_string_lossy().to_string();
        let username = whoami::username().unwrap_or_else(|_| "unknown".to_string());
        let (cmd, args) = if self.privileged && username != "root" {
            (
                self.privilege_provider.clone(),
                vec![
                    "launchctl".to_string(),
                    subcommand.to_string(),
                    "-w".to_string(),
                    plist_str,
                ],
            )
        } else {
            (
                "launchctl".to_string(),
                vec![subcommand.to_string(), "-w".to_string(), plist_str],
            )
        };
        let status = std::process::Command::new(&cmd).args(&args).status()?;
        if !status.success() {
            anyhow::bail!(
                "launchctl {} -w {} failed with {}",
                subcommand,
                self.plist.display(),
                status
            );
        }
        Ok(())
    }
}

impl Atom for Service {
    #[instrument(name = "macos.service.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "macos.service.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        let label = self.resolve_label()?;
        let currently_loaded = self.is_loaded(&label);

        if self.load == currently_loaded {
            tracing::info!(
                label = %label,
                load = self.load,
                "macos.service: already in desired state, skipping"
            );
            return Ok(());
        }

        let subcommand = if self.load { "load" } else { "unload" };
        self.run_launchctl(subcommand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn write_mock_launchctl(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        list_exit: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nif [[ \"$1\" == \"list\" ]]; then\n  exit {}\nfi\nexit 0\n",
            calls_file.display(),
            list_exit
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_mock_defaults(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        label: &str,
        exit_code: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("defaults");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'defaults %s\\n' \"$*\" >> '{}'\nprintf '{}\\n'\nexit {}\n",
            calls_file.display(),
            label,
            exit_code
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn make_service(plist: &str, label: Option<&str>, load: bool) -> Service {
        Service {
            plist: PathBuf::from(plist),
            label: label.map(String::from),
            load,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        }
    }

    #[test]
    fn plan_returns_should_run_true() {
        let atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            true,
        );
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn display_load_format() {
        let atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            true,
        );
        let s = format!("{atom}");
        assert!(s.contains("load"), "expected 'load' in: {s}");
        assert!(
            s.contains("/Library/LaunchDaemons/com.example.plist"),
            "expected plist path in: {s}"
        );
    }

    #[test]
    fn display_unload_format() {
        let atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            false,
        );
        let s = format!("{atom}");
        assert!(s.contains("unload"), "expected 'unload' in: {s}");
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 0 = loaded; load: true => already in desired state
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            true,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("load -w"),
            "expected no 'load -w' when already loaded, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = not loaded; load: false => already in desired state
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            false,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("unload -w"),
            "expected no 'unload -w' when already unloaded, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_loads_when_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = not loaded; load: true => should load
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            true,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("load -w"),
            "expected 'load -w' in log, got: {log}"
        );
        assert!(
            log.contains("/Library/LaunchDaemons/com.example.plist"),
            "expected plist path in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_unloads_when_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 0 = loaded; load: false => should unload
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            false,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("unload -w"),
            "expected 'unload -w' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_uses_explicit_label_without_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = not loaded; load: true
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);
        // No defaults mock needed — but write one anyway to detect if it's called
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        // label: Some(...) => should NOT call defaults
        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example.explicit"),
            true,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("defaults"),
            "expected no 'defaults' call when label is explicit, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_calls_defaults_when_label_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = not loaded; load: true
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example.fromdefaults", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        // label: None => should call defaults
        let mut atom = make_service("/Library/LaunchDaemons/com.example.plist", None, true);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("defaults"),
            "expected 'defaults' call when label is None, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_errors_if_defaults_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);
        // defaults exits 1 => error
        write_mock_defaults(mock_dir.path(), &calls_file, "", 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("/Library/LaunchDaemons/com.example.plist", None, true);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when defaults fails, got Ok");
    }

    #[test]
    #[serial]
    fn execute_errors_if_launchctl_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");

        // launchctl always exits 1
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.path().join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nexit 1\n",
            calls_file.display()
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        write_mock_defaults(mock_dir.path(), &calls_file, "com.example", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        // load: true, launchctl list exits 1 (not loaded) => will try to load => fails
        let mut atom = make_service(
            "/Library/LaunchDaemons/com.example.plist",
            Some("com.example"),
            true,
        );
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when launchctl fails, got Ok");
    }
}
