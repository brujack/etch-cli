# user.default_shell Action — Design Spec

## Context

dotfiles `setup_zsh_as_default_shell()` calls `chsh -s /bin/zsh`. No native etch-cli action exists — etch-config works around this with `command.run`. A `user.default_shell` action eliminates the workaround and provides idempotency: it reads the user's current shell from `/etc/passwd` (via `uzers`) at plan time and emits a `chsh` step only when a change is needed.

---

## Scope

One new action file. No changes to existing `User` struct or `user/mod.rs`. Registered in `actions/mod.rs` as `"user.default_shell"`.

---

## Fields

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDefaultShell {
    pub shell: String,
    pub username: Option<String>,
}
```

| Field      | Type             | Required | Description                                        |
| ---------- | ---------------- | -------- | -------------------------------------------------- |
| `shell`    | `String`         | Yes      | Absolute path to the login shell (e.g. `/bin/zsh`) |
| `username` | `Option<String>` | No       | Target user; absent = current user                 |

---

## Behavior

| Scenario                         | Result                                                 |
| -------------------------------- | ------------------------------------------------------ |
| `shell:` empty                   | `plan()` returns `Err` — fail early                    |
| Shell matches current value      | `plan()` returns empty steps — skip                    |
| Shell differs from current value | `chsh -s <shell>` step (current user, not privileged)  |
| `username:` set, shell differs   | `chsh -s <shell> <username>` step (`privileged: true`) |
| User not found                   | `chsh` step emitted; `chsh` fails with its own error   |
| Shell not in `/etc/shells`       | `chsh` step emitted; `chsh` fails with its own error   |

No `/etc/shells` validation at plan time — `chsh` is the authoritative validator.

---

## Implementation

### `lib/src/actions/user/default_shell.rs`

```rust
use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDefaultShell {
    pub shell: String,
    pub username: Option<String>,
}

impl Action for UserDefaultShell {
    fn summarize(&self) -> String {
        match &self.username {
            Some(u) => format!("Setting default shell for {} to {}", u, self.shell),
            None => format!("Setting default shell to {}", self.shell),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.shell.is_empty() {
            anyhow::bail!("user.default_shell requires 'shell' to be specified");
        }

        let current_shell = match &self.username {
            Some(name) => uzers::get_user_by_name(name.as_str())
                .map(|u| u.shell().to_string_lossy().into_owned()),
            None => uzers::get_current_user()
                .map(|u| u.shell().to_string_lossy().into_owned()),
        };

        if current_shell.as_deref() == Some(self.shell.as_str()) {
            return Ok(vec![]);
        }

        let mut args = vec![String::from("-s"), self.shell.clone()];
        let privileged = self.username.is_some();
        if let Some(name) = &self.username {
            args.push(name.clone());
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("chsh"),
                arguments: args,
                privileged,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
```

### `lib/src/actions/user/mod.rs`

Add: `pub mod default_shell;`

### `lib/src/actions/mod.rs` (6 edits, same pattern as all new actions)

- `use user::default_shell::UserDefaultShell;`
- Enum variant: `#[serde(rename = "user.default_shell")] UserDefaultShell(ConditionalVariantAction<UserDefaultShell>)`
- Match arm in `inner_ref()`
- Match arm in `notify`
- Match arm in `Deref`
- Match arm in `Display` → `"user.default_shell"`
- Update the 3 dispatch test YAML lists + counts

---

## Testing

All in `lib/src/actions/user/default_shell.rs`:

| Test                                              | What it checks                                                                                                                       |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `plan_errors_when_shell_empty`                    | empty `shell:` → `Err`                                                                                                               |
| `plan_skips_when_shell_matches_current_user`      | reads real current shell via `uzers::get_current_user()`, uses it as `self.shell` → empty steps                                      |
| `plan_emits_chsh_when_shell_differs_current_user` | `shell: "/bin/definitely-not-a-shell"` → 1 step, command `"chsh"`, args `["-s", "/bin/definitely-not-a-shell"]`, `privileged: false` |
| `plan_emits_privileged_chsh_with_username`        | `username: Some("bob")`, `shell: "/bin/zsh"` → args `["-s", "/bin/zsh", "bob"]`, `privileged: true`                                  |
| `deserialization_with_shell_only`                 | YAML `shell: /bin/zsh` → `username: None`                                                                                            |
| `deserialization_with_username`                   | YAML with both fields → correct struct                                                                                               |

No mocking of `uzers` — tests check step structure (command, args, privileged flag). The "shell differs" tests use a path guaranteed not to match any real user's shell.

---

## Documentation

### `examples/user/user.yaml`

```yaml
# Change the current user's default login shell. Idempotent: no-op if already correct.
# Shell must be listed in /etc/shells — chsh errors otherwise.
- action: user.default_shell
  shell: /bin/zsh

# Change another user's default shell (requires privilege escalation).
- action: user.default_shell
  shell: /bin/zsh
  username: bruce
  where: 'os.name == "linux"'
```

### `README.md`

Add `user.default_shell` to the action catalog table.

### `docs/knowledge/action-catalog.md`

Add row: `user.default_shell` — Change login shell via `chsh`. Idempotent: reads current shell from `/etc/passwd` at plan time; skips if already correct. `username:` uses `privileged: true`. Fields: `shell` (required), `username` (optional).

### `CLAUDE.md`

Action count 48 → 49.
