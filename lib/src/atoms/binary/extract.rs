use crate::atoms::{Atom, Outcome};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;

pub enum ArchiveFormat {
    Raw,
    TarGz,
    TarXz,
    Zip,
}

impl ArchiveFormat {
    pub fn detect(url: &str) -> Self {
        let path = url.split('?').next().unwrap_or(url);
        let path = path.split('#').next().unwrap_or(path);
        if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
            ArchiveFormat::TarGz
        } else if path.ends_with(".tar.xz") {
            ArchiveFormat::TarXz
        } else if path.ends_with(".zip") {
            ArchiveFormat::Zip
        } else {
            ArchiveFormat::Raw
        }
    }
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
        match self.format {
            ArchiveFormat::Raw => {
                std::fs::rename(&self.src, &self.dest)?;
                Ok(())
            }
            ArchiveFormat::TarGz => {
                let file = File::open(&self.src)?;
                let gz = GzDecoder::new(file);
                let mut archive = Archive::new(gz);
                let target = self.file.as_deref().unwrap();
                for entry in archive.entries()? {
                    let mut entry = entry?;
                    let path = entry.path()?.to_string_lossy().into_owned();
                    if path == target {
                        if let Some(parent) = self.dest.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        entry.unpack(&self.dest)?;
                        std::fs::remove_file(&self.src)?;
                        return Ok(());
                    }
                }
                Err(anyhow::anyhow!(
                    "binary.url: '{}' not found in archive",
                    target
                ))
            }
            _ => todo!("TarXz and Zip — implemented in Tasks 5–6"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_detection_tar_gz() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.tar.gz"),
            ArchiveFormat::TarGz
        ));
    }

    #[test]
    fn format_detection_tgz() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.tgz"),
            ArchiveFormat::TarGz
        ));
    }

    #[test]
    fn format_detection_tar_xz() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.tar.xz"),
            ArchiveFormat::TarXz
        ));
    }

    #[test]
    fn format_detection_zip() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.zip"),
            ArchiveFormat::Zip
        ));
    }

    #[test]
    fn format_detection_raw() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool"),
            ArchiveFormat::Raw
        ));
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.exe"),
            ArchiveFormat::Raw
        ));
    }

    #[test]
    fn format_detection_ignores_query_string() {
        assert!(matches!(
            ArchiveFormat::detect("https://example.com/tool.tar.gz?v=1"),
            ArchiveFormat::TarGz
        ));
    }

    #[test]
    fn plan_should_run_when_dest_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = BinaryExtract {
            src: tmp.path().join("src"),
            dest: tmp.path().join("nonexistent"),
            file: None,
            format: ArchiveFormat::Raw,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_dest_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("tool");
        std::fs::write(&dest, b"binary").unwrap();
        let atom = BinaryExtract {
            src: tmp.path().join("src"),
            dest,
            file: None,
            format: ArchiveFormat::Raw,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_format() {
        let atom = BinaryExtract {
            src: PathBuf::from("/tmp/src"),
            dest: PathBuf::from("/usr/local/bin/tool"),
            file: None,
            format: ArchiveFormat::Raw,
        };
        assert!(format!("{atom}").contains("tool"));
    }

    fn make_tar_gz(dir: &std::path::Path, entry_name: &str, content: &[u8]) -> PathBuf {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::fs::File;
        let path = dir.join("test.tar.gz");
        let file = File::create(&path).unwrap();
        let enc = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, entry_name, content).unwrap();
        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();
        path
    }

    #[test]
    fn extract_tar_gz_extracts_named_file() {
        let tmp = tempfile::tempdir().unwrap();
        let content = b"go binary content";
        let src = make_tar_gz(tmp.path(), "go/bin/go", content);
        let dest = tmp.path().join("go");
        let mut atom = BinaryExtract {
            src,
            dest: dest.clone(),
            file: Some(String::from("go/bin/go")),
            format: ArchiveFormat::TarGz,
        };
        atom.execute().unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), content);
    }

    #[test]
    fn extract_raw_renames_src_to_dest() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("tool.etch-tmp");
        let dest = tmp.path().join("tool");
        std::fs::write(&src, b"binary content").unwrap();
        let mut atom = BinaryExtract {
            src: src.clone(),
            dest: dest.clone(),
            file: None,
            format: ArchiveFormat::Raw,
        };
        atom.execute().unwrap();
        assert!(dest.exists());
        assert!(!src.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"binary content");
    }
}
