# ruby.install version_manager Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `version_manager` field to `ruby.install` that emits `rbenv global <version>` + `rbenv rehash` steps after ruby-install when set to `"rbenv"`.

**Architecture:** Add a `VersionManager` enum (`Rbenv` | `Chruby`) and an optional `version_manager` field to `RubyInstall`. The `plan()` method appends two extra `Exec` steps after the ruby-install step when `version_manager == Rbenv`. All extra steps are conditional on ruby not already being installed — idempotency is preserved.

**Tech Stack:** Rust, serde, schemars, `lib/src/atoms/command::Exec`

---

## Files

- Modify: `lib/src/actions/ruby/install.rs` — add enum, field, plan logic, tests

---

### Task 1: Add `VersionManager` enum, field, and deserialization test

**Files:**

- Modify: `lib/src/actions/ruby/install.rs`

This task adds the new type and field to the struct. No plan behaviour changes yet.

- [ ] **Step 1: Write the failing test**

Add this test inside the `#[cfg(test)]` `mod tests` block in `lib/src/actions/ruby/install.rs`, alongside the existing deserialization tests:

```rust
#[test]
fn it_can_be_deserialized_with_version_manager() {
    let yaml = r#"
- action: ruby.install
  version: "3.3.0"
  version_manager: rbenv
"#;
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::RubyInstall(action)) => {
            assert_eq!("3.3.0", action.action.version);
            assert_eq!(Some(VersionManager::Rbenv), action.action.version_manager);
        }
        _ => panic!("RubyInstall didn't deserialize to the correct type"),
    }
}
```

- [ ] **Step 2: Run to confirm it fails**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib -E 'test(it_can_be_deserialized_with_version_manager)'
```

Expected: compile error — `VersionManager` not defined.

- [ ] **Step 3: Add the enum and field**

In `lib/src/actions/ruby/install.rs`, add the enum before `RubyInstall` and the field to the struct:

```rust
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionManager {
    Rbenv,
    Chruby,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyInstall {
    pub version: String,
    pub implementation: Option<String>,
    pub rubies_dir: Option<String>,
    pub version_manager: Option<VersionManager>,
}
```

Note: `Default` cannot be derived for `VersionManager` (no obvious default), so keep it on `RubyInstall` via `Option<VersionManager>` which defaults to `None`. The `#[derive(Default)]` on `RubyInstall` remains valid because `Option<T>` defaults to `None`.

- [ ] **Step 4: Run to confirm it passes**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib -E 'test(it_can_be_deserialized_with_version_manager)'
```

Expected: PASS

- [ ] **Step 5: Run full test suite**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib
```

Expected: all existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/ruby/install.rs
git commit -m "feat(ruby.install): add VersionManager enum and version_manager field"
```

---

### Task 2: Emit rbenv post-install steps in `plan()`

**Files:**

- Modify: `lib/src/actions/ruby/install.rs`

- [ ] **Step 1: Write the failing test**

Add these three tests to `mod tests` in `lib/src/actions/ruby/install.rs`:

```rust
#[test]
fn plan_with_rbenv_emits_three_steps() {
    let tmp = tempfile::tempdir().unwrap();
    let action = RubyInstall {
        version: String::from("3.3.0"),
        implementation: None,
        rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
        version_manager: Some(VersionManager::Rbenv),
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert_eq!(3, steps.len(), "expected 3 steps for rbenv install");
    let step2 = steps[1].atom.to_string();
    assert!(step2.contains("rbenv"), "step 2 should invoke rbenv: {step2}");
    assert!(step2.contains("global"), "step 2 should set global: {step2}");
    assert!(step2.contains("3.3.0"), "step 2 should include version: {step2}");
    let step3 = steps[2].atom.to_string();
    assert!(step3.contains("rbenv"), "step 3 should invoke rbenv: {step3}");
    assert!(step3.contains("rehash"), "step 3 should rehash: {step3}");
}

#[test]
fn plan_with_chruby_emits_one_step() {
    let tmp = tempfile::tempdir().unwrap();
    let action = RubyInstall {
        version: String::from("3.3.0"),
        implementation: None,
        rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
        version_manager: Some(VersionManager::Chruby),
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert_eq!(1, steps.len(), "chruby should emit only the ruby-install step");
}

#[test]
fn plan_skips_all_if_installed_with_rbenv() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("ruby-3.3.0")).unwrap();
    let action = RubyInstall {
        version: String::from("3.3.0"),
        implementation: None,
        rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
        version_manager: Some(VersionManager::Rbenv),
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert!(
        steps.is_empty(),
        "expected no steps when ruby already installed, even with rbenv set"
    );
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib -E 'test(plan_with_rbenv_emits_three_steps) | test(plan_with_chruby_emits_one_step) | test(plan_skips_all_if_installed_with_rbenv)'
```

Expected: `plan_with_rbenv_emits_three_steps` FAIL (1 step, not 3). Others PASS (they test existing behaviour).

- [ ] **Step 3: Add rbenv steps to `plan()`**

In `lib/src/actions/ruby/install.rs`, update the `plan()` method. Replace the current return statement with:

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    use crate::atoms::command::Exec;

    let ruby_dir =
        self.resolved_rubies_dir()
            .join(format!("{}-{}", self.impl_name(), self.version));

    if ruby_dir.exists() {
        return Ok(vec![]);
    }

    let mut arguments = vec![self.impl_name().to_string(), self.version.clone()];
    if let Some(dir) = &self.rubies_dir {
        let expanded = shellexpand::tilde(dir).into_owned();
        arguments.push(String::from("--rubies-dir"));
        arguments.push(expanded);
    }

    let mut steps = vec![Step {
        atom: Box::new(Exec {
            command: String::from("ruby-install"),
            arguments,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }];

    if let Some(VersionManager::Rbenv) = &self.version_manager {
        steps.push(Step {
            atom: Box::new(Exec {
                command: String::from("rbenv"),
                arguments: vec![String::from("global"), self.version.clone()],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        });
        steps.push(Step {
            atom: Box::new(Exec {
                command: String::from("rbenv"),
                arguments: vec![String::from("rehash")],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        });
    }

    Ok(steps)
}
```

- [ ] **Step 4: Run the new tests**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib -E 'test(plan_with_rbenv_emits_three_steps) | test(plan_with_chruby_emits_one_step) | test(plan_skips_all_if_installed_with_rbenv)'
```

Expected: all three PASS.

- [ ] **Step 5: Run full test suite**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo nextest run -p etch-lib
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/ruby/install.rs
git commit -m "feat(ruby.install): emit rbenv global + rehash steps when version_manager: rbenv"
```

---

### Task 3: Update examples and Action Catalog

**Files:**

- Modify: `examples/ruby/ruby-install.yaml`
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Update the example file**

Replace the contents of `examples/ruby/ruby-install.yaml` with:

```yaml
actions:
    # Install default Ruby implementation. Idempotent: skips if ~/.rubies/ruby-3.3.0 exists.
    # Requires ruby-install to be on PATH (e.g. installed via brew).
    - action: ruby.install
      version: "3.3.0"

    # Install a specific implementation (jruby, truffleruby, etc.)
    - action: ruby.install
      version: "9.4.0.0"
      implementation: jruby

    # Install to a custom rubies directory (passes --rubies-dir to ruby-install)
    - action: ruby.install
      version: "3.3.0"
      rubies_dir: /opt/rubies

    # Install with rbenv: after ruby-install, runs `rbenv global <version>` and `rbenv rehash`
    - action: ruby.install
      version: "3.3.0"
      version_manager: rbenv

    # chruby: no post-install steps needed (auto-discovers from ~/.rubies)
    - action: ruby.install
      version: "3.3.0"
      version_manager: chruby
```

- [ ] **Step 2: Update `CLAUDE.md` Action Catalog**

Find the `ruby.install` row in the Action Catalog table in `CLAUDE.md`. The current `Key fields` cell ends with `Requires ruby-install on PATH.`

Append to that cell (before the closing `|`):

```
, `version_manager` (Option `"rbenv"` | `"chruby"` — when `"rbenv"`, appends `rbenv global <version>` and `rbenv rehash` steps after installation; `"chruby"` accepted but no extra steps emitted)
```

- [ ] **Step 3: Update `README.md` Action Catalog**

Apply the identical change to the `ruby.install` row in `README.md` (same table, same cell format).

- [ ] **Step 4: Run make lint**

```bash
export PATH="/Users/bruce/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
make lint
```

Expected: clean (no Rust changes in this task — just YAML and Markdown).

- [ ] **Step 5: Commit**

```bash
git add examples/ruby/ruby-install.yaml CLAUDE.md README.md
git commit -m "docs(ruby.install): document version_manager field and update examples"
```

---

### Task 4: Post-merge docs status update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index**

In `docs/superpowers/README.md`, find the row:

```
| 2026-06-01 | —                                                                                        | [ruby-install-version-manager](specs/2026-06-01-ruby-install-version-manager-design.md)                                   | Pending |
```

Change it to:

```
| 2026-06-01 | [ruby-install-version-manager](plans/2026-06-01-ruby-install-version-manager.md)          | [ruby-install-version-manager](specs/2026-06-01-ruby-install-version-manager-design.md)                                   | Done    |
```

Also remove the entry from the `## Bugs` table (the `ruby.install + rbenv` row).

- [ ] **Step 2: Add Done banner to plan file**

At the top of `docs/superpowers/plans/2026-06-01-ruby-install-version-manager.md`, add:

```markdown
> **Status: DONE**
```

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-06-01-ruby-install-version-manager.md
git commit -m "docs(superpowers): mark ruby.install version_manager Done"
```
