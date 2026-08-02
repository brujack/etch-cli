use crate::atoms::{Atom, Outcome};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

#[derive(Debug)]
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

#[derive(Debug)]
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
                if let Some(parent) = self.dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::rename(&self.src, &self.dest)?;
                Ok(())
            }
            ArchiveFormat::TarGz => {
                let file = File::open(&self.src)?;
                let gz = GzDecoder::new(file);
                let mut archive = Archive::new(gz);
                let target = self.file.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("binary.url: 'file' is required for archive extraction")
                })?;
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
            ArchiveFormat::TarXz => {
                let file = File::open(&self.src)?;
                let xz = XzDecoder::new(file);
                let mut archive = Archive::new(xz);
                let target = self.file.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("binary.url: 'file' is required for archive extraction")
                })?;
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
            ArchiveFormat::Zip => {
                let file = File::open(&self.src)?;
                let mut archive = ZipArchive::new(file)?;
                let target = self.file.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("binary.url: 'file' is required for archive extraction")
                })?;
                let mut entry = archive.by_name(target).map_err(|_| {
                    anyhow::anyhow!("binary.url: '{}' not found in archive", target)
                })?;
                if let Some(parent) = self.dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut dest_file = File::create(&self.dest)?;
                std::io::copy(&mut entry, &mut dest_file)?;
                drop(entry);
                drop(archive);
                std::fs::remove_file(&self.src)?;
                Ok(())
            }
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

    fn make_tar_xz(dir: &std::path::Path, entry_name: &str, content: &[u8]) -> PathBuf {
        use std::fs::File;
        use xz2::write::XzEncoder;
        let path = dir.join("test.tar.xz");
        let file = File::create(&path).unwrap();
        let enc = XzEncoder::new(file, 6);
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
    fn extract_tar_xz_extracts_named_file() {
        let tmp = tempfile::tempdir().unwrap();
        let content = b"vault binary content";
        let src = make_tar_xz(tmp.path(), "vault", content);
        let dest = tmp.path().join("vault");
        let mut atom = BinaryExtract {
            src,
            dest: dest.clone(),
            file: Some(String::from("vault")),
            format: ArchiveFormat::TarXz,
        };
        atom.execute().unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), content);
    }

    fn make_zip(dir: &std::path::Path, entry_name: &str, content: &[u8]) -> PathBuf {
        use std::fs::File;
        use std::io::Write;
        let path = dir.join("test.zip");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(entry_name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap();
        path
    }

    #[test]
    fn extract_zip_extracts_named_file() {
        let tmp = tempfile::tempdir().unwrap();
        let content = b"consul binary content";
        let src = make_zip(tmp.path(), "consul", content);
        let dest = tmp.path().join("consul");
        let mut atom = BinaryExtract {
            src,
            dest: dest.clone(),
            file: Some(String::from("consul")),
            format: ArchiveFormat::Zip,
        };
        atom.execute().unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), content);
    }

    #[test]
    fn extract_missing_file_tar_gz_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_tar_gz(tmp.path(), "other/file", b"content");
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join("notfound"),
            file: Some(String::from("notfound")),
            format: ArchiveFormat::TarGz,
        };
        let err = atom.execute().err().unwrap();
        let msg = err.to_string();
        assert!(msg.contains("notfound"), "msg was: {msg}");
        assert!(msg.contains("not found in archive"), "msg was: {msg}");
    }

    #[test]
    fn extract_missing_file_zip_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_zip(tmp.path(), "other", b"content");
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join("notfound"),
            file: Some(String::from("notfound")),
            format: ArchiveFormat::Zip,
        };
        let err = atom.execute().err().unwrap();
        let msg = err.to_string();
        assert!(msg.contains("notfound"), "msg was: {msg}");
        assert!(msg.contains("not found in archive"), "msg was: {msg}");
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
