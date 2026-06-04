use crate::atoms::{AtomStatus, Outcome};

use super::super::Atom;
use super::FileAtom;
use anyhow::Context;
use file_diff::diff;
use std::path::{Path, PathBuf};
use tracing::error;

pub struct Copy {
    pub from: PathBuf,
    pub to: PathBuf,
}

impl FileAtom for Copy {
    fn get_path(&self) -> &PathBuf {
        &self.from
    }
}

impl std::fmt::Display for Copy {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} contents needs to be copied from {}", self.to.display(), self.from.display())
    }
}

impl Atom for Copy {
    fn plan(&self) -> anyhow::Result<Outcome> {
        if !self.to.is_file() {
            error!("Cannot plan: target isn't a file: {}", self.to.display());

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        Ok(Outcome {
            side_effects: vec![],
            should_run: !diff(
                &self.from.display().to_string(),
                &self.to.display().to_string(),
            ),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::copy(&self.from, &self.to)?;

        Ok(())
    }

    fn status(&self) -> anyhow::Result<AtomStatus> {
        if !self.to.exists() {
            return Ok(AtomStatus::Missing);
        }
        let src_hash = sha256_file(&self.from)
            .with_context(|| format!("status: cannot hash source {}", self.from.display()))?;
        let dst_hash = sha256_file(&self.to)
            .with_context(|| format!("status: cannot hash target {}", self.to.display()))?;
        if src_hash == dst_hash {
            Ok(AtomStatus::Ok)
        } else {
            Ok(AtomStatus::Drifted {
                expected: src_hash,
                actual: dst_hash,
            })
        }
    }
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    sha256::try_digest(path).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;

    #[test]
    fn it_can_plan() {
        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(true, file_copy.plan().unwrap().should_run);

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: from_file.path().to_path_buf(),
        };

        assert_eq!(false, file_copy.plan().unwrap().should_run);
    }

    #[test]
    fn it_can_execute() {
        use std::io::Write;

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let mut file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(true, file_copy.plan().unwrap().should_run);
        assert_eq!(true, file_copy.execute().is_ok());
        assert_eq!(false, file_copy.plan().unwrap().should_run);
    }

    #[test]
    fn display_format() {
        use super::Copy;
        let atom = Copy {
            from: std::path::PathBuf::from("/src/file.txt"),
            to: std::path::PathBuf::from("/dst/file.txt"),
        };
        let display = format!("{atom}");
        assert!(display.contains("src/file.txt"));
        assert!(display.contains("dst/file.txt"));
    }

    #[test]
    fn get_path_returns_from() {
        use super::super::FileAtom;
        use super::Copy;
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from.txt");
        let to = tmp.path().join("to.txt");
        let atom = Copy {
            from: from.clone(),
            to,
        };
        assert_eq!(atom.get_path(), &from);
    }

    #[test]
    fn status_missing_when_target_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from.txt");
        std::fs::write(&from, "content").unwrap();
        let atom = Copy {
            from,
            to: tmp.path().join("nonexistent.txt"),
        };
        assert_eq!(atom.status().unwrap(), AtomStatus::Missing);
    }

    #[test]
    fn status_ok_when_contents_match() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from.txt");
        let to = tmp.path().join("to.txt");
        std::fs::write(&from, "same content").unwrap();
        std::fs::write(&to, "same content").unwrap();
        let atom = Copy { from, to };
        assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
    }

    #[test]
    fn status_drifted_when_contents_differ() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from.txt");
        let to = tmp.path().join("to.txt");
        std::fs::write(&from, "source content").unwrap();
        std::fs::write(&to, "different content").unwrap();
        let atom = Copy { from, to };
        match atom.status().unwrap() {
            AtomStatus::Drifted { expected, actual } => {
                assert_ne!(expected, actual);
                assert!(!expected.is_empty());
                assert!(!actual.is_empty());
            }
            other => panic!("expected Drifted, got {:?}", other),
        }
    }

    #[test]
    fn it_wont_destroy_directories() {
        let to = match tempfile::TempDir::new() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to.path().to_path_buf(),
        };

        assert_eq!(false, file_copy.plan().unwrap().should_run);
    }
}
