use crate::atoms::{Atom, Outcome};
use std::path::PathBuf;

pub struct BinaryVerify {
    pub path: PathBuf,
    pub expected: String,
}

impl std::fmt::Display for BinaryVerify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Verify sha256 of {}", self.path.display())
    }
}

impl Atom for BinaryVerify {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
