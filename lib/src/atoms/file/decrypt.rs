use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use age::armor::ArmoredReader;
use age::secrecy::SecretString;
use std::io::Read;
use std::path::PathBuf;
use tracing::error;

pub struct Decrypt {
    pub encrypted_content: Vec<u8>,
    pub passphrase: String,
    pub path: PathBuf,
}

// Hand-written rather than derived: `Atom` requires `Debug`, and a derive here
// would print the passphrase. The `Display` impl below already omits it
// deliberately, printing only the path — Debug must not reopen what Display
// was written to keep closed.
impl std::fmt::Debug for Decrypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decrypt")
            .field(
                "encrypted_content",
                &format_args!("<{} bytes>", self.encrypted_content.len()),
            )
            .field("passphrase", &"<redacted>")
            .field("path", &self.path)
            .finish()
    }
}

impl FileAtom for Decrypt {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for Decrypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The content needs to be decrypted to {}",
            self.path.as_path().display()
        )
    }
}

impl Atom for Decrypt {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }

        // Decrypting file with provided passphrase makes plan work
        match decrypt(&self.passphrase, &self.encrypted_content) {
            Ok(_) => Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            }),
            Err(err) => {
                error!(
                    "Cannot decrypt file {} because {:?}. Skipping.",
                    self.path.display(),
                    err
                );

                Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                })
            }
        }
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let decrypted_content = decrypt(&self.passphrase, &self.encrypted_content)?;

        std::fs::write(&self.path, decrypted_content)?;

        Ok(())
    }
}

fn decrypt(passphrase: &str, encrypted_content: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decryptor = age::Decryptor::new(ArmoredReader::new(encrypted_content))?;
    if !decryptor.is_scrypt() {
        return Err(anyhow::anyhow!("Cannot create passphrase decryptor!"));
    }

    let mut decrypted = vec![];
    let secret = SecretString::from(passphrase.to_owned());
    let identity = age::scrypt::Identity::new(secret);
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;

    #[test]
    fn it_can_plan() -> anyhow::Result<()> {
        // encrypt and write to file
        let passphrase = "Teal'c".to_string();
        let content = b"Shol'va";
        let encrypted_content = encrypt(passphrase.to_owned(), content.to_vec())?;

        // prepare atom
        let mut file = NamedTempFile::new()?;
        file.reopen()?;
        file.write_all(&encrypted_content)?;

        let decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase,
        };

        // plan
        assert_eq!(true, decrypt.plan().unwrap().should_run);

        // prepare another atom
        let another_decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase: "fkbr".to_string(),
        };

        // plan
        assert_eq!(false, another_decrypt.plan().unwrap().should_run);

        Ok(())
    }

    #[test]
    fn it_can_execute() -> anyhow::Result<()> {
        // encrypt and write to file
        let passphrase = "Teal'c".to_string();
        let content = b"Shol'va";
        let encrypted_content = encrypt(passphrase.to_owned(), content.to_vec())?;

        // prepare atom
        let mut file = NamedTempFile::new()?;
        file.reopen()?;
        file.write_all(&encrypted_content)?;

        let mut decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase,
        };

        // plan, execute
        assert_eq!(true, decrypt.plan().unwrap().should_run);
        assert_eq!(true, decrypt.execute().is_ok());

        Ok(())
    }

    #[test]
    fn display_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secret.txt");
        let atom = Decrypt {
            encrypted_content: vec![],
            passphrase: "pass".to_string(),
            path: path.clone(),
        };
        let s = format!("{atom}");
        assert!(s.contains("secret.txt"));
    }

    #[test]
    fn get_path_returns_path() {
        use super::super::FileAtom;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("secret.txt");
        let atom = Decrypt {
            encrypted_content: vec![],
            passphrase: "pass".to_string(),
            path: path.clone(),
        };
        assert_eq!(atom.get_path(), &path);
    }

    #[test]
    fn plan_returns_should_run_true_when_path_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.txt");
        let atom = Decrypt {
            encrypted_content: vec![],
            passphrase: "pass".to_string(),
            path,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    fn encrypt(passphrase: String, content: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let secret = SecretString::from(passphrase);
        let encryptor = age::Encryptor::with_user_passphrase(secret);

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&content)?;
        writer.finish()?;

        Ok(encrypted)
    }

    #[test]
    fn debug_output_redacts_the_passphrase() {
        // Display deliberately prints only the path; Debug must not be the
        // channel that leaks what Display was written to keep out.
        let decrypt = Decrypt {
            encrypted_content: vec![1, 2, 3],
            passphrase: "hunter2-super-secret".to_string(),
            path: PathBuf::from("/tmp/x"),
        };
        let rendered = format!("{decrypt:?}");
        assert!(
            !rendered.contains("hunter2-super-secret"),
            "passphrase leaked into Debug output: {rendered}"
        );
        assert!(
            rendered.contains("redacted"),
            "expected an explicit redaction marker, got: {rendered}"
        );
    }
}
