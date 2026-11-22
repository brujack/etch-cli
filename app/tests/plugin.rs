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
fn plugin_remove_dotdot_traversal_rejected() {
    let home = TempDir::new().unwrap();
    etch()
        .env("HOME", home.path())
        .args(["plugin", "remove", "../../etc"])
        .assert()
        .failure()
        .stderr(contains("invalid plugin name"));
}

/// Writes a local plugin whose top level and `plan` function each try to create a marker
/// file through `io` and `os`, guarded so the plugin still loads when those libraries are
/// absent. Returns (manifest dir, list of marker paths the plugin would create).
fn authority_probe_plugin(root: &Path) -> (PathBuf, Vec<PathBuf>) {
    let manifests = root.join("manifests");
    std::fs::create_dir_all(&manifests).unwrap();
    let markers: Vec<PathBuf> = ["top-io", "top-os", "plan-io"]
        .iter()
        .map(|name| root.join(format!("marker-{name}")))
        .collect();
    let lua = format!(
        r#"
local function touch(path)
    if io then
        local f = io.open(path, "w")
        if f then f:write("x") f:close() end
    end
end
touch({top_io:?})
if os then os.execute("touch " .. {top_os:?}) end
return {{
    name = "probe",
    summary = "authority probe",
    actions = {{
        probe = {{
            plan = function() touch({plan_io:?}) end,
            exec = function() end,
        }},
    }},
}}
"#,
        top_io = markers[0].to_str().unwrap(),
        top_os = markers[1].to_str().unwrap(),
        plan_io = markers[2].to_str().unwrap(),
    );
    let plugin_file = root.join("plugin.lua");
    std::fs::write(&plugin_file, lua).unwrap();
    std::fs::write(
        manifests.join("probe.yaml"),
        format!(
            "actions:\n  - action: plugin\n    dir: {}\n    actions:\n      probe: {{}}\n",
            plugin_file.display()
        ),
    )
    .unwrap();
    (manifests, markers)
}

fn etch_in(root: &Path, manifests: &Path) -> Command {
    let mut cmd = etch();
    cmd.env("HOME", root)
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("ETCH_STATE_DIR", root.join("state"))
        .args(["--no-color", "-d"])
        .arg(manifests);
    cmd
}

// audit 2026-11-16 F15: a dry-run must not give plugin code OS authority.
#[test]
fn apply_dry_run_does_not_give_plugin_os_authority() {
    let root = TempDir::new().unwrap();
    let (manifests, markers) = authority_probe_plugin(root.path());

    etch_in(root.path(), &manifests)
        .args(["apply", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("[dry run]"))
        // Proves the plugin was evaluated and planned, so the marker check is not vacuous.
        .stdout(contains("1 step(s) would run"));

    for marker in &markers {
        assert!(
            !marker.exists(),
            "dry-run plugin wrote {}",
            marker.display()
        );
    }
}

// Error path of the restriction: a plugin that uses `io` without guarding it fails to
// load under dry-run, the action is logged as FAILED, and nothing is written. A dry-run
// still exits 0 when an action fails to plan; that is pre-existing and backlogged.
#[test]
fn apply_dry_run_fails_an_unguarded_plugin_without_writing() {
    let root = TempDir::new().unwrap();
    let manifests = root.path().join("manifests");
    std::fs::create_dir_all(&manifests).unwrap();
    let marker = root.path().join("marker-unguarded");
    let plugin_file = root.path().join("plugin.lua");
    std::fs::write(
        &plugin_file,
        format!(
            "local f = io.open({:?}, \"w\") f:write(\"x\") f:close()\n\
             return {{ actions = {{ probe = {{ plan = function() end, exec = function() end }} }} }}\n",
            marker.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        manifests.join("probe.yaml"),
        format!(
            "actions:\n  - action: plugin\n    dir: {}\n    actions:\n      probe: {{}}\n",
            plugin_file.display()
        ),
    )
    .unwrap();

    etch_in(root.path(), &manifests)
        .args(["apply", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("FAILED"))
        .stdout(contains("global 'io'"));

    assert!(
        !marker.exists(),
        "dry-run plugin wrote {}",
        marker.display()
    );
}

// audit 2026-11-16 F15: `etch status` is read-only and plans every action too.
#[test]
fn status_does_not_give_plugin_os_authority() {
    let root = TempDir::new().unwrap();
    let (manifests, markers) = authority_probe_plugin(root.path());

    // The step line proves status evaluated the plugin, so the marker check is not vacuous.
    etch_in(root.path(), &manifests)
        .arg("status")
        .assert()
        .success()
        .stdout(contains("probe"))
        .stdout(contains("1 ok"));

    for marker in &markers {
        assert!(!marker.exists(), "status plugin wrote {}", marker.display());
    }
}

// Positive control for the two tests above: the probe does write its markers when the
// plugin runs with full authority, so their absence under dry-run is meaningful.
#[test]
fn apply_gives_plugin_os_authority() {
    let root = TempDir::new().unwrap();
    let (manifests, markers) = authority_probe_plugin(root.path());

    etch_in(root.path(), &manifests)
        .arg("apply")
        .assert()
        .success();

    for marker in &markers {
        assert!(
            marker.exists(),
            "apply plugin did not write {}",
            marker.display()
        );
    }
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
