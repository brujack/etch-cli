use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn apply(dir: &std::path::Path) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

#[test]
fn apply_with_missing_source_file_exits_nonzero() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("should_not_exist.txt");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: nonexistent.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).failure();
}

#[test]
fn apply_with_malformed_yaml_exits_nonzero() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("test.yaml"), "actions: [unclosed").unwrap();

    apply(dir.path()).failure();
}

#[test]
fn apply_with_unknown_action_exits_nonzero() {
    let dir = tempdir().unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: does.not.exist\n    some: value\n",
    )
    .unwrap();

    apply(dir.path()).failure();
}

#[test]
#[cfg(unix)]
fn apply_to_unwritable_target_dir_exits_nonzero() {
    use std::os::unix::fs::PermissionsExt;

    // Skip if running as root — root bypasses permission enforcement
    let uid_out = std::process::Command::new("id").arg("-u").output().unwrap();
    if String::from_utf8_lossy(&uid_out.stdout).trim() == "0" {
        return;
    }

    let dir = tempdir().unwrap();
    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "content").unwrap();

    let target_dir = dir.path().join("locked");
    fs::create_dir_all(&target_dir).unwrap();
    fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o000)).unwrap();

    let target_path = target_dir.join("dest.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            target_path.display()
        ),
    )
    .unwrap();

    let result = apply(dir.path());

    // Restore perms so tempdir cleanup succeeds
    fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o755)).unwrap();

    result.failure();
}
