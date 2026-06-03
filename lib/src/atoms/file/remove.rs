use std::path::PathBuf;

use tracing::error;

use crate::atoms::{Atom, Outcome};

use super::FileAtom;

pub struct Remove {
    pub target: PathBuf,
}

impl FileAtom for Remove {
    fn get_path(&self) -> &std::path::PathBuf {
        &self.target
    }
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file {} needs to be removed", self.target.display())
    }
}

impl Atom for Remove {
    fn plan(&self) -> anyhow::Result<crate::atoms::Outcome> {
        if !self.target.is_file() {
            error!(
                "Cannot plan: target isn`t a file: {}",
                self.target.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        let metadata = self.target.parent().map(|p| p.metadata());

        match metadata {
            Some(v) => {
                if v?.permissions().readonly() {
                    error!(
                        "Cannot plan: Dont have permission to delete {}",
                        self.target.display()
                    );

                    return Ok(Outcome {
                        side_effects: vec![],
                        should_run: false,
                    });
                }
            }
            None => {
                error!(
                    "Cannot plan: Failed to get parent directory of file: {}",
                    self.target.display()
                );

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::remove_file(&self.target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(true, file_remove.plan().unwrap().should_run)
    }

    #[test]
    fn it_can_execute() {
        let target_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut file_remove = Remove {
            target: target_file.path().to_path_buf(),
        };

        assert_eq!(true, file_remove.plan().unwrap().should_run);
        assert_eq!(true, file_remove.execute().is_ok());
        assert_eq!(false, file_remove.plan().unwrap().should_run)
    }

    #[test]
    fn plan_should_not_run_for_nonexistent_path() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Remove {
            target: tmp.path().join("nonexistent"),
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_for_directory() {
        let tmp = tempfile::tempdir().unwrap();
        // target is a directory, not a file
        let atom = Remove {
            target: tmp.path().to_path_buf(),
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_format() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Remove {
            target: tmp.path().join("myfile.txt"),
        };
        let display = format!("{atom}");
        assert!(display.contains("myfile.txt"));
    }

    #[test]
    fn get_path_returns_target() {
        use super::super::FileAtom;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("myfile.txt");
        let atom = Remove {
            target: path.clone(),
        };
        assert_eq!(atom.get_path(), &path);
    }

    #[test]
    fn execute_on_deleted_file_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("target.txt");
        std::fs::write(&path, "content").unwrap();

        let mut atom = Remove {
            target: path.clone(),
        };
        // plan() sees the file and says should_run = true
        assert!(atom.plan().unwrap().should_run);
        // delete the file before execute()
        std::fs::remove_file(&path).unwrap();
        // execute() now fails because the file is gone
        assert!(atom.execute().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn plan_returns_false_when_parent_dir_is_readonly() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("target.txt");
        std::fs::write(&file_path, "content").unwrap();

        let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(dir.path(), perms).unwrap();

        let atom = Remove { target: file_path };
        let result = atom.plan().unwrap();

        // Restore permissions so tempdir cleanup can succeed
        let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(dir.path(), perms).unwrap();

        assert!(!result.should_run);
    }
}
