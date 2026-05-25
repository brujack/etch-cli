use crate::atoms::{Atom, Outcome};
use std::path::PathBuf;

pub enum ArchiveFormat {
    Raw,
    TarGz,
    TarXz,
    Zip,
}

pub struct BinaryExtract {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub file: Option<String>,
    pub format: ArchiveFormat,
}

impl std::fmt::Display for BinaryExtract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Extract binary to {}", self.dest.display())
    }
}

impl Atom for BinaryExtract {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: !self.dest.exists(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
