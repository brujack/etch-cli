use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn etch(stash_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("etch"));
    cmd.env("ETCH_STASH_DIR", stash_dir);
    cmd.env("ETCH_STATE_DIR", stash_dir); // avoid polluting real state
    cmd
}

/// Write a file.copy manifest into `manifest_dir`.
fn setup_copy_manifest(
    manifest_dir: &std::path::Path,
    source_content: &str,
    target: &std::path::Path,
) {
    let files_dir = manifest_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), source_content).unwrap();
    fs::write(
        manifest_dir.join("main.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            target.display()
        ),
    )
    .unwrap();
}

/// Return the sha256 hex key that the stash store uses for `target`.
///
/// StashStore::stash() calls `sha256::digest(path.to_string_lossy().as_ref())`
/// on exactly the path string passed in by the Stash atom.  The Stash atom
/// receives the path as written in the manifest YAML (`to:` field), so it
/// is the same string as `target.display()` — not the canonicalized form.
fn stash_hex(target: &std::path::Path) -> String {
    sha256::digest(target.to_string_lossy().as_ref())
}

#[test]
fn apply_file_copy_stashes_pre_existing_file() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original content").unwrap();
    setup_copy_manifest(manifest_dir.path(), "new content", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    let hex = stash_hex(&target);
    let hex_dir = stash_dir.path().join(&hex);
    assert!(hex_dir.is_dir(), "hex dir must exist after stash");

    let stash_files: Vec<_> = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .collect();
    assert_eq!(stash_files.len(), 1, "exactly one stash file");

    let saved = fs::read_to_string(stash_files[0].path()).unwrap();
    assert_eq!(
        saved, "original content",
        "stash must contain pre-apply content"
    );
}

#[test]
fn apply_twice_creates_two_stashes() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original").unwrap();
    setup_copy_manifest(manifest_dir.path(), "new", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    // Change source so second apply sees a different file to copy
    let files_dir = manifest_dir.path().join("files");
    fs::write(files_dir.join("source.txt"), "newer").unwrap();

    // Sleep to guarantee distinct second-resolution timestamps in stash filenames
    std::thread::sleep(std::time::Duration::from_millis(1100));

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    let hex = stash_hex(&target);
    let hex_dir = stash_dir.path().join(&hex);
    let count = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .count();
    assert_eq!(count, 2, "two applies must produce two stashes");
}

#[test]
fn rollback_path_restores_original_content() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "original").unwrap();
    setup_copy_manifest(manifest_dir.path(), "replaced", &target);

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&target).unwrap(), "replaced");

    // Mutate after apply to simulate drift
    fs::write(&target, "mutated").unwrap();

    // Pass the same raw path that was written to the manifest YAML — the stash
    // key is sha256 of that exact string, so rollback --path must match it.
    etch(stash_dir.path())
        .args([
            "--no-color",
            "rollback",
            "--path",
            &target.display().to_string(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "original",
        "rollback must restore pre-apply content"
    );
}

#[test]
fn prune_limits_stash_count_to_three() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();
    let target = target_dir.path().join("config.txt");

    fs::write(&target, "v0").unwrap();
    let files_dir = manifest_dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();

    for i in 1..=4u8 {
        fs::write(files_dir.join("source.txt"), format!("v{}", i)).unwrap();
        fs::write(
            manifest_dir.path().join("main.yaml"),
            format!(
                "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
                target.display()
            ),
        )
        .unwrap();
        etch(stash_dir.path())
            .current_dir(manifest_dir.path())
            .args(["--no-color", "-d", ".", "apply"])
            .assert()
            .success();
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    let hex = stash_hex(&target);
    let hex_dir = stash_dir.path().join(&hex);
    let count = fs::read_dir(&hex_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".meta.yaml"))
        .count();
    assert_eq!(count, 3, "keep=3 default: 4 applies must leave 3 stashes");
}

#[test]
fn file_link_creates_no_stash() {
    let stash_dir = tempdir().unwrap();
    let manifest_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    let files_dir = manifest_dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "link source").unwrap();
    let target = target_dir.path().join("linked.txt");

    fs::write(
        manifest_dir.path().join("main.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    etch(stash_dir.path())
        .current_dir(manifest_dir.path())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
        .success();

    // Stash dir must contain no hex subdirectories — file.link never stashes.
    // state.yaml may be present (written by ETCH_STATE_DIR) — exclude it.
    let has_hex_dirs = fs::read_dir(stash_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false));
    assert!(
        !has_hex_dirs,
        "file.link must not create any stash hex directory"
    );
}

#[test]
fn rollback_list_exits_zero_with_no_stashes() {
    let stash_dir = tempdir().unwrap();

    etch(stash_dir.path())
        .args(["--no-color", "rollback"])
        .assert()
        .success();
}

#[test]
fn rollback_path_unknown_exits_nonzero() {
    let stash_dir = tempdir().unwrap();

    etch(stash_dir.path())
        .args(["--no-color", "rollback", "--path", "/no/stash/for/this"])
        .assert()
        .failure();
}
