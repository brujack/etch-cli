use crate::atoms::{AtomStatus, Outcome};

use super::super::Atom;
use std::io;
use std::path::PathBuf;

pub struct Create {
    pub path: PathBuf,
}

impl std::fmt::Display for Create {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The directory {} needs to be created",
            self.path.display(),
        )
    }
}

impl Atom for Create {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: !self.path.exists(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.path)?;

        Ok(())
    }

    fn status(&self) -> anyhow::Result<AtomStatus> {
        match std::fs::metadata(&self.path) {
            Ok(meta) if meta.is_dir() => Ok(AtomStatus::Ok),
            Ok(_) => Ok(AtomStatus::Drifted {
                expected: "directory".to_string(),
                actual: "file".to_string(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
            Err(e) => Err(anyhow::Error::from(e)
                .context(format!("status: cannot stat {}", self.path.display()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_format() {
        let atom = Create {
            path: std::path::PathBuf::from("/tmp/mydir"),
        };
        let display = format!("{atom}");
        assert!(display.contains("mydir"));
    }

    #[test]
    fn it_can_plan() {
        let atom = Create {
            path: std::path::PathBuf::from("/some-random-path"),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);

        let temp = temp_dir();
        let atom = Create { path: temp };
        assert_eq!(false, atom.plan().unwrap().should_run);
    }

    #[test]
    fn status_missing_when_path_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Create {
            path: tmp.path().join("nonexistent"),
        };
        assert_eq!(atom.status().unwrap(), AtomStatus::Missing);
    }

    #[test]
    fn status_ok_when_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Create {
            path: tmp.path().to_path_buf(),
        };
        assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
    }

    #[test]
    fn status_drifted_when_path_is_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file_not_dir");
        std::fs::write(&path, "x").unwrap();
        let atom = Create { path };
        match atom.status().unwrap() {
            AtomStatus::Drifted { expected, actual } => {
                assert_eq!(expected, "directory");
                assert_eq!(actual, "file");
            }
            other => panic!("expected Drifted, got {:?}", other),
        }
    }

    #[test]
    fn it_can_execute() {
        let temp_dir = match tempfile::tempdir() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut atom = Create {
            path: temp_dir.path().join("create-me"),
        };

        assert_eq!(false, temp_dir.path().join("create-me").exists());

        assert_eq!(true, atom.execute().is_ok());
        assert_eq!(false, atom.plan().unwrap().should_run);

        assert_eq!(true, temp_dir.path().join("create-me").is_dir());
    }
}
