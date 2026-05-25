use crate::atoms::{Atom, Outcome};
use std::path::PathBuf;

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
            _ => todo!("archive formats — implemented in Tasks 4–6"),
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
