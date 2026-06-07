use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

fn filters() -> Vec<(&'static str, &'static str)> {
    vec![
        (r"\d+\.\d+\.\d+", "[VERSION]"),
        (r"/(?:private/)?(?:var/folders|tmp)/\S+", "[TMPDIR]"),
    ]
}

#[test]
fn help() {
    let output = etch().arg("-h").output().unwrap();
    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn apply_help() {
    let output = etch().args(["apply", "--help"]).output().unwrap();
    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn version() {
    let output = etch().arg("version").output().unwrap();
    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn dry_run() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: directory.create\n    path: somedir\n",
    )
    .unwrap();

    let output = etch()
        .current_dir(dir.path())
        .args(["--no-color", "-d", ".", "apply", "--dry-run"])
        .output()
        .unwrap();

    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn doctor_help() {
    let output = etch().args(["doctor", "--help"]).output().unwrap();
    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}

#[test]
fn dry_run_verbose() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: directory.create\n    path: somedir\n",
    )
    .unwrap();

    let output = etch()
        .current_dir(dir.path())
        .args(["--no-color", "-v", "-d", ".", "apply", "--dry-run"])
        .output()
        .unwrap();

    insta::with_settings!({
        filters => filters()
    }, {
        insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
    });
}
