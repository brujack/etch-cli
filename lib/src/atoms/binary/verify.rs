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
        let actual = sha256::try_digest(&self.path)?;
        if actual != self.expected {
            return Err(anyhow::anyhow!(
                "binary.url: sha256 mismatch: expected {} got {}",
                self.expected,
                actual
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_always_returns_should_run_true() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.bin");
        std::fs::write(&path, b"content").unwrap();
        let atom = BinaryVerify {
            path,
            expected: String::from("abc"),
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn verify_correct_sha256_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.bin");
        std::fs::write(&path, b"hello").unwrap();
        let expected = sha256::try_digest(&path).unwrap();
        let mut atom = BinaryVerify { path, expected };
        assert!(atom.execute().is_ok());
    }

    #[test]
    fn verify_wrong_sha256_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.bin");
        std::fs::write(&path, b"hello").unwrap();
        let mut atom = BinaryVerify {
            path,
            expected: String::from(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        };
        let err = atom.execute().err().unwrap();
        let msg = err.to_string();
        assert!(msg.contains("sha256 mismatch"), "msg was: {msg}");
        assert!(
            msg.contains("0000000000000000000000000000000000000000000000000000000000000000"),
            "msg was: {msg}"
        );
        // actual hash of b"hello" must also appear so both hashes are in the message
        assert!(
            msg.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
            "msg was: {msg}"
        );
    }

    #[test]
    fn display_format() {
        let atom = BinaryVerify {
            path: PathBuf::from("/tmp/file.bin"),
            expected: String::from("abc"),
        };
        assert!(format!("{atom}").contains("file.bin"));
    }
}
