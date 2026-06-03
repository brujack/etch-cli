use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn etch() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("etch"));
    cmd.arg("--no-color");
    cmd
}

/// Write a manifest with a directory.create action targeting /tmp.
/// /tmp always exists, so this action is always a no-op: atom.plan().should_run == false.
fn write_noop_manifest(dir: &std::path::Path) {
    fs::write(
        dir.join("test.yaml"),
        "actions:\n  - action: directory.create\n    path: /tmp\n",
    )
    .unwrap();
}

#[test]
fn apply_without_verbose_suppresses_nothing_to_be_done() {
    let dir = tempdir().unwrap();
    write_noop_manifest(dir.path());

    let output = etch()
        .current_dir(dir.path())
        .args(["-d", ".", "apply"])
        .output()
        .unwrap();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("nothing to be done"),
        "Expected 'nothing to be done' suppressed without --verbose, got:\n{combined}"
    );
}

#[test]
fn apply_with_verbose_shows_nothing_to_be_done_with_subject() {
    let dir = tempdir().unwrap();
    write_noop_manifest(dir.path());

    let output = etch()
        .current_dir(dir.path())
        .args(["-d", ".", "apply", "--verbose"])
        .output()
        .unwrap();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("nothing to be done"),
        "Expected 'nothing to be done' shown with --verbose, got:\n{combined}"
    );
    assert!(
        combined.contains("/tmp"),
        "Expected action subject (/tmp) in verbose output, got:\n{combined}"
    );
}

#[test]
fn apply_action_result_shown_without_verbose() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("newdir");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            target.display()
        ),
    )
    .unwrap();

    let output = etch()
        .current_dir(dir.path())
        .args(["-d", ".", "apply"])
        .output()
        .unwrap();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Creating directory"),
        "Expected action summary in output without --verbose, got:\n{combined}"
    );
    assert!(
        target.exists(),
        "directory.create must have actually created the dir"
    );
}

#[test]
fn dry_run_shows_nothing_to_be_done_without_verbose_flag() {
    let dir = tempdir().unwrap();
    write_noop_manifest(dir.path());

    let output = etch()
        .current_dir(dir.path())
        .args(["-d", ".", "apply", "--dry-run"])
        .output()
        .unwrap();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("nothing to be done"),
        "Expected 'nothing to be done' shown in dry-run without --verbose, got:\n{combined}"
    );
}
