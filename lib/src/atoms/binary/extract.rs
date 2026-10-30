use crate::atoms::{Atom, Outcome};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::{Path, PathBuf};
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

/// Runs `write` against a temp path beside `dest`, then renames it onto
/// `dest`. A failed write removes the temp file, so `dest` never holds a
/// partial binary that `plan()` would take for a finished install.
fn write_atomically(
    dest: &Path,
    write: impl FnOnce(&Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = dest
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("binary: no file name in {}", dest.display()))?;
    let tmp = dest.with_file_name(format!(
        ".{}.etch-partial-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = write(&tmp).and_then(|()| Ok(std::fs::rename(&tmp, dest)?));
    if result.is_err() {
        // Best effort: the write error is the one worth reporting, and a
        // temp file that was never created makes this fail harmlessly.
        let _ = std::fs::remove_file(&tmp);
    }
    result
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
                        write_atomically(&self.dest, |tmp| {
                            entry.unpack(tmp)?;
                            Ok(())
                        })?;
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
                        write_atomically(&self.dest, |tmp| {
                            entry.unpack(tmp)?;
                            Ok(())
                        })?;
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
                write_atomically(&self.dest, |tmp| {
                    std::io::copy(&mut entry, &mut File::create(tmp)?)?;
                    Ok(())
                })?;
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

    /// Bytes that neither gzip nor xz can compress, so truncating the
    /// compressed archive truncates the entry's data and not only padding.
    fn incompressible(len: usize) -> Vec<u8> {
        let mut state: u32 = 0x9E37_79B9;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state.to_le_bytes()[0]
            })
            .collect()
    }

    /// Asserts a failed extract left nothing behind in `dir` except `src`,
    /// so `plan()` schedules the step again on the next run.
    fn assert_failed_extract_left_no_trace(atom: &BinaryExtract, dir: &std::path::Path) {
        assert!(
            !atom.dest.exists(),
            "partial binary left at {}",
            atom.dest.display()
        );
        assert!(atom.plan().unwrap().should_run);
        let leftovers: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p != &atom.src)
            .collect();
        assert!(leftovers.is_empty(), "leftover files: {leftovers:?}");
    }

    #[test]
    fn extract_zip_with_bad_checksum_leaves_no_partial_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let content = incompressible(64 * 1024);
        let src = tmp.path().join("test.zip");
        {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(File::create(&src).unwrap());
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("consul", opts).unwrap();
            zip.write_all(&content).unwrap();
            zip.finish().unwrap();
        }
        // Flip one byte of the stored entry data: the archive still opens,
        // and the CRC mismatch surfaces only after most bytes are copied.
        let mut bytes = std::fs::read(&src).unwrap();
        let at = bytes
            .windows(64)
            .position(|w| w == &content[32 * 1024..32 * 1024 + 64])
            .unwrap();
        bytes[at] ^= 0xFF;
        std::fs::write(&src, bytes).unwrap();
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join("consul"),
            file: Some(String::from("consul")),
            format: ArchiveFormat::Zip,
        };
        assert!(atom.execute().is_err());
        assert_failed_extract_left_no_trace(&atom, tmp.path());
    }

    /// Cuts `src` to half its length, so the tar header still decodes and
    /// the entry's data ends early.
    fn truncate_to_half(src: &std::path::Path) {
        let len = std::fs::metadata(src).unwrap().len();
        File::options()
            .write(true)
            .open(src)
            .unwrap()
            .set_len(len / 2)
            .unwrap();
    }

    #[test]
    fn extract_truncated_tar_gz_leaves_no_partial_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_tar_gz(tmp.path(), "go/bin/go", &incompressible(64 * 1024));
        truncate_to_half(&src);
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join("go"),
            file: Some(String::from("go/bin/go")),
            format: ArchiveFormat::TarGz,
        };
        assert!(atom.execute().is_err());
        assert_failed_extract_left_no_trace(&atom, tmp.path());
    }

    #[test]
    fn extract_truncated_tar_xz_leaves_no_partial_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_tar_xz(tmp.path(), "vault", &incompressible(64 * 1024));
        truncate_to_half(&src);
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join("vault"),
            file: Some(String::from("vault")),
            format: ArchiveFormat::TarXz,
        };
        assert!(atom.execute().is_err());
        assert_failed_extract_left_no_trace(&atom, tmp.path());
    }

    #[test]
    fn extract_onto_existing_directory_removes_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_zip(tmp.path(), "consul", b"consul binary content");
        let dest = tmp.path().join("consul");
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(dest.join("keep"), b"occupied").unwrap();
        let mut atom = BinaryExtract {
            src: src.clone(),
            dest: dest.clone(),
            file: Some(String::from("consul")),
            format: ArchiveFormat::Zip,
        };
        assert!(atom.execute().is_err());
        let mut names: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, ["consul", "test.zip"], "temp file left behind");
    }

    #[test]
    fn extract_to_dest_without_file_name_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let src = make_zip(tmp.path(), "consul", b"consul binary content");
        let mut atom = BinaryExtract {
            src,
            dest: tmp.path().join(".."),
            file: Some(String::from("consul")),
            format: ArchiveFormat::Zip,
        };
        let msg = atom.execute().err().unwrap().to_string();
        assert!(msg.contains("no file name"), "msg was: {msg}");
    }

    #[cfg(unix)]
    #[test]
    fn extract_tar_gz_keeps_executable_mode() {
        use flate2::write::GzEncoder;
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("test.tar.gz");
        let enc = GzEncoder::new(File::create(&src).unwrap(), flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_size(2);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "go", &b"go"[..]).unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let dest = tmp.path().join("bin").join("go");
        let mut atom = BinaryExtract {
            src,
            dest: dest.clone(),
            file: Some(String::from("go")),
            format: ArchiveFormat::TarGz,
        };
        atom.execute().unwrap();
        let mode = std::fs::metadata(&dest).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
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
