use std::path::Path;

/// Runs `write` against a temp path beside `dest`, then renames it onto
/// `dest`. A failed write removes the temp file, so `dest` never holds a
/// partial file that an atom's `dest.exists()` plan would take for done.
pub(crate) fn write_atomically(
    dest: &Path,
    write: impl FnOnce(&Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = dest
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("no file name in {}", dest.display()))?;
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
