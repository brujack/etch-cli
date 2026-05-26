# macos.defaults: array-add and delete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `macos.default` with `operation: write | array-add | delete`, making all three idempotent.

**Architecture:** Single struct `MacOSDefault` gains an `operation` enum field (default `Write`). `kind`/`value` become `Option<String>`. Command strings for `delete` and `array-add` are built by pure helper functions (`delete_shell_cmd`, `array_add_shell_cmd`) that are unit-tested directly, then wrapped in `Exec` atoms in `plan()`. All operations emit exactly one `Step`.

**Tech Stack:** Rust, serde/serde_yaml_ng, schemars, anyhow, existing `Exec` atom

---

## Files

- Modify: `lib/src/actions/macos/default.rs` — all implementation and tests

---

### Task 1: Add `MacOSDefaultOperation` enum, update struct, fix Write for optional fields

**Files:**

- Modify: `lib/src/actions/macos/default.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests to the `#[cfg(test)]` block in `lib/src/actions/macos/default.rs`:

```rust
#[test]
fn operation_defaults_to_write() {
    let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::MacOSDefault(action)) => {
            assert_eq!(MacOSDefaultOperation::Write, action.action.operation);
        }
        _ => panic!("MacOSDefault didn't deserialize"),
    }
}

#[test]
fn write_missing_kind_returns_error() {
    let action = MacOSDefault {
        domain: String::from("com.apple.dock"),
        key: String::from("autohide"),
        operation: MacOSDefaultOperation::Write,
        kind: None,
        value: Some(String::from("true")),
    };
    assert!(action
        .plan(&Manifest::default(), &Contexts::default())
        .is_err());
}

#[test]
fn write_missing_value_returns_error() {
    let action = MacOSDefault {
        domain: String::from("com.apple.dock"),
        key: String::from("autohide"),
        operation: MacOSDefaultOperation::Write,
        kind: Some(String::from("bool")),
        value: None,
    };
    assert!(action
        .plan(&Manifest::default(), &Contexts::default())
        .is_err());
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /path/to/etch-cli
cargo test -p etch-lib macos::default 2>&1 | tail -20
```

Expected: compilation errors — `MacOSDefaultOperation` not defined, struct fields wrong types.

- [ ] **Step 3: Replace the entire `lib/src/actions/macos/default.rs` with the updated implementation**

```rust
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacOSDefaultOperation {
    #[default]
    Write,
    ArrayAdd,
    Delete,
}

// I went through all the examples here: https://macos-defaults.com/
// and while arrays and dictionaries are valid values, I couldn't
// find any usable examples. So omitting for now
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    pub domain: String,
    pub key: String,
    #[serde(default)]
    pub operation: MacOSDefaultOperation,
    pub kind: Option<String>,
    pub value: Option<String>,
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn delete_shell_cmd(domain: &str, key: &str) -> String {
    format!(
        "defaults delete {} {} 2>/dev/null || true",
        sh_quote(domain),
        sh_quote(key),
    )
}

fn array_add_shell_cmd(domain: &str, key: &str, kind: &str, value: &str) -> String {
    let domain_q = sh_quote(domain);
    let key_q = sh_quote(key);
    let value_q = sh_quote(value);
    format!(
        "defaults read {domain} {key} 2>/dev/null | grep -qF {value} || defaults write {domain} {key} -array-add -{kind} {value}",
        domain = domain_q,
        key = key_q,
        kind = kind,
        value = value_q,
    )
}

impl Action for MacOSDefault {
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        match self.operation {
            MacOSDefaultOperation::Write => {
                let kind = self
                    .kind
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`kind` is required for operation `write`"))?;
                let value = self
                    .value
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`value` is required for operation `write`"))?;
                Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("defaults"),
                        arguments: vec![
                            String::from("write"),
                            self.domain.clone(),
                            self.key.clone(),
                            format!("-{}", kind),
                            value.to_string(),
                        ],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }])
            }
            MacOSDefaultOperation::Delete => Ok(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("sh"),
                    arguments: vec![
                        String::from("-c"),
                        delete_shell_cmd(&self.domain, &self.key),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]),
            MacOSDefaultOperation::ArrayAdd => {
                let kind = self
                    .kind
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`kind` is required for operation `array-add`"))?;
                let value = self
                    .value
                    .as_deref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("`value` is required for operation `array-add`")
                    })?;
                Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("sh"),
                        arguments: vec![
                            String::from("-c"),
                            array_add_shell_cmd(&self.domain, &self.key, kind, value),
                        ],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!("com.apple.dock", action.action.domain);
                assert_eq!("autohide", action.action.key);
                assert_eq!(Some(String::from("bool")), action.action.kind);
                assert_eq!(Some(String::from("true")), action.action.value);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("bool")),
            value: Some(String::from("true")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_integer_kind() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("tilesize"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("integer")),
            value: Some(String::from("48")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_string_kind() {
        let action = MacOSDefault {
            domain: String::from("com.example.app"),
            key: String::from("mykey"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("string")),
            value: Some(String::from("myvalue")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn operation_defaults_to_write() {
        let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::Write, action.action.operation);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn write_missing_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: None,
            value: Some(String::from("true")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn write_missing_value_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            operation: MacOSDefaultOperation::Write,
            kind: Some(String::from("bool")),
            value: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn it_can_deserialize_delete() {
        let yaml = r#"
- action: macos.default
  operation: delete
  domain: com.apple.dock
  key: stale-key
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::Delete, action.action.operation);
                assert_eq!("com.apple.dock", action.action.domain);
                assert_eq!("stale-key", action.action.key);
            }
            _ => panic!("MacOSDefault delete didn't deserialize"),
        }
    }

    #[test]
    fn delete_emits_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("stale-key"),
            operation: MacOSDefaultOperation::Delete,
            kind: None,
            value: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn delete_ignores_kind_and_value() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("stale-key"),
            operation: MacOSDefaultOperation::Delete,
            kind: Some(String::from("bool")),
            value: Some(String::from("true")),
        };
        // Should succeed despite kind/value being present (they are ignored)
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_ok());
    }

    #[test]
    fn delete_shell_cmd_produces_correct_command() {
        let cmd = delete_shell_cmd("com.apple.dock", "tilesize");
        assert_eq!(
            "defaults delete 'com.apple.dock' 'tilesize' 2>/dev/null || true",
            cmd
        );
    }

    #[test]
    fn delete_shell_cmd_escapes_single_quotes() {
        let cmd = delete_shell_cmd("com.apple.it's", "key");
        assert!(cmd.contains(r"'\''"), "single quote not escaped: {cmd}");
    }

    #[test]
    fn it_can_deserialize_array_add() {
        let yaml = r#"
- action: macos.default
  operation: array-add
  domain: com.apple.systemuiserver
  key: menuExtras
  kind: string
  value: "/System/Library/CoreServices/Menu Extras/Volume.menu"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!(MacOSDefaultOperation::ArrayAdd, action.action.operation);
                assert_eq!("com.apple.systemuiserver", action.action.domain);
                assert_eq!("menuExtras", action.action.key);
                assert_eq!(Some(String::from("string")), action.action.kind);
                assert_eq!(
                    Some(String::from(
                        "/System/Library/CoreServices/Menu Extras/Volume.menu"
                    )),
                    action.action.value
                );
            }
            _ => panic!("MacOSDefault array-add didn't deserialize"),
        }
    }

    #[test]
    fn array_add_emits_one_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: Some(String::from("string")),
            value: Some(String::from(
                "/System/Library/CoreServices/Menu Extras/Volume.menu",
            )),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn array_add_missing_kind_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: None,
            value: Some(String::from("/System/Library/CoreServices/Menu Extras/Volume.menu")),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_missing_value_returns_error() {
        let action = MacOSDefault {
            domain: String::from("com.apple.systemuiserver"),
            key: String::from("menuExtras"),
            operation: MacOSDefaultOperation::ArrayAdd,
            kind: Some(String::from("string")),
            value: None,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn array_add_shell_cmd_produces_correct_command() {
        let cmd = array_add_shell_cmd(
            "com.apple.systemuiserver",
            "menuExtras",
            "string",
            "/System/Library/CoreServices/Menu Extras/Volume.menu",
        );
        assert_eq!(
            "defaults read 'com.apple.systemuiserver' 'menuExtras' 2>/dev/null \
             | grep -qF '/System/Library/CoreServices/Menu Extras/Volume.menu' \
             || defaults write 'com.apple.systemuiserver' 'menuExtras' -array-add \
             -string '/System/Library/CoreServices/Menu Extras/Volume.menu'",
            cmd
        );
    }

    #[test]
    fn array_add_shell_cmd_escapes_single_quotes_in_value() {
        let cmd = array_add_shell_cmd("com.example.app", "key", "string", "it's a value");
        assert!(
            cmd.contains(r"'\''"),
            "single quote not escaped in value: {cmd}"
        );
    }
}
```

- [ ] **Step 4: Run all tests in the module**

```bash
cargo test -p etch-lib macos::default 2>&1 | tail -30
```

Expected: all tests pass, including existing `it_can_be_deserialized`, `plan_returns_one_step`, `plan_with_integer_kind`, `plan_with_string_kind`.

- [ ] **Step 5: Run the full test suite to verify no regressions**

```bash
make test 2>&1 | tail -20
```

Expected: `test result: ok` for all crates.

- [ ] **Step 6: Commit**

Run the `caveman:caveman-commit` skill to generate the message, then:

```bash
git add lib/src/actions/macos/default.rs
git commit -m "<generated message>"
```

---

### Task 2: Update examples and CLAUDE.md action catalog

**Files:**

- Modify: `examples/macos/defaults.yaml`
- Modify: `CLAUDE.md` (action catalog table row for `macos.defaults`)

- [ ] **Step 1: Add array-add and delete examples to `examples/macos/defaults.yaml`**

Append to the end of `examples/macos/defaults.yaml`:

```yaml
# array-add: idempotent append to an array key
- action: macos.default
  operation: array-add
  domain: com.apple.systemuiserver
  key: menuExtras
  kind: string
  value: "/System/Library/CoreServices/Menu Extras/Volume.menu"

# delete: remove a key (no-op if absent)
- action: macos.default
  operation: delete
  domain: com.apple.dock
  key: stale-key
```

- [ ] **Step 2: Update the `macos.defaults` row in the CLAUDE.md action catalog**

Find this row in `CLAUDE.md`:

```markdown
| `macos.defaults` | Write macOS defaults | domain, key, type, value fields
```

Replace with:

```markdown
| `macos.defaults` | Write macOS defaults (write/array-add/delete) | `domain`, `key`, `operation` (`write` default \| `array-add` \| `delete`), `kind` (required for write/array-add), `value` (required for write/array-add)
```

- [ ] **Step 3: Verify lint passes**

```bash
make lint 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

Run the `caveman:caveman-commit` skill to generate the message, then:

```bash
git add examples/macos/defaults.yaml CLAUDE.md
git commit -m "<generated message>"
```

---

### Task 3: Open PR and monitor CI

- [ ] **Step 1: Push the feature branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli --title "feat(macos.defaults): add array-add and delete operations" --body "$(cat <<'EOF'
## Summary
- Adds `operation: array-add | delete` to `macos.default` action
- `array-add` is idempotent (reads current array, skips if value present)
- `delete` is idempotent (absorbs exit 1 when key absent via `|| true`)
- `kind`/`value` become optional; missing them on `write`/`array-add` is a runtime error
- Backward-compatible: existing manifests without `operation` field default to `write`

## Test plan
- [ ] `cargo test -p etch-lib macos::default` — all 17 tests pass
- [ ] `make test` — no regressions
- [ ] CI green

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Run code review**

Run the `code-review:code-review` skill on the new PR number.

- [ ] **Step 4: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli <pr-number> --watch
```

Wait for all checks to pass. If any fail, read the logs:

```bash
gh run view <run-id> --log-failed
```

Fix, commit, push. CI re-runs automatically.

- [ ] **Step 5: Post-merge cleanup** (do this on main, not in the worktree)

After auto-merge:

```bash
git worktree remove /path/to/worktree   # if using worktree
git branch -d <branch-name>
git push origin --delete <branch-name>
git fetch --prune
git reset --hard origin/main
```

- [ ] **Step 6: Update plan index** (on main, after merge)

In `docs/superpowers/README.md`, update the macos-defaults row:

```markdown
| 2026-05-26 | [macos-defaults](plans/2026-05-26-macos-defaults.md) | [macos-defaults](specs/2026-05-26-macos-defaults-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

Commit directly to main:

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-26-macos-defaults.md
git commit -m "docs(macos-defaults): mark Done in plan index"
```
