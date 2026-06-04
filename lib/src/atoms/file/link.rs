use crate::atoms::{AtomStatus, Outcome};

use super::super::Atom;
use super::FileAtom;
use std::io;
use std::path::PathBuf;
use tracing::{error, warn};

pub struct Link {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl FileAtom for Link {
    fn get_path(&self) -> &PathBuf {
        &self.source
    }
}

impl std::fmt::Display for Link {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} contents needs to be linked from {}", self.target.display(), self.source.display())
    }
}

impl Atom for Link {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // First, ensure source exists and can be linked to
        if !self.source.exists() {
            error!(
                "Cannot plan: source file is missing: {}",
                self.source.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        // Target file doesn't exist, we can run safely
        if !self.target.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }
        // Target file exists, lets check if it's a symlink which can be safely updated
        // or return a false and emit some logging that we can't create the link
        // without purging a file
        let link = match std::fs::read_link(&self.target) {
            Ok(link) => link,
            Err(err) => {
                warn!(
                    "Cannot plan: target already exists and isn't a link: {}",
                    self.target.display()
                );
                error!("Cannot plan: {}", err);

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        let source = self.source.to_owned();

        // If this file doesn't link to what we expect, lets make it so
        Ok(Outcome {
            side_effects: vec![],
            should_run: !link.eq(&source),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    fn status(&self) -> anyhow::Result<AtomStatus> {
        match std::fs::read_link(&self.target) {
            Ok(actual) if actual == self.source => Ok(AtomStatus::Ok),
            Ok(actual) => Ok(AtomStatus::Drifted {
                expected: self.source.display().to_string(),
                actual: actual.display().to_string(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
            Err(e) => Err(anyhow::Error::from(e).context(format!(
                "status: cannot read symlink {}",
                self.target.display()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can() {
        let from_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut atom = Link {
            target: from_dir.path().join("symlink"),
            source: to_file.path().to_path_buf(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);
        assert_eq!(true, atom.execute().is_ok());
        assert_eq!(false, atom.plan().unwrap().should_run);
    }

    #[test]
    fn get_path_returns_source() {
        use super::super::FileAtom;
        let source = std::path::PathBuf::from("/tmp/source");
        let atom = Link {
            source: source.clone(),
            target: std::path::PathBuf::from("/tmp/target"),
        };
        assert_eq!(atom.get_path(), &source);
    }

    #[test]
    fn plan_should_not_run_when_source_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Link {
            source: tmp.path().join("nonexistent_source"),
            target: tmp.path().join("target"),
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_target_is_regular_file() {
        let source_file = tempfile::NamedTempFile::new().unwrap();
        let target_file = tempfile::NamedTempFile::new().unwrap();

        let atom = Link {
            source: source_file.path().to_path_buf(),
            target: target_file.path().to_path_buf(),
        };
        // target exists and is not a symlink
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_format() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Link {
            source: tmp.path().join("src"),
            target: tmp.path().join("tgt"),
        };
        let display = format!("{atom}");
        assert!(display.contains("src"));
        assert!(display.contains("tgt"));
    }

    #[test]
    fn status_missing_when_target_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        std::fs::write(&source, "x").unwrap();
        let atom = Link {
            source,
            target: tmp.path().join("nonexistent"),
        };
        assert_eq!(atom.status().unwrap(), AtomStatus::Missing);
    }

    #[test]
    fn status_ok_when_symlink_correct() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let target = tmp.path().join("link");
        std::fs::write(&source, "x").unwrap();
        std::os::unix::fs::symlink(&source, &target).unwrap();
        let atom = Link { source, target };
        assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
    }

    #[test]
    fn status_drifted_when_symlink_points_elsewhere() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let other = tmp.path().join("other");
        let target = tmp.path().join("link");
        std::fs::write(&source, "x").unwrap();
        std::fs::write(&other, "y").unwrap();
        std::os::unix::fs::symlink(&other, &target).unwrap();
        let atom = Link {
            source: source.clone(),
            target,
        };
        match atom.status().unwrap() {
            AtomStatus::Drifted { expected, actual } => {
                assert_eq!(expected, source.display().to_string());
                assert_eq!(actual, other.display().to_string());
            }
            other => panic!("expected Drifted, got {:?}", other),
        }
    }

    #[test]
    fn plan_should_run_when_symlink_points_elsewhere() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("real_file");
        let other = tmp.path().join("other_file");
        let target = tmp.path().join("symlink");

        std::fs::write(&source, "content").unwrap();
        std::fs::write(&other, "other").unwrap();
        // Create a symlink pointing to 'other', but we want it to point to 'source'
        std::os::unix::fs::symlink(&other, &target).unwrap();

        let atom = Link { source, target };
        assert!(atom.plan().unwrap().should_run);
    }
}
