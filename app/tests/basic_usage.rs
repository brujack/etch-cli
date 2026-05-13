use tempfile::TempDir;
use utils::*;

mod utils;

#[test]
fn prints_help() {
    run("-h")
        .success()
        .stdout(predicates::str::contains("etch"));
}

#[test]
fn dry_run_doesnt_error() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();
    dir(
        "directory",
        vec![dir(
            "copy",
            vec![
                dir(
                    "files",
                    vec![dir(
                        "mydir",
                        vec![
                            f("file-a", "some content a"),
                            f("file-b", "some other thing"),
                        ],
                    )],
                ),
                f(
                    "main.yaml",
                    r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
                ),
                f(
                    "where_condition.yaml",
                    r#"
where: non.existing.variable == true

actions:
  - action: command.run
    command: echo
    args:
      - hello, world!
                    "#,
                ),
            ],
        )],
    )
    .create_in(&path)
    .expect("should have create test directories");

    let assert = cd(path).run("--no-color -d ./directory apply -m copy --dry-run");

    assert.success();
}

#[test]
fn dry_run_prints_banner() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();
    dir(
        "directory",
        vec![dir(
            "copy",
            vec![
                dir(
                    "files",
                    vec![dir(
                        "mydir",
                        vec![
                            f("file-a", "some content a"),
                            f("file-b", "some other thing"),
                        ],
                    )],
                ),
                f(
                    "main.yaml",
                    r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
                ),
            ],
        )],
    )
    .create_in(&path)
    .expect("should have created test directories");

    cd(path.clone())
        .run("--no-color -d ./directory apply -m copy --dry-run")
        .success()
        .stdout(predicates::str::contains(
            "[dry run] no changes will be made",
        ));

    assert!(
        !path.join("mydircopy").exists(),
        "dry-run must not create mydircopy"
    );
}

#[test]
fn dry_run_shows_action_summary() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();
    dir(
        "directory",
        vec![dir(
            "copy",
            vec![
                dir(
                    "files",
                    vec![dir(
                        "mydir",
                        vec![
                            f("file-a", "some content a"),
                            f("file-b", "some other thing"),
                        ],
                    )],
                ),
                f(
                    "main.yaml",
                    r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
                ),
            ],
        )],
    )
    .create_in(&path)
    .expect("should have created test directories");

    cd(path)
        .run("--no-color -d ./directory apply -m copy --dry-run")
        .success()
        .stdout(predicates::str::contains("2 step(s) would run"));
}

#[test]
fn dry_run_verbose_shows_atoms() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();
    dir(
        "directory",
        vec![dir(
            "copy",
            vec![
                dir(
                    "files",
                    vec![dir(
                        "mydir",
                        vec![
                            f("file-a", "some content a"),
                            f("file-b", "some other thing"),
                        ],
                    )],
                ),
                f(
                    "main.yaml",
                    r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
                ),
            ],
        )],
    )
    .create_in(&path)
    .expect("should have created test directories");

    cd(path)
        .run("--no-color -v -d ./directory apply -m copy --dry-run")
        .success()
        .stdout(predicates::str::contains("  would:"));
}

#[test]
fn dry_run_nothing_to_do() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();

    // Pre-create the directory the manifest will try to create.
    // directory.create resolves path relative to CWD (= path), so
    // path/existing_dir must exist before etch runs.
    std::fs::create_dir(path.join("existing_dir")).expect("could not create existing_dir");

    dir(
        "directory",
        vec![dir(
            "create",
            vec![f(
                "main.yaml",
                r#"
actions:
  - action: directory.create
    path: existing_dir
"#,
            )],
        )],
    )
    .create_in(&path)
    .expect("should have created test directories");

    cd(path)
        .run("--no-color -d ./directory apply -m create --dry-run")
        .success()
        .stdout(predicates::prelude::PredicateBooleanExt::not(
            predicates::str::contains("step(s) would run"),
        ));
}
