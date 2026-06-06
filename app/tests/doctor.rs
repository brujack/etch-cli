use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

#[test]
fn doctor_help_renders() {
    etch()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(contains("--json"))
        .stdout(contains("--missing-only"));
}

#[test]
fn doctor_with_empty_config_exits_zero() {
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    std::fs::write(&config, "").unwrap();

    etch()
        .args(["-c", &config.display().to_string(), "doctor"])
        .assert()
        .success();
}

#[test]
fn doctor_with_failing_cred_dir_exits_one() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempdir().unwrap();

    let cred_dir = tmp.path().join("bad_cred_dir");
    std::fs::create_dir(&cred_dir).unwrap();
    std::fs::set_permissions(&cred_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    let config = tmp.path().join("etch.yaml");
    std::fs::write(
        &config,
        format!(
            "doctor:\n  credential_dirs:\n    - {}\n",
            cred_dir.display()
        ),
    )
    .unwrap();

    etch()
        .args(["-c", &config.display().to_string(), "doctor"])
        .assert()
        .failure();
}

#[test]
fn doctor_json_flag_outputs_valid_json() {
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    std::fs::write(&config, "").unwrap();

    let output = etch()
        .args(["-c", &config.display().to_string(), "doctor", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    assert!(
        parsed.get("checks").is_some(),
        "JSON should have 'checks' key"
    );
    assert!(
        parsed.get("summary").is_some(),
        "JSON should have 'summary' key"
    );
}

#[test]
fn doctor_missing_only_suppresses_passing_checks() {
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    std::fs::write(&config, "doctor:\n  tools:\n    - sh\n").unwrap();

    let output = etch()
        .args([
            "-c",
            &config.display().to_string(),
            "doctor",
            "--missing-only",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('✓'),
        "expected no passing checks in --missing-only output, got:\n{stdout}"
    );
}
