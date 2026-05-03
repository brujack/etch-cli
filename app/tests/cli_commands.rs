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
fn contexts_exits_successfully() {
    etch().arg("contexts").assert().success();
}
