> **Status: DONE** — implemented in PR #98, merged 2026-06-08.

# zsh.oh-my-zsh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `zsh.oh-my-zsh` action that installs oh-my-zsh via git clone and optionally clones community plugins into `~/.oh-my-zsh/custom/plugins/`.

**Architecture:** New `lib/src/actions/zsh/oh_my_zsh.rs` contains `ZshOhMyZsh` struct, `plugin_name_from_url` helper, and `impl Action`. Both install and plugin steps reuse the existing `crate::atoms::git::Clone` atom. Registered in `lib/src/actions/mod.rs` following the exact same pattern as every other action.

**Tech Stack:** Rust, `gix::url::parse` (already in lib deps), `shellexpand` (already in lib deps), `crate::atoms::git::Clone` (existing atom), serde, schemars

---

### Task 1: Scaffold `zsh.oh-my-zsh` and register it

**Files:**

- Create: `lib/src/actions/zsh/oh_my_zsh.rs`
- Create: `lib/src/actions/zsh/mod.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Create `lib/src/actions/zsh/oh_my_zsh.rs` with stub plan**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZshOhMyZsh {
    /// Git URLs of oh-my-zsh community plugins to install.
    /// Each URL is cloned into ~/.oh-my-zsh/custom/plugins/<repo-name>.
    /// The repo name is the last path segment of the URL, with any trailing .git stripped.
    #[serde(default)]
    pub plugins: Vec<String>,
}

impl Action for ZshOhMyZsh {
    fn summarize(&self) -> String {
        if self.plugins.is_empty() {
            String::from("Installing oh-my-zsh")
        } else {
            format!("Installing oh-my-zsh with {} plugin(s)", self.plugins.len())
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }
}

pub(crate) fn plugin_name_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let segments: Vec<&str> = without_scheme
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let name = segments.last()?.trim_end_matches(".git");
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}
```

- [ ] **Step 2: Create `lib/src/actions/zsh/mod.rs`**

```rust
mod oh_my_zsh;
pub(crate) use oh_my_zsh::{ZshOhMyZsh, plugin_name_from_url};
```

- [ ] **Step 3: Add `mod zsh;` in `lib/src/actions/mod.rs`**

Find (module declarations near the top, around line 10):

```rust
mod git;
```

Add immediately after:

```rust
mod zsh;
```

- [ ] **Step 4: Add `use zsh::ZshOhMyZsh;` import in `lib/src/actions/mod.rs`**

Find (use statements, around line 43):

```rust
use git::{GitClone, GitConfig, GitPull};
```

Add immediately after:

```rust
use zsh::ZshOhMyZsh;
```

- [ ] **Step 5: Add enum variant in `lib/src/actions/mod.rs`**

Find (end of the `Actions` enum, around line 286):

```rust
    #[serde(rename = "pyenv.virtualenv")]
    PyenvVirtualenv(ConditionalVariantAction<PyenvVirtualenv>),
}
```

Replace with:

```rust
    #[serde(rename = "pyenv.virtualenv")]
    PyenvVirtualenv(ConditionalVariantAction<PyenvVirtualenv>),

    #[serde(rename = "zsh.oh-my-zsh")]
    ZshOhMyZsh(ConditionalVariantAction<ZshOhMyZsh>),
}
```

- [ ] **Step 6: Add `inner_ref()` match arm in `lib/src/actions/mod.rs`**

Find (in the `inner_ref()` impl):

```rust
            Actions::PyenvVirtualenv(a) => a,
        }
    }
```

Replace with:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
        }
    }
```

- [ ] **Step 7: Add `notify()` match arm in `lib/src/actions/mod.rs`**

Find (in the `notify()` impl):

```rust
            Actions::PyenvVirtualenv(a) => &a.notify,
        }
    }
}
```

Replace with:

```rust
            Actions::PyenvVirtualenv(a) => &a.notify,
            Actions::ZshOhMyZsh(a) => &a.notify,
        }
    }
}
```

- [ ] **Step 8: Add `Deref` match arm in `lib/src/actions/mod.rs`**

Find (in the `Deref` impl):

```rust
            Actions::PyenvVirtualenv(a) => a,
        }
    }
}
```

Replace with:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
        }
    }
}
```

- [ ] **Step 9: Add `Display` match arm in `lib/src/actions/mod.rs`**

Find (in the `Display` impl):

```rust
            Actions::PyenvVirtualenv(_) => "pyenv.virtualenv",
        };
```

Replace with:

```rust
            Actions::PyenvVirtualenv(_) => "pyenv.virtualenv",
            Actions::ZshOhMyZsh(_) => "zsh.oh-my-zsh",
        };
```

- [ ] **Step 10: Update `all_major_action_variants_can_be_deserialized` test**

Find (end of YAML + count assertion, around line 691):

```
  - action: ruby.chruby
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(32, manifest.actions.len());
```

Replace with:

```
  - action: ruby.chruby
  - action: zsh.oh-my-zsh
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(33, manifest.actions.len());
```

- [ ] **Step 11: Update `all_action_variants_inner_ref_and_deref` test**

In the `all_action_variants_inner_ref_and_deref` test YAML, find:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: package.upgrade
  provider: snap
```

Add `  - action: zsh.oh-my-zsh` after the pyenv.virtualenv entry, before `- action: package.upgrade`.

Then find `assert_eq!(45, manifest.actions.len());` inside this test and change to `assert_eq!(46, manifest.actions.len());`.

- [ ] **Step 12: Update `all_action_variants_display` test**

In the `all_action_variants_display` test YAML, find:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: claude.marketplace
```

Add `  - action: zsh.oh-my-zsh` after pyenv.virtualenv, before claude.marketplace.

Then find `assert_eq!(45, manifest.actions.len());` inside this test and change to `assert_eq!(46, manifest.actions.len());`.

Then add after `assert!(names.contains(&"pyenv.virtualenv".to_string()));`:

```rust
        assert!(names.contains(&"zsh.oh-my-zsh".to_string()));
```

- [ ] **Step 13: Compile check**

```bash
cd /path/to/worktree && cargo check -p etch-lib 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 14: Run tests**

```bash
cargo test -p etch-lib 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 15: Commit**

```bash
git add lib/src/actions/zsh/oh_my_zsh.rs lib/src/actions/zsh/mod.rs lib/src/actions/mod.rs
git commit -m "feat(zsh): scaffold zsh.oh-my-zsh action and register it"
```

---

### Task 2: TDD — `plugin_name_from_url` unit tests and `plan()` implementation

**Files:**

- Modify: `lib/src/actions/zsh/oh_my_zsh.rs`

- [ ] **Step 1: Add failing tests to `oh_my_zsh.rs`**

Add at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use tempfile::TempDir;

    // ── plugin_name_from_url ──────────────────────────────────────────────

    #[test]
    fn plugin_name_from_https_url() {
        assert_eq!(
            Some(String::from("zsh-autosuggestions")),
            plugin_name_from_url("https://github.com/zsh-users/zsh-autosuggestions")
        );
    }

    #[test]
    fn plugin_name_strips_git_suffix() {
        assert_eq!(
            Some(String::from("bar")),
            plugin_name_from_url("https://github.com/foo/bar.git")
        );
    }

    #[test]
    fn plugin_name_strips_trailing_slash() {
        assert_eq!(
            Some(String::from("bar")),
            plugin_name_from_url("https://github.com/foo/bar/")
        );
    }

    #[test]
    fn plugin_name_returns_none_for_no_path() {
        assert_eq!(None, plugin_name_from_url("https://example.com"));
    }

    // ── plan() ───────────────────────────────────────────────────────────

    #[test]
    fn plan_no_plugins_omz_absent_emits_one_step() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        // omz_dir does NOT exist
        let action = ZshOhMyZsh {
            plugins: vec![],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_no_plugins_omz_present_emits_zero_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        std::fs::create_dir_all(&omz_dir).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_two_plugins_nothing_installed_emits_three_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        // nothing exists
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len());
    }

    #[test]
    fn plan_two_plugins_omz_exists_plugins_absent_emits_two_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        std::fs::create_dir_all(&omz_dir).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
    }

    #[test]
    fn plan_everything_installed_emits_zero_steps() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let plugins_dir = omz_dir.join("custom/plugins");
        std::fs::create_dir_all(plugins_dir.join("zsh-autosuggestions")).unwrap();
        std::fs::create_dir_all(plugins_dir.join("zsh-syntax-highlighting")).unwrap();
        let action = ZshOhMyZsh {
            plugins: vec![
                String::from("https://github.com/zsh-users/zsh-autosuggestions"),
                String::from("https://github.com/zsh-users/zsh-syntax-highlighting"),
            ],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_malformed_url_returns_err() {
        let tmp = TempDir::new().unwrap();
        let omz_dir = tmp.path().join(".oh-my-zsh");
        let action = ZshOhMyZsh {
            plugins: vec![String::from("https://example.com")],
            omz_dir: omz_dir.to_string_lossy().to_string(),
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    // ── deserialization ───────────────────────────────────────────────────

    #[test]
    fn it_can_be_deserialized_without_plugins() {
        let yaml = r#"
actions:
  - action: zsh.oh-my-zsh
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::ZshOhMyZsh(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert!(action.plugins.is_empty());
    }

    #[test]
    fn it_can_be_deserialized_with_plugins() {
        let yaml = r#"
actions:
  - action: zsh.oh-my-zsh
    plugins:
      - "https://github.com/zsh-users/zsh-autosuggestions"
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::ZshOhMyZsh(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(1, action.plugins.len());
        assert_eq!(
            "https://github.com/zsh-users/zsh-autosuggestions",
            action.plugins[0]
        );
    }
}
```

**Note:** The tests use `omz_dir` as an injectable path field on `ZshOhMyZsh`. You will add this field in the next step. The tests will fail with a compile error until the field exists — that is expected TDD RED state.

- [ ] **Step 2: Run tests to confirm RED state**

```bash
cargo test -p etch-lib zsh 2>&1 | tail -15
```

Expected: compile error — `omz_dir` field not found on `ZshOhMyZsh`.

- [ ] **Step 3: Add `omz_dir` field and implement `plan()`**

Replace the entire content of `lib/src/actions/zsh/oh_my_zsh.rs` with:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZshOhMyZsh {
    /// Git URLs of oh-my-zsh community plugins to install.
    /// Each URL is cloned into ~/.oh-my-zsh/custom/plugins/<repo-name>.
    /// The repo name is the last path segment of the URL, with any trailing .git stripped.
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Override the oh-my-zsh install directory. Defaults to ~/.oh-my-zsh.
    /// Used in tests to inject a temp directory; not normally set in manifests.
    #[serde(default = "default_omz_dir", skip_serializing)]
    #[schemars(skip)]
    pub omz_dir: String,
}

fn default_omz_dir() -> String {
    shellexpand::tilde("~/.oh-my-zsh").into_owned()
}

impl Default for ZshOhMyZsh {
    fn default() -> Self {
        Self {
            plugins: vec![],
            omz_dir: default_omz_dir(),
        }
    }
}

impl Action for ZshOhMyZsh {
    fn summarize(&self) -> String {
        if self.plugins.is_empty() {
            String::from("Installing oh-my-zsh")
        } else {
            format!("Installing oh-my-zsh with {} plugin(s)", self.plugins.len())
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::git::Clone;

        let omz_path = PathBuf::from(&self.omz_dir);
        let plugins_base = omz_path.join("custom/plugins");

        let mut steps: Vec<Step> = vec![];

        // Step 1: install oh-my-zsh if not present
        if !omz_path.exists() {
            let url = gix::url::parse("https://github.com/ohmyzsh/ohmyzsh".into())?;
            steps.push(Step {
                atom: Box::new(Clone {
                    repository: url,
                    directory: omz_path.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        // Step 2: clone any plugins not yet present
        for plugin_url in &self.plugins {
            let name = plugin_name_from_url(plugin_url)
                .ok_or_else(|| anyhow::anyhow!("cannot extract plugin name from URL: {plugin_url}"))?;
            let plugin_dir = plugins_base.join(&name);
            if !plugin_dir.exists() {
                let url = gix::url::parse(plugin_url.as_str().into())?;
                steps.push(Step {
                    atom: Box::new(Clone {
                        repository: url,
                        directory: plugin_dir,
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                });
            }
        }

        Ok(steps)
    }
}

pub(crate) fn plugin_name_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let segments: Vec<&str> = without_scheme
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let name = segments.last()?.trim_end_matches(".git");
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}
```

- [ ] **Step 4: Run tests to confirm GREEN**

```bash
cargo test -p etch-lib zsh 2>&1 | tail -20
```

Expected: all 12 tests pass (`plugin_name_*` ×4, `plan_*` ×6, `it_can_be_deserialized_*` ×2).

- [ ] **Step 5: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/zsh/oh_my_zsh.rs
git commit -m "feat(zsh): implement zsh.oh-my-zsh plan with git clone for omz and plugins"
```

---

### Task 3: Add example, update catalogs, open PR

**Files:**

- Create: `examples/zsh/oh-my-zsh.yaml`
- Modify: `docs/knowledge/action-catalog.md`
- Modify: `README.md`

- [ ] **Step 1: Create `examples/zsh/oh-my-zsh.yaml`**

```yaml
# zsh.oh-my-zsh — install oh-my-zsh and optionally clone community plugins
#
# Installs oh-my-zsh by cloning https://github.com/ohmyzsh/ohmyzsh to ~/.oh-my-zsh.
# Shell RC sourcing (source $ZSH/oh-my-zsh.sh) and the plugins: list in .zshrc are
# left to file.link or file.copy — this action only installs the files on disk.
#
# oh-my-zsh updates are handled by etch update (git_tools.oh_my_zsh: true in etch.yaml).

# Minimal: install oh-my-zsh only
- action: zsh.oh-my-zsh
  where: 'os.family == "unix"'

---
# Install oh-my-zsh and clone two community plugins.
# The plugin name in ~/.zshrc must match the cloned directory name (last URL segment).
- action: zsh.oh-my-zsh
  plugins:
      - "https://github.com/zsh-users/zsh-autosuggestions"
      - "https://github.com/zsh-users/zsh-syntax-highlighting"
  where: 'os.family == "unix"'
```

- [ ] **Step 2: Add `zsh.oh-my-zsh` row to `docs/knowledge/action-catalog.md`**

Read the file to find the correct insertion point. Add a new `zsh.oh-my-zsh` row in the table. If there is no `zsh` section, add it at the end of the table. The row to add:

```
| `zsh.oh-my-zsh` | Install oh-my-zsh by git-cloning ohmyzsh/ohmyzsh to `~/.oh-my-zsh`; optionally clone community plugins into `~/.oh-my-zsh/custom/plugins/<name>`. Idempotent: skips install if `~/.oh-my-zsh` exists; skips each plugin if its directory exists. Shell RC sourcing and `.zshrc` plugins list are left to the manifest. | `plugins` (Vec<String> — default empty; each entry is a git URL cloned as `~/.oh-my-zsh/custom/plugins/<last-url-segment-without-.git>`; errors if URL has no valid path component) |
```

- [ ] **Step 3: Add `zsh.oh-my-zsh` row to `README.md` action catalog table**

Read README.md to find the action catalog table. Add a `zsh.oh-my-zsh` row. Description: `Install oh-my-zsh and optionally clone community plugins`

- [ ] **Step 4: Commit examples and docs**

```bash
git add examples/zsh/oh-my-zsh.yaml docs/knowledge/action-catalog.md README.md
git commit -m "docs: add zsh.oh-my-zsh example and catalog entries"
```

- [ ] **Step 5: Push and open PR**

```bash
git push -u origin HEAD
gh pr create --repo brujack/etch-cli \
  --title "feat(zsh): add zsh.oh-my-zsh action" \
  --body "$(cat <<'EOF'
## Summary
- New \`zsh.oh-my-zsh\` action: installs oh-my-zsh via git clone to \`~/.oh-my-zsh\`
- Optional \`plugins:\` list of git URLs; each cloned to \`~/.oh-my-zsh/custom/plugins/<name>\`
- Both install and plugin steps use the existing \`atoms::git::Clone\` atom
- Fully idempotent: skips steps where directory already exists
- Shell RC sourcing and \`.zshrc\` plugins list left to the manifest

## Test Plan
- [x] \`make test\` passes
- [x] No plugins, omz absent → 1 step
- [x] No plugins, omz present → 0 steps
- [x] 2 plugins, nothing installed → 3 steps
- [x] 2 plugins, omz exists, plugins absent → 2 steps
- [x] Everything installed → 0 steps
- [x] Malformed URL (no path) → \`Err\`
- [x] \`plugin_name_from_url\` handles .git suffix, trailing slash, no-path case

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 6: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli --watch
```

Fix any failures before merging.

- [ ] **Step 7: After merge — update plan index** _(do this on main after the PR merges, not inside the worktree)_

In `docs/superpowers/README.md`, update the `zsh-oh-my-zsh` row status to `Done` and add the plan link.

Add `> **Status: DONE**` banner at the top of this plan file.
