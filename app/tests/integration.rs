use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

/// Spawn `etch --no-color -d . apply` from `dir`.
fn apply(dir: &std::path::Path) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

// ─── file.link ────────────────────────────────────────────────────────────────

#[test]
fn file_link_creates_symlink() {
    let dir = tempdir().unwrap();

    // Source file must live in a `files/` subdirectory — FileAction::resolve()
    // joins manifest root_dir + "files/" + source.
    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("dotfile.txt"), "hello from etch").unwrap();

    // Target is an absolute path so it doesn't depend on CWD at execution time.
    let target = dir.path().join("linked.txt");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: dotfile.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(target.is_symlink(), "linked.txt should be a symlink");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "hello from etch",
        "symlink should resolve to dotfile.txt content"
    );
}

#[test]
fn file_link_is_idempotent() {
    let dir = tempdir().unwrap();

    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("dotfile.txt"), "hello").unwrap();

    let target = dir.path().join("linked.txt");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: dotfile.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success(); // second apply must also succeed

    assert!(
        target.is_symlink(),
        "symlink should still exist after second apply"
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
}

// ─── file.copy ────────────────────────────────────────────────────────────────

#[test]
fn file_copy_copies_content() {
    let dir = tempdir().unwrap();

    // Source file in files/ subdir (FileAction::resolve() pattern)
    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "copy me").unwrap();

    let dest = dir.path().join("dest.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            dest.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(dest.exists(), "dest.txt should exist after copy");
    assert_eq!(fs::read_to_string(&dest).unwrap(), "copy me");
}

#[test]
fn file_copy_is_idempotent() {
    let dir = tempdir().unwrap();

    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "content").unwrap();

    let dest = dir.path().join("dest.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            dest.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success();

    assert!(dest.exists());
    assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
}
