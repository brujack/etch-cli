use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

/// Spawn `etch --no-color -d . apply` from `dir`, redirecting state to `state_dir`.
fn apply_with_state(
    dir: &std::path::Path,
    state_dir: &std::path::Path,
) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .env("ETCH_STATE_DIR", state_dir)
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

/// Like `apply_with_state`, but also sets `envs` on the spawned etch process.
fn apply_with_state_env(
    dir: &std::path::Path,
    state_dir: &std::path::Path,
    envs: &[(&str, &str)],
) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .env("ETCH_STATE_DIR", state_dir)
        .envs(envs.iter().copied())
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

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

// ─── command.run env ──────────────────────────────────────────────────────────

/// A `command.run` action that records `$ETCH_SPEC_PROBE` (or `unset`) in `<dir>/probe`.
/// Written to a file because non-streaming Exec logs stdout only at debug level.
fn env_probe_action(dir: &std::path::Path) -> String {
    format!(
        "  - action: command.run\n    command: sh\n    args:\n      - -c\n      - printf %s \"${{ETCH_SPEC_PROBE-unset}}\" > {}/probe\n",
        dir.display()
    )
}

#[test]
fn command_run_env_restores_inherited_value() {
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: \"true\"\n    env:\n      ETCH_SPEC_PROBE: x\n{}",
            env_probe_action(dir.path())
        ),
    )
    .unwrap();

    apply_with_state_env(dir.path(), state.path(), &[("ETCH_SPEC_PROBE", "orig")]).success();

    assert_eq!(
        fs::read_to_string(dir.path().join("probe")).unwrap(),
        "orig"
    );
}

#[test]
fn command_run_env_value_reaches_child() {
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: sh\n    args:\n      - -c\n      - printf %s \"${{ETCH_SPEC_PROBE-unset}}\" > {}/probe\n    env:\n      ETCH_SPEC_PROBE: \"x\"\n",
            dir.path().display()
        ),
    )
    .unwrap();

    apply_with_state_env(dir.path(), state.path(), &[("ETCH_SPEC_PROBE", "orig")]).success();

    assert_eq!(fs::read_to_string(dir.path().join("probe")).unwrap(), "x");
}

#[test]
fn command_run_env_with_privileged_is_refused() {
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    let marker = dir.path().join("ran");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: touch\n    args:\n      - {}\n    privileged: true\n    env:\n      ETCH_SPEC_PROBE: x\n",
            marker.display()
        ),
    )
    .unwrap();

    let assert = apply_with_state_env(dir.path(), state.path(), &[]).failure();

    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("cannot be combined with privileged"),
        "refusal text missing: {combined}"
    );
    assert!(!marker.exists(), "command ran despite refused env");
}

#[test]
fn command_run_env_does_not_leak_after_failure() {
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: \"false\"\n    env:\n      ETCH_SPEC_PROBE: x\n{}",
            env_probe_action(dir.path())
        ),
    )
    .unwrap();

    apply_with_state_env(dir.path(), state.path(), &[("ETCH_SPEC_PROBE", "orig")]).failure();

    assert_eq!(
        fs::read_to_string(dir.path().join("probe")).unwrap(),
        "orig"
    );
}

#[test]
fn command_run_env_path_does_not_break_later_actions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    let ok = dir.path().join("ok");
    fs::write(&ok, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&ok, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: {}\n    env:\n      PATH: {}\n{}",
            ok.display(),
            dir.path().display(),
            env_probe_action(dir.path())
        ),
    )
    .unwrap();

    apply_with_state_env(dir.path(), state.path(), &[("ETCH_SPEC_PROBE", "orig")]).success();

    assert_eq!(
        fs::read_to_string(dir.path().join("probe")).unwrap(),
        "orig"
    );
}

#[test]
fn command_run_env_path_resolves_same_step_program() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();
    let mock = dir.path().join("etch-spec-mock");
    fs::write(
        &mock,
        format!("#!/bin/sh\n: > {}/ran\n", dir.path().display()),
    )
    .unwrap();
    fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
    let real_path = std::env::var("PATH").unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: etch-spec-mock\n    env:\n      PATH: \"{}:{}\"\n",
            dir.path().display(),
            real_path
        ),
    )
    .unwrap();

    apply_with_state_env(dir.path(), state.path(), &[]).success();

    assert!(dir.path().join("ran").exists(), "mock did not run");
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
        .args(["-lO", path.to_str().expect("path is valid UTF-8")])
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

    // cleanup BEFORE assert so it runs even if assert panics
    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().expect("path is valid UTF-8")])
        .status()
        .expect("chflags cleanup failed");

    assert!(
        flags_output.contains("hidden"),
        "expected 'hidden' in ls -lO output, got: {flags_output}"
    );
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

    // cleanup BEFORE assert so it runs even if assert panics
    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().expect("path is valid UTF-8")])
        .status()
        .expect("chflags cleanup failed");

    assert!(flags_output.contains("hidden"));
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

    // safety: ensure flag is cleared regardless of apply outcome
    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().expect("path is valid UTF-8")])
        .status()
        .ok(); // ignore error — file may already be clear

    assert!(
        !ls_flags(&target).contains("hidden"),
        "expected 'hidden' to be cleared"
    );
}

// ─── state manifest ───────────────────────────────────────────────────────────

#[test]
fn apply_writes_state_file_with_file_copy_key() {
    let dir = tempdir().unwrap();
    let state_dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, "hello").unwrap();

    fs::write(
        dir.path().join("main.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: {}\n    to: {}\n",
            src.display(),
            dst.display()
        ),
    )
    .unwrap();

    apply_with_state(dir.path(), state_dir.path()).success();

    let state_file = state_dir.path().join("state.yaml");
    assert!(state_file.exists(), "state.yaml should be created");
    let content = fs::read_to_string(&state_file).unwrap();
    assert!(
        content.contains(dst.to_str().unwrap()),
        "state should contain destination path as key"
    );
}

#[test]
fn apply_twice_merges_not_appends() {
    let dir = tempdir().unwrap();
    let state_dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, "hello").unwrap();

    let manifest = format!(
        "actions:\n  - action: file.copy\n    from: {}\n    to: {}\n",
        src.display(),
        dst.display()
    );
    fs::write(dir.path().join("main.yaml"), &manifest).unwrap();

    apply_with_state(dir.path(), state_dir.path()).success();
    apply_with_state(dir.path(), state_dir.path()).success();

    let content = fs::read_to_string(state_dir.path().join("state.yaml")).unwrap();
    let state: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).unwrap();
    let atoms = state["atoms"].as_sequence().unwrap();
    assert_eq!(atoms.len(), 1, "second apply should merge, not append");
}

#[test]
fn etch_history_stdout_contains_destination_path() {
    let dir = tempdir().unwrap();
    let state_dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, "hello").unwrap();

    fs::write(
        dir.path().join("main.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: {}\n    to: {}\n",
            src.display(),
            dst.display()
        ),
    )
    .unwrap();
    apply_with_state(dir.path(), state_dir.path()).success();

    // Use --json to get untruncated key (table output truncates path to 30 chars)
    let output = Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir.path())
        .env("ETCH_STATE_DIR", state_dir.path())
        .args(["--no-color", "history", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(dst.to_str().unwrap()),
        "history --json should contain full destination path"
    );
}

#[test]
fn etch_history_json_first_line_has_file_copy_action() {
    let dir = tempdir().unwrap();
    let state_dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, "hello").unwrap();

    fs::write(
        dir.path().join("main.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: {}\n    to: {}\n",
            src.display(),
            dst.display()
        ),
    )
    .unwrap();
    apply_with_state(dir.path(), state_dir.path()).success();

    let output = Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir.path())
        .env("ETCH_STATE_DIR", state_dir.path())
        .args(["--no-color", "history", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let first_line = stdout.lines().next().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(first_line).unwrap();
    assert_eq!(parsed["action"], "file.copy");
}

// ─── --label ──────────────────────────────────────────────────────────────────

#[test]
fn apply_label_runs_only_manifests_carrying_the_label() {
    let dir = tempdir().unwrap();
    let state = tempdir().unwrap();

    let kept = dir.path().join("kept_dir");
    let skipped = dir.path().join("skipped_dir");
    fs::write(
        dir.path().join("a_labelled.yaml"),
        format!(
            "labels:\n  - keep\nactions:\n  - action: directory.create\n    path: {}\n",
            kept.display()
        ),
    )
    .unwrap();
    fs::write(
        dir.path().join("b_unlabelled.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            skipped.display()
        ),
    )
    .unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir.path())
        .env("ETCH_STATE_DIR", state.path())
        .args(["--no-color", "-d", ".", "apply", "--label", "keep"])
        .assert()
        .success();

    assert!(kept.is_dir(), "labelled manifest should have run");
    assert!(
        !skipped.exists(),
        "manifest without the label should have been skipped"
    );
}
