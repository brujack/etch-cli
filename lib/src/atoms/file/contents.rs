use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::error;

#[derive(Debug)]
pub struct SetContents {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

impl FileAtom for SetContents {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for SetContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents need to be set",
            self.path.display(),
        )
    }
}

impl Atom for SetContents {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }

        let contents = match std::fs::read(&self.path) {
            Ok(contents) => contents,
            Err(error) => {
                error!(
                    "Failed to read contents of {} for diff because {:?}. Skipping",
                    error, self.path
                );

                return Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                });
            }
        };

        Ok(Outcome {
            side_effects: vec![],
            should_run: !contents.eq(&self.contents),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::write(&self.path, &self.contents)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can() {
        let file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("").into_bytes(),
        };

        assert_eq!(false, file_contents.plan().unwrap().should_run);

        let mut file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("Hello, world!").into_bytes(),
        };

        assert_eq!(true, file_contents.plan().unwrap().should_run);
        assert_eq!(true, file_contents.execute().is_ok());
        assert_eq!(false, file_contents.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_true_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.txt");
        let atom = SetContents {
            path,
            contents: b"hello".to_vec(),
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_writes_contents_to_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("output.txt");
        let mut atom = SetContents {
            path: path.clone(),
            contents: b"written content".to_vec(),
        };
        assert!(atom.execute().is_ok());
        assert_eq!(b"written content", std::fs::read(&path).unwrap().as_slice());
    }

    #[test]
    fn display_format() {
        let atom = SetContents {
            path: std::path::PathBuf::from("/tmp/myfile.txt"),
            contents: vec![],
        };
        let display = format!("{atom}");
        assert!(display.contains("myfile.txt"));
    }

    #[test]
    fn get_path_returns_path() {
        use super::super::FileAtom;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        let atom = SetContents {
            path: path.clone(),
            contents: vec![],
        };
        assert_eq!(atom.get_path(), &path);
    }
}
