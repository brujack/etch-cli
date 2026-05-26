use super::super::Atom;
use crate::atoms::Outcome;

pub struct GitConfigUnset {
    /// Args inserted between `git` and `--get`/`--unset`:
    /// global → ["config", "--global"]
    /// local  → ["-C", "/path", "config", "--local"]
    /// system → ["config", "--system"]
    pub config_args: Vec<String>,
    pub key: String,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for GitConfigUnset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitConfigUnset key={}", self.key)
    }
}

impl Atom for GitConfigUnset {
    fn plan(&self) -> anyhow::Result<Outcome> {
        todo!()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
