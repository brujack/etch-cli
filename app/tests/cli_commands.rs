use assert_cmd::Command;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

#[test]
fn version_prints_version_string() {
    etch()
        .arg("version")
        .assert()
        .success()
        .stdout(predicates::str::contains("."));
}

#[test]
fn gen_completions_bash_produces_output() {
    etch()
        .args(["gen-completions", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("etch"));
}

#[test]
fn gen_completions_zsh_produces_output() {
    etch()
        .args(["gen-completions", "zsh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("etch"));
}

#[test]
fn gen_completions_fish_produces_output() {
    etch()
        .args(["gen-completions", "fish"])
        .assert()
        .success()
        .stdout(predicates::str::contains("etch"));
}

#[test]
fn contexts_exits_successfully() {
    etch().arg("contexts").assert().success();
}

#[test]
fn contexts_output_contains_os_keys() {
    etch()
        .arg("contexts")
        .assert()
        .success()
        .stdout(predicates::str::contains("os"));
}

#[test]
fn plugin_without_subcommand_prints_help() {
    etch().arg("plugin").assert().failure();
}

#[test]
fn help_all_exits_successfully() {
    etch().arg("help-all").assert().success();
}

#[test]
fn help_all_contains_apply_flags() {
    etch()
        .arg("help-all")
        .assert()
        .success()
        .stdout(predicates::str::contains("--dry-run"))
        .stdout(predicates::str::contains("--verbose"));
}

#[test]
fn help_all_contains_update_subcommand_flags() {
    etch()
        .arg("help-all")
        .assert()
        .success()
        .stdout(predicates::str::contains("Update installed tools"));
}

#[test]
fn help_all_does_not_recurse() {
    let out = etch().arg("help-all").output().unwrap().stdout;
    let text = String::from_utf8_lossy(&out);
    // "help-all" appears at most once in the output (its own section header)
    assert!(text.matches("help-all").count() <= 1);
}
