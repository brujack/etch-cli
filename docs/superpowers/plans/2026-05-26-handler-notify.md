# Handler/Notify Pattern Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Ansible-style handlers to etch manifests — actions declare `notify: [name]`; named handlers in a `handlers:` section run once at end of manifest if any notifying action made a change.

**Architecture:** Add `notify: Vec<String>` to `ConditionalVariantAction<T>` (the existing per-action metadata wrapper), a `ManifestHandler` struct + `handlers:` field to `Manifest`, and notification tracking in the `apply.rs` action execution loop. No new action types; no new crates — `indexmap` is already in `lib/Cargo.toml`.

**Tech Stack:** Rust, serde/serde_yaml_ng, indexmap::IndexSet, assert_cmd (integration tests).

---

## File Map

| File                                   | Change                                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------------------------ |
| `lib/src/actions/mod.rs`               | Add `notify: Vec<String>` to `ConditionalVariantAction<T>`; add `Actions::notify()` method |
| `lib/src/manifests/mod.rs`             | Add `ManifestHandler` struct; add `handlers: Vec<ManifestHandler>` to `Manifest`           |
| `app/src/commands/apply.rs`            | Track notifications in action loop; run handlers after; import `IndexSet`                  |
| `app/tests/integration.rs`             | Three integration tests for handler behavior                                               |
| `examples/handler-notify/service.yaml` | Example manifest with handlers                                                             |
| `README.md`                            | Add `handlers:` + `notify:` to manifest format section                                     |

---

## Task 1: `notify` field on `ConditionalVariantAction<T>` and `Actions::notify()` method

**Files:**

- Modify: `lib/src/actions/mod.rs:46-56` (struct) and `lib/src/actions/mod.rs:219-253` (impl block)

### Background

`ConditionalVariantAction<T>` is the wrapper around every action type. It currently holds `action: T`, `condition: Option<String>`, and `variants: Vec<Variant<T>>`. Every `Actions` enum variant holds a `ConditionalVariantAction<T>`.

The `Actions` impl block starts at line 219 with `inner_ref()`. You're adding a new method `notify()` after it.

- [ ] **Step 1: Write two failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `lib/src/actions/mod.rs`:

```rust
#[test]
fn notify_deserializes_from_yaml() {
    let yaml = r#"
actions:
  - action: command.run
    command: echo
    notify: [restart-dock, reload-nginx]
"#;
    let m: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
    let action = match &m.actions[0] {
        Actions::CommandRun(a) => a,
        _ => panic!("expected CommandRun"),
    };
    assert_eq!(action.notify, vec!["restart-dock", "reload-nginx"]);
}

#[test]
fn notify_defaults_empty() {
    let yaml = r#"
actions:
  - action: command.run
    command: echo
"#;
    let m: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
    let action = match &m.actions[0] {
        Actions::CommandRun(a) => a,
        _ => panic!("expected CommandRun"),
    };
    assert!(action.notify.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p etch-lib notify_deserializes_from_yaml notify_defaults_empty -- --nocapture
```

Expected: compile error "no field `notify` on type `ConditionalVariantAction<RunCommand>`"

- [ ] **Step 3: Add `notify` field to `ConditionalVariantAction<T>`**

In `lib/src/actions/mod.rs`, replace the struct definition (lines 46–56):

```rust
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConditionalVariantAction<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "where")]
    pub condition: Option<String>,

    #[serde(default)]
    pub variants: Vec<Variant<T>>,

    #[serde(default)]
    pub notify: Vec<String>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p etch-lib notify_deserializes_from_yaml notify_defaults_empty
```

Expected: both PASS

- [ ] **Step 5: Add `Actions::notify()` method**

After the closing `}` of the `inner_ref()` impl block (after line 253, before the `impl Deref for Actions` at line 256), insert:

```rust
pub fn notify(&self) -> &[String] {
    match self {
        Actions::BinaryGitHub(a) => &a.notify,
        Actions::BinaryUrl(a) => &a.notify,
        Actions::BrewBundle(a) => &a.notify,
        Actions::BrewCleanup(a) => &a.notify,
        Actions::BrewUpgrade(a) => &a.notify,
        Actions::CommandRun(a) => &a.notify,
        Actions::DirectoryCopy(a) => &a.notify,
        Actions::DirectoryCreate(a) => &a.notify,
        Actions::DirectoryRemove(a) => &a.notify,
        Actions::FileChmod(a) => &a.notify,
        Actions::FileCopy(a) => &a.notify,
        Actions::FileChown(a) => &a.notify,
        Actions::FileDownload(a) => &a.notify,
        Actions::FileLink(a) => &a.notify,
        Actions::FileRemove(a) => &a.notify,
        Actions::FileUnarchive(a) => &a.notify,
        Actions::GitClone(a) => &a.notify,
        Actions::GitConfig(a) => &a.notify,
        Actions::GitPull(a) => &a.notify,
        Actions::GroupAdd(a) => &a.notify,
        Actions::MacOSDefault(a) => &a.notify,
        Actions::MacOSService(a) => &a.notify,
        Actions::SystemdService(a) => &a.notify,
        Actions::MasInstall(a) => &a.notify,
        Actions::MasUpgrade(a) => &a.notify,
        Actions::PackageInstall(a) => &a.notify,
        Actions::PackageRepository(a) => &a.notify,
        Actions::UserAdd(a) => &a.notify,
        Actions::UserAddGroup(a) => &a.notify,
        Actions::Plugin(a) => &a.notify,
    }
}
```

The `notify()` method lives inside the existing `impl Actions { ... }` block — add it after `inner_ref()`, still inside the same `impl Actions` block.

- [ ] **Step 6: Run full lib tests to confirm no regressions**

```bash
cargo test -p etch-lib
```

Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/mod.rs
git commit -m "feat(actions): add notify field to ConditionalVariantAction"
```

---

## Task 2: `ManifestHandler` struct and `handlers` field on `Manifest`

**Files:**

- Modify: `lib/src/manifests/mod.rs`

### Background

`Manifest` is defined in `lib/src/manifests/mod.rs`. It currently has fields: `where`, `name`, `labels`, `depends`, `actions`, `root_dir` (skip), `dag_index` (skip). You're adding `handlers: Vec<ManifestHandler>`.

`ManifestHandler` wraps an `Actions` value with a `name` field. The `#[serde(flatten)]` on `action: Actions` merges the `Actions` fields into the `ManifestHandler` level, so the YAML looks like a normal action entry plus a `name:` field.

- [ ] **Step 1: Write a failing test**

Add to the `#[cfg(test)] mod test` block at the bottom of `lib/src/manifests/mod.rs`:

```rust
#[test]
fn handlers_section_deserializes() {
    let yaml = r#"
handlers:
  - name: restart-dock
    action: command.run
    command: killall
    args: [Dock]
  - name: reload-nginx
    action: command.run
    command: systemctl
    args: [reload, nginx]
"#;
    let m: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.handlers.len(), 2);
    assert_eq!(m.handlers[0].name, "restart-dock");
    assert_eq!(m.handlers[1].name, "reload-nginx");
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p etch-lib handlers_section_deserializes -- --nocapture
```

Expected: compile error "no field `handlers` on type `Manifest`"

- [ ] **Step 3: Add `ManifestHandler` and update `Manifest`**

In `lib/src/manifests/mod.rs`, add the `ManifestHandler` struct and the `handlers` field. The file currently imports `Actions` from `crate::actions`. Add the struct before `Manifest`, and add the field to `Manifest`:

```rust
use crate::actions::Actions;
use petgraph::prelude::*;
pub use providers::register_providers;
pub use providers::ManifestProvider;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::error;

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub struct ManifestHandler {
    pub name: String,
    #[serde(flatten)]
    pub action: Actions,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub r#where: Option<String>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub labels: Vec<String>,

    #[serde(default)]
    pub depends: Vec<String>,

    #[serde(default)]
    pub actions: Vec<Actions>,

    #[serde(default)]
    pub handlers: Vec<ManifestHandler>,

    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p etch-lib handlers_section_deserializes
```

Expected: PASS

- [ ] **Step 5: Run full lib tests to confirm no regressions**

```bash
cargo test -p etch-lib
```

Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add lib/src/manifests/mod.rs
git commit -m "feat(manifests): add ManifestHandler and handlers field to Manifest"
```

---

## Task 3: Notification tracking and handler execution in `apply.rs`

**Files:**

- Modify: `app/src/commands/apply.rs`

### Background

The main action loop is in `apply.rs` inside `execute()`. Here is the relevant structure (starting around line 175):

```
let mut overall_successful = true;

for manifest in &run_manifests {
    while let Some(visited) = dfs.next(&dag) {
        let m1 = dag.node_weight(visited).unwrap();
        let mut successful = true;
        // ... where condition check ...

        for action in m1.actions.iter() {           // <-- action loop to modify
            let span_action = span!(..., %action).entered();
            let action = action.inner_ref();          // shadows outer `action`
            let plan = action.plan(m1, contexts)?;
            let mut steps = plan.into_iter()
                .filter(initializers)
                .filter(should_run)
                .peekable();
            if steps.peek().is_none() { continue; }

            let mut dry_run_count = 0usize;
            for mut step in steps {
                if dry_run { dry_run_count += 1; ... continue; }
                match step.atom.execute() {
                    Ok(_) => (),
                    Err(_) => { successful = false; break; }
                }
                if !step.do_finalizers_allow_us_to_continue() {
                    successful = false; break;
                }
            }
            if dry_run { println!(...) } else { info!(...) }
            span_action.exit();
        }
        // <-- handler loop goes here

        if dry_run { span_manifest.exit(); continue; }
        if !successful { ... break; }
        info!("Completed");
        span_manifest.exit();
    }
}
```

You need to:

1. Add `use indexmap::IndexSet;` to imports
2. Rename outer `action` to `raw_action` in the loop to avoid shadowing before extracting `.notify()`
3. Track `all_succeeded` + `steps_ran` per action
4. Accumulate `notified: IndexSet<String>` per manifest
5. Add handler loop after the action loop

- [ ] **Step 1: Add `IndexSet` import**

At the top of `app/src/commands/apply.rs`, change:

```rust
use std::{collections::HashMap, ops::Deref};
```

to:

```rust
use indexmap::IndexSet;
use std::{collections::HashMap, ops::Deref};
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p etch-cli 2>&1 | head -20
```

Expected: compiles (IndexSet unused warning is OK at this point)

- [ ] **Step 3: Add `notified` initialization per manifest**

Locate `let mut successful = true;` (line ~216, inside the `while let Some(visited)` loop). Add `notified` immediately after:

```rust
let mut successful = true;
let mut notified: IndexSet<String> = IndexSet::new();
```

- [ ] **Step 4: Rewrite the action loop with notification tracking**

Replace the entire `for action in m1.actions.iter() { ... }` block (lines ~252–316) with:

```rust
for raw_action in m1.actions.iter() {
    let span_action = span!(tracing::Level::INFO, "", %raw_action).entered();

    let action = raw_action.inner_ref();

    let plan = match action.plan(m1, contexts) {
        Ok(steps) => steps,
        Err(err) => {
            info!("Action failed to get plan: {:?}", err);
            successful = false;
            span_action.exit();
            continue;
        }
    };

    let mut steps = plan
        .into_iter()
        .filter(|step| step.do_initializers_allow_us_to_run())
        .filter(|step| match step.atom.plan() {
            Ok(outcome) => outcome.should_run,
            Err(_) => false,
        })
        .peekable();

    if steps.peek().is_none() {
        info!("nothing to be done to reconcile action");
        span_action.exit();
        continue;
    }

    let mut dry_run_count = 0usize;
    let mut steps_ran = 0usize;
    let mut all_succeeded = true;
    for mut step in steps {
        if dry_run {
            dry_run_count += 1;
            if runtime.args.verbose > 0 {
                println!("  would: {}", step.atom);
            }
            continue;
        }

        match step.atom.execute() {
            Ok(_) => {
                steps_ran += 1;
            }
            Err(err) => {
                debug!("Atom failed to execute: {:?}", err);
                successful = false;
                all_succeeded = false;
                break;
            }
        }

        if !step.do_finalizers_allow_us_to_continue() {
            debug!("Finalizers won't allow us to continue with this action");
            successful = false;
            all_succeeded = false;
            break;
        }
    }

    if dry_run {
        println!(
            "[dry run] {}: {} step(s) would run",
            action.summarize(),
            dry_run_count
        );
        if dry_run_count > 0 {
            for name in raw_action.notify() {
                println!("[dry run] handler '{}' would run", name);
            }
        }
    } else {
        info!("{}", action.summarize());
        if all_succeeded && steps_ran > 0 {
            for name in raw_action.notify() {
                notified.insert(name.clone());
            }
        }
    }
    span_action.exit();
}
```

- [ ] **Step 5: Add handler execution loop after the action loop**

Insert the handler loop immediately after the closing `}` of the `for raw_action in m1.actions.iter()` loop, and before the `if dry_run { span_manifest.exit(); continue; }` block:

```rust
// Run notified handlers in declaration order
if !dry_run {
    for handler in m1.handlers.iter() {
        if !notified.contains(&handler.name) {
            continue;
        }

        let handler_action = handler.action.inner_ref();

        let plan = match handler_action.plan(m1, contexts) {
            Ok(steps) => steps,
            Err(err) => {
                info!("Handler '{}' failed to plan: {:?}", handler.name, err);
                successful = false;
                continue;
            }
        };

        let mut steps = plan
            .into_iter()
            .filter(|step| step.do_initializers_allow_us_to_run())
            .filter(|step| match step.atom.plan() {
                Ok(outcome) => outcome.should_run,
                Err(_) => false,
            })
            .peekable();

        if steps.peek().is_none() {
            info!("Handler '{}': nothing to do", handler.name);
            continue;
        }

        for mut step in steps {
            match step.atom.execute() {
                Ok(_) => {}
                Err(err) => {
                    debug!("Handler '{}' failed: {:?}", handler.name, err);
                    successful = false;
                    break;
                }
            }
            if !step.do_finalizers_allow_us_to_continue() {
                debug!(
                    "Handler '{}': finalizers won't allow us to continue",
                    handler.name
                );
                successful = false;
                break;
            }
        }
        info!("Handler '{}' complete", handler.name);
    }
}
```

- [ ] **Step 6: Build to verify it compiles**

```bash
cargo build -p etch-cli
```

Expected: compiles with no errors

- [ ] **Step 7: Run all tests**

```bash
cargo test
```

Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add app/src/commands/apply.rs
git commit -m "feat(apply): collect handler notifications and run handlers post-actions"
```

---

## Task 4: Integration tests

**Files:**

- Modify: `app/tests/integration.rs`

### Background

The integration tests spawn the real `etch` binary via `assert_cmd`. The helper `apply(dir)` runs `etch --no-color -d . apply` from `dir`. Tests create a temp directory, write a YAML manifest file into it, call `apply()`, then assert on side effects.

`command.run` always has `should_run = true` — it always executes. To test "handler not triggered when action is skipped", use `skip_if_exists` on `command.run`: the action skips if a given path exists.

- [ ] **Step 1: Write three failing integration tests**

Add to `app/tests/integration.rs`:

```rust
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

    assert!(sentinel.exists(), "handler sentinel should exist after apply");
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
    assert_eq!(count, 1, "handler should run exactly once, ran {count} times");
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
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test -p etch-cli --test integration handler_runs_when_action_executes handler_runs_once_when_notified_by_multiple_actions handler_not_triggered_when_action_skipped handler_not_triggered_when_action_fails failed_handler_does_not_stop_subsequent_handlers -- --nocapture
```

Expected: all three FAIL (feature not yet implemented — handler block doesn't exist yet, or handlers field missing, depending on what order tasks were done)

If running this task after Tasks 1–3 are complete, the tests should PASS here. That's also fine — TDD across tasks.

- [ ] **Step 3: Run all tests to confirm**

```bash
cargo test
```

Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add handler/notify integration tests"
```

---

## Task 5: Example manifest and docs

**Files:**

- Create: `examples/handler-notify/service.yaml`
- Modify: `README.md`

- [ ] **Step 1: Create example manifest**

Create `examples/handler-notify/service.yaml`:

```yaml
# Handler/notify pattern examples
#
# Handlers run once at the end of the manifest if any notifying action
# made a change. Multiple actions can notify the same handler — it runs once.

actions:
    # macOS: restart Dock after any dock pref change
    - action: macos.default
      domain: com.apple.dock
      key: autohide
      kind: bool
      value: "true"
      notify: [restart-dock]
      where: 'os.name == "macos"'

    - action: macos.default
      domain: com.apple.dock
      key: tilesize
      kind: integer
      value: "48"
      notify: [restart-dock]
      where: 'os.name == "macos"'

    # Linux: reload nginx after enabling the service
    - action: systemd.service
      unit: nginx.service
      enabled: true
      started: true
      notify: [reload-nginx]
      where: 'os.family == "linux"'

    # Handler notified but skipped — skip_if_exists makes this a no-op
    # when the file already exists, so the handler does not run.
    - action: command.run
      command: touch
      args: [/tmp/etch-example-marker]
      skip_if_exists: /tmp/etch-example-marker
      notify: [on-first-run]

handlers:
    # Runs once after all dock pref actions, only if any changed
    - name: restart-dock
      action: command.run
      command: killall
      args: [Dock]
      where: 'os.name == "macos"'

    # Runs once after nginx is enabled, only if the service state changed
    - name: reload-nginx
      action: command.run
      command: systemctl
      args: [reload, nginx]
      privileged: true
      where: 'os.family == "linux"'

    # Runs only on first apply (when marker file doesn't exist yet)
    - name: on-first-run
      action: command.run
      command: echo
      args: ["First run complete"]
```

- [ ] **Step 2: Update `README.md` manifest format section**

Locate the `## Manifest format` section in `README.md`. After the closing code fence of the YAML example, add:

````markdown
Handlers run once at end of the manifest when a notifying action made a change:

```yaml
actions:
    - action: macos.defaults
      domain: com.apple.dock
      key: autohide
      kind: bool
      value: "true"
      notify: [restart-dock]

handlers:
    - name: restart-dock
      action: command.run
      command: killall
      args: [Dock]
```
````

````

- [ ] **Step 3: Run all tests one final time**

```bash
cargo test
````

Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add examples/handler-notify/service.yaml README.md
git commit -m "docs(handler-notify): add example manifest and README update"
```

---

## Post-merge (do on main after PR merges — NOT inside the worktree)

- [ ] Update `docs/superpowers/README.md`: change handler-notify row status from In Progress → Done
- [ ] Add `> **Status: DONE**` banner to this plan file
- [ ] Commit directly to main: `git commit -m "docs(handler-notify): mark Done"`
