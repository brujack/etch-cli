use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

/// Creates the directory structure for a file.link manifest test.
/// Returns the canonicalized absolute path to the source file.
/// root_tmp must stay in scope for the duration of the test.
fn setup_link_manifest(root: &Path, manifest_name: &str, target_path: &Path) -> PathBuf {
    let manifest_dir = root.join("directory").join(manifest_name);
    let files_dir = manifest_dir.join("files");
    std::fs::create_dir_all(&files_dir).unwrap();

    let source_file = files_dir.join("source.txt");
    std::fs::write(&source_file, "link source content").unwrap();
    // Canonicalize resolves macOS /var -> /private/var symlink so the path
    // matches what file.link resolves from the manifest root_dir.
    let source_file = source_file.canonicalize().unwrap();

    let yaml = format!(
        "actions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
        target_path.display()
    );
    std::fs::write(manifest_dir.join("main.yaml"), yaml).unwrap();

    source_file
}

#[test]
fn status_exits_zero_when_all_atoms_ok() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    let source_file = setup_link_manifest(&root, "mymanifest", &target_link);

    // Create the correct symlink — status should be Ok
    std::os::unix::fs::symlink(&source_file, &target_link).unwrap();

    etch()
        .current_dir(&root)
        .args([
            "--no-color",
            "-d",
            "./directory",
            "status",
            "-m",
            "mymanifest",
        ])
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn status_exits_nonzero_when_atom_missing() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    setup_link_manifest(&root, "mymanifest", &target_link);
    // No symlink created — status should be Missing

    etch()
        .current_dir(&root)
        .args([
            "--no-color",
            "-d",
            "./directory",
            "status",
            "-m",
            "mymanifest",
        ])
        .assert()
        .failure()
        .stdout(contains("missing"));
}

#[test]
fn status_exits_nonzero_when_atom_drifted() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    setup_link_manifest(&root, "mymanifest", &target_link);

    // Create a symlink pointing to a different file — status should be Drifted
    let other_file = target_tmp.path().join("other.txt");
    std::fs::write(&other_file, "other content").unwrap();
    std::os::unix::fs::symlink(&other_file, &target_link).unwrap();

    etch()
        .current_dir(&root)
        .args([
            "--no-color",
            "-d",
            "./directory",
            "status",
            "-m",
            "mymanifest",
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("drifted"));
}

#[test]
fn status_unchecked_atom_exits_zero() {
    let root_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let manifest_dir = root.join("directory/mymanifest");
    std::fs::create_dir_all(&manifest_dir).unwrap();

    // command.run atoms return Unchecked — always exit 0
    std::fs::write(
        manifest_dir.join("main.yaml"),
        "actions:\n  - action: command.run\n    command: echo\n    args:\n      - hello\n",
    )
    .unwrap();

    etch()
        .current_dir(&root)
        .args([
            "--no-color",
            "-d",
            "./directory",
            "status",
            "-m",
            "mymanifest",
        ])
        .assert()
        .success()
        .stdout(contains("unchecked"));
}

#[test]
fn status_where_false_skips_manifest() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    let manifest_dir = root.join("directory/mymanifest");
    let files_dir = manifest_dir.join("files");
    std::fs::create_dir_all(&files_dir).unwrap();
    std::fs::write(files_dir.join("source.txt"), "content").unwrap();

    // where: 'false' — manifest is skipped entirely, even though symlink is missing
    let yaml = format!(
        "where: 'false'\nactions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
        target_link.display()
    );
    std::fs::write(manifest_dir.join("main.yaml"), yaml).unwrap();

    etch()
        .current_dir(&root)
        .args([
            "--no-color",
            "-d",
            "./directory",
            "status",
            "-m",
            "mymanifest",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("mymanifest").not());
}
