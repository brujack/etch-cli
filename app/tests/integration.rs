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

// ─── command.run ──────────────────────────────────────────────────────────────

#[test]
fn command_run_creates_file() {
    let dir = tempdir().unwrap();

    let output = dir.path().join("created.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: touch\n    args:\n      - {}\n",
            output.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(output.exists(), "touch should have created the file");
}

#[test]
fn command_run_is_idempotent() {
    let dir = tempdir().unwrap();

    let output = dir.path().join("created.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: touch\n    args:\n      - {}\n",
            output.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success();

    assert!(output.exists());
}

// ─── directory.create ─────────────────────────────────────────────────────────

#[test]
fn directory_create_makes_dir() {
    let dir = tempdir().unwrap();

    let new_dir = dir.path().join("myconfig");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            new_dir.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(
        new_dir.is_dir(),
        "myconfig directory should have been created"
    );
}

#[test]
fn directory_create_is_idempotent() {
    let dir = tempdir().unwrap();

    let new_dir = dir.path().join("myconfig");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            new_dir.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success();

    assert!(
        new_dir.is_dir(),
        "directory should still exist after second apply"
    );
}

// ─── handler/notify ───────────────────────────────────────────────────────────

#[test]
fn handler_runs_when_action_executes() {
    let dir = tempdir().unwrap();
    let sentinel = dir.path().join("handler_ran");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n\
             - action: command.run\n\
             \x20 command: echo\n\
             \x20 args: [hello]\n\
             \x20 notify: [mark-done]\n\
             handlers:\n\
             - name: mark-done\n\
             \x20 action: command.run\n\
             \x20 command: touch\n\
             \x20 args: ['{sentinel}']\n",
            sentinel = sentinel.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(
        sentinel.exists(),
        "handler sentinel should exist after apply"
    );
}

#[test]
fn handler_runs_once_when_notified_by_multiple_actions() {
    let dir = tempdir().unwrap();
    let counter = dir.path().join("count.txt");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n\
             - action: command.run\n\
             \x20 command: echo\n\
             \x20 args: [a]\n\
             \x20 notify: [increment]\n\
             - action: command.run\n\
             \x20 command: echo\n\
             \x20 args: [b]\n\
             \x20 notify: [increment]\n\
             handlers:\n\
             - name: increment\n\
             \x20 action: command.run\n\
             \x20 command: sh\n\
             \x20 args: ['-c', 'echo done >> {counter}']\n",
            counter = counter.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    let content = fs::read_to_string(&counter).unwrap();
    let count = content.lines().filter(|l| *l == "done").count();
    assert_eq!(
        count, 1,
        "handler should run exactly once, ran {count} times"
    );
}

#[test]
fn handler_not_triggered_when_action_skipped() {
    let dir = tempdir().unwrap();
    let guard_file = dir.path().join("guard.txt");
    let sentinel = dir.path().join("handler_ran");

    // Create the guard file so skip_if_exists causes the action to be skipped
    fs::write(&guard_file, "exists").unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n\
             - action: command.run\n\
             \x20 command: echo\n\
             \x20 args: [hello]\n\
             \x20 skip_if_exists: '{guard}'\n\
             \x20 notify: [mark-done]\n\
             handlers:\n\
             - name: mark-done\n\
             \x20 action: command.run\n\
             \x20 command: touch\n\
             \x20 args: ['{sentinel}']\n",
            guard = guard_file.display(),
            sentinel = sentinel.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(
        !sentinel.exists(),
        "handler should NOT run when action was skipped"
    );
}

#[test]
fn handler_not_triggered_when_action_fails() {
    let dir = tempdir().unwrap();
    let sentinel = dir.path().join("handler_ran");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n\
             - action: command.run\n\
             \x20 command: /nonexistent-binary-that-does-not-exist\n\
             \x20 notify: [mark-done]\n\
             handlers:\n\
             - name: mark-done\n\
             \x20 action: command.run\n\
             \x20 command: touch\n\
             \x20 args: ['{sentinel}']\n",
            sentinel = sentinel.display()
        ),
    )
    .unwrap();

    // apply fails because the action errors, but handler must not have run
    apply(dir.path()).failure();

    assert!(
        !sentinel.exists(),
        "handler should NOT run when notifying action failed"
    );
}

#[test]
fn failed_handler_does_not_stop_subsequent_handlers() {
    let dir = tempdir().unwrap();
    let sentinel = dir.path().join("second_handler_ran");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n\
             - action: command.run\n\
             \x20 command: echo\n\
             \x20 args: [hello]\n\
             \x20 notify: [fail-first, mark-done]\n\
             handlers:\n\
             - name: fail-first\n\
             \x20 action: command.run\n\
             \x20 command: /nonexistent-binary-that-does-not-exist\n\
             - name: mark-done\n\
             \x20 action: command.run\n\
             \x20 command: touch\n\
             \x20 args: ['{sentinel}']\n",
            sentinel = sentinel.display()
        ),
    )
    .unwrap();

    // apply fails because first handler errors
    apply(dir.path()).failure();

    // but second handler still ran
    assert!(
        sentinel.exists(),
        "second handler should run even when first handler failed"
    );
}

// ─── file.flags (macOS only) ──────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn ls_flags(path: &std::path::Path) -> String {
    let output = std::process::Command::new("ls")
        .args(["-lO", path.to_str().unwrap()])
        .output()
        .expect("ls -lO failed");
    String::from_utf8(output.stdout).expect("non-UTF8 output")
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_sets_hidden() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [hidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    let flags_output = ls_flags(&target);
    assert!(
        flags_output.contains("hidden"),
        "expected 'hidden' in ls -lO output, got: {flags_output}"
    );

    // Clear flag so tempdir cleanup succeeds
    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().unwrap()])
        .status()
        .unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_is_idempotent() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [hidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success(); // second apply must also succeed

    let flags_output = ls_flags(&target);
    assert!(flags_output.contains("hidden"));

    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().unwrap()])
        .status()
        .unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_clears_hidden() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    // Set the flag manually first
    std::process::Command::new("chflags")
        .args(["hidden", target.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(ls_flags(&target).contains("hidden"));

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [nohidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(
        !ls_flags(&target).contains("hidden"),
        "expected 'hidden' to be cleared"
    );
}
