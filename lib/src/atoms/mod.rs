pub mod binary;
pub mod command;
pub mod directory;
pub mod file;
pub mod git;
pub mod http;
pub mod macos;
pub mod plugin;
pub mod status;
pub mod systemd;

pub use status::AtomStatus;

pub enum SideEffect {}

pub struct Outcome {
    pub side_effects: Vec<SideEffect>,
    pub should_run: bool,
}

pub trait Atom: std::fmt::Display {
    // Determine if this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome>;

    // Apply new to old
    fn execute(&mut self) -> anyhow::Result<()>;

    /// Check whether this atom's declared state matches the current machine state.
    /// Default: Unchecked (used for atoms whose state cannot be read, e.g. command.run).
    fn status(&self) -> anyhow::Result<AtomStatus> {
        Ok(AtomStatus::Unchecked)
    }

    // These methods allow for finalizers to query the outcome of the Atom.
    // We'll provide default implementations to allow Atoms to opt in to
    // the queries that make sense for them
    fn output_string(&self) -> String {
        String::from("")
    }

    fn error_message(&self) -> String {
        String::from("")
    }

    fn status_code(&self) -> i32 {
        0
    }
}

pub struct Echo(pub &'static str);

impl Atom for Echo {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn output_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for Echo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Echo: {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_plan_returns_should_run_true() {
        let echo = Echo("test");
        let outcome = echo.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn echo_execute_returns_ok() {
        let mut echo = Echo("test");
        assert!(echo.execute().is_ok());
    }

    #[test]
    fn echo_output_string_returns_content() {
        let echo = Echo("hello");
        assert_eq!(echo.output_string(), "hello");
    }

    #[test]
    fn echo_error_message_returns_empty() {
        let echo = Echo("test");
        assert_eq!(echo.error_message(), "");
    }

    #[test]
    fn echo_status_code_returns_zero() {
        let echo = Echo("test");
        assert_eq!(echo.status_code(), 0);
    }

    #[test]
    fn echo_display_format() {
        let echo = Echo("stargate");
        assert_eq!(format!("{echo}"), "Echo: stargate");
    }

    #[test]
    fn echo_status_returns_unchecked() {
        let echo = Echo("test");
        assert_eq!(echo.status().unwrap(), AtomStatus::Unchecked);
    }

    #[test]
    fn trait_default_output_string_returns_empty() {
        // Tests the default Atom::output_string() impl — Echo overrides it,
        // so we need a minimal implementor that doesn't.
        struct Minimal;
        impl std::fmt::Display for Minimal {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "minimal")
            }
        }
        impl Atom for Minimal {
            fn plan(&self) -> anyhow::Result<Outcome> {
                Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                })
            }
            fn execute(&mut self) -> anyhow::Result<()> {
                Ok(())
            }
        }
        assert_eq!(Minimal.output_string(), "");
    }
}
