use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

fn fake_plugin_path(home: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    return home.join("Library/Application Support/etch/plugins");
    #[cfg(not(target_os = "macos"))]
    return home.join(".local/share/etch/plugins");
}

#[test]
fn plugin_list_fails_when_no_plugins_installed() {
    let home = TempDir::new().unwrap();
    etch()
        .env("HOME", home.path())
        .args(["plugin", "list"])
        .assert()
        .failure();
}

#[test]
fn plugin_remove_nonexistent_exits_success() {
    let home = TempDir::new().unwrap();
    etch()
        .env("HOME", home.path())
        .args(["plugin", "remove", "no-such-plugin"])
        .assert()
        .success()
        .stdout(contains("does not exist"));
}

#[test]
fn plugin_remove_installed_deletes_directory() {
    let home = TempDir::new().unwrap();
    let plugin_dir = fake_plugin_path(home.path()).join("my-plugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    etch()
        .env("HOME", home.path())
        .args(["plugin", "remove", "my-plugin"])
        .assert()
        .success()
        .stdout(contains("Removed"));

    assert!(!plugin_dir.exists());
}
