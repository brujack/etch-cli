use crate::atoms::{AtomStatus, Outcome};

use super::super::Atom;
use super::FileAtom;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Chmod {
    pub path: PathBuf,
    pub mode: u32,
}

impl FileAtom for Chmod {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for Chmod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The permissions on {} need to be set to {:o}",
            self.path.display(),
            self.mode
        )
    }
}

#[cfg(unix)]
use {std::os::unix::prelude::PermissionsExt, tracing::error};

#[cfg(unix)]
impl Atom for Chmod {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }

        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(err) => {
                error!(
                    "Couldn't get metadata for {}, rejecting atom: {}",
                    &self.path.display(),
                    err.to_string()
                );

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        // We expect permissions to come through as if the user was using chmod themselves.
        // This means we support 644/755 decimal syntax. We need to add 0o100000 to support
        // the part of chmod they don't often type.
        Ok(Outcome {
            side_effects: vec![],
            should_run: std::fs::Permissions::from_mode(0o100000 + self.mode).mode()
                != metadata.permissions().mode(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::set_permissions(
            self.path.as_path(),
            std::fs::Permissions::from_mode(self.mode),
        )?;

        Ok(())
    }

    fn status(&self) -> anyhow::Result<AtomStatus> {
        match std::fs::metadata(&self.path) {
            Ok(meta) => {
                let actual = meta.permissions().mode() & 0o7777;
                if actual == self.mode {
                    Ok(AtomStatus::Ok)
                } else {
                    Ok(AtomStatus::Drifted {
                        expected: format!("{:04o}", self.mode),
                        actual: format!("{:04o}", actual),
                    })
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
            Err(e) => Err(anyhow::Error::from(e)
                .context(format!("status: cannot stat {}", self.path.display()))),
        }
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let temp_dir = match tempfile::tempdir() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        match std::fs::File::create(temp_dir.path().join("644")) {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            std::fs::set_permissions(
                temp_dir.path().join("644"),
                std::fs::Permissions::from_mode(0o644)
            )
            .is_ok(),
        );

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o644,
        };

        assert_eq!(false, file_chmod.plan().unwrap().should_run);

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan().unwrap().should_run);
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

        match std::fs::File::create(temp_dir.path().join("644")) {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            std::fs::set_permissions(
                temp_dir.path().join("644"),
                std::fs::Permissions::from_mode(0o644)
            )
            .is_ok(),
        );

        let file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o644,
        };

        assert_eq!(false, file_chmod.plan().unwrap().should_run);

        let mut file_chmod = Chmod {
            path: temp_dir.path().join("644"),
            mode: 0o640,
        };

        assert_eq!(true, file_chmod.plan().unwrap().should_run);
        assert_eq!(true, file_chmod.execute().is_ok());
        assert_eq!(false, file_chmod.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_true_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent");
        let atom = Chmod { path, mode: 0o644 };
        // File doesn't exist — atom assumes another atom will create it
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_format() {
        let atom = Chmod {
            path: std::path::PathBuf::from("/tmp/myfile.txt"),
            mode: 0o755,
        };
        let display = format!("{atom}");
        assert!(display.contains("myfile.txt"));
        assert!(display.contains("755"));
    }

    #[test]
    fn status_missing_when_path_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Chmod {
            path: tmp.path().join("nonexistent"),
            mode: 0o644,
        };
        assert_eq!(atom.status().unwrap(), AtomStatus::Missing);
    }

    #[test]
    fn status_ok_when_permissions_match() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.txt");
        std::fs::write(&path, "x").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let atom = Chmod { path, mode: 0o644 };
        assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
    }

    #[test]
    fn status_drifted_when_permissions_differ() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.txt");
        std::fs::write(&path, "x").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let atom = Chmod { path, mode: 0o600 };
        match atom.status().unwrap() {
            AtomStatus::Drifted { expected, actual } => {
                assert_eq!(expected, "0600");
                assert_eq!(actual, "0644");
            }
            other => panic!("expected Drifted, got {:?}", other),
        }
    }

    #[test]
    fn get_path_returns_path() {
        use super::super::FileAtom;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        let atom = Chmod {
            path: path.clone(),
            mode: 0o644,
        };
        assert_eq!(atom.get_path(), &path);
    }

    #[test]
    fn execute_on_nonexistent_path_returns_err() {
        // plan() returns should_run=true for nonexistent files (assume another atom provides it)
        // but execute() fails because the path doesn't exist
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.txt");
        let mut atom = Chmod { path, mode: 0o644 };
        assert!(atom.plan().unwrap().should_run);
        assert!(atom.execute().is_err());
    }
}
