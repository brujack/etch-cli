use std::{fs::File, path::PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::atoms::{Atom, Outcome};

use super::FileAtom;

pub struct Unarchive {
    pub origin: PathBuf,
    pub dest: PathBuf,
    pub force: bool,
}

impl FileAtom for Unarchive {
    fn get_path(&self) -> &PathBuf {
        &self.origin
    }
}

impl Atom for Unarchive {
    // Determine if this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome> {
        if self.dest.exists() {
            if self.force {
                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: self.origin.exists(),
                });
            }
            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        Ok(Outcome {
            side_effects: vec![],
            should_run: self.origin.exists(),
        })
    }

    // Apply new to old
    fn execute(&mut self) -> anyhow::Result<()> {
        let tar_gz = File::open(&self.origin)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&self.dest)?;
        Ok(())
    }
}

impl std::fmt::Display for Unarchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let origin_path = self.origin.display().to_string();
        let dest_path = self.dest.display().to_string();

        write!(
            f,
            "The archive {origin_path} to be decompressed to {dest_path}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_should_run_when_dest_does_not_exist() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("output");

        let atom = Unarchive {
            origin: fixture,
            dest,
            force: true,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_dest_exists_and_force_false() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();

        let atom = Unarchive {
            origin: fixture,
            dest: tmp.path().to_path_buf(),
            force: false,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_when_dest_exists_and_force_true() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();

        let atom = Unarchive {
            origin: fixture,
            dest: tmp.path().to_path_buf(),
            force: true,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_extracts_archive() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("extracted");
        std::fs::create_dir_all(&dest).unwrap();

        let mut atom = Unarchive {
            origin: fixture,
            dest: dest.clone(),
            force: true,
        };
        assert!(atom.execute().is_ok());
        assert!(dest.join("etch_hello.txt").exists());
    }
}
