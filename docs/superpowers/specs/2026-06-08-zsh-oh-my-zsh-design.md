# zsh.oh-my-zsh Action Design

## Overview

Add a `zsh.oh-my-zsh` action that installs oh-my-zsh via git clone and optionally clones community plugins into `~/.oh-my-zsh/custom/plugins/`.

## Motivation

setup_env.sh installs oh-my-zsh via the official curl installer and then clones plugins (zsh-autosuggestions, etc.) into the custom plugins directory. This feature brings those steps into etch manifests using the existing `Git::Clone` atom — transparent, no shell script download, idempotent via directory existence checks.

## YAML

```yaml
# Install oh-my-zsh only
- action: zsh.oh-my-zsh
  where: 'os.family == "unix"'

# Install oh-my-zsh and community plugins
- action: zsh.oh-my-zsh
  plugins:
      - "https://github.com/zsh-users/zsh-autosuggestions"
      - "https://github.com/zsh-users/zsh-syntax-highlighting"
  where: 'os.family == "unix"'
```

## Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZshOhMyZsh {
    /// Git URLs of oh-my-zsh community plugins to install.
    /// Each URL is cloned into ~/.oh-my-zsh/custom/plugins/<repo-name>.
    /// The repo name is the last path segment of the URL, with any trailing .git stripped.
    #[serde(default)]
    pub plugins: Vec<String>,
}
```

`plugins` defaults to empty. The bare action installs oh-my-zsh only.

No platform gate in the action itself — use `where: 'os.family == "unix"'` in the manifest.

## Plan Logic

### Install step

Check if `~/.oh-my-zsh` exists. If absent, emit:

```
atoms::git::Clone {
    repository: gix::url::parse("https://github.com/ohmyzsh/ohmyzsh"),
    directory: PathBuf::from(shellexpand::tilde("~/.oh-my-zsh")),
}
```

### Plugin steps

For each URL in `plugins`:

1. Extract repo name: take the last non-empty path segment, strip trailing `.git` suffix if present.
    - `https://github.com/zsh-users/zsh-autosuggestions` → `zsh-autosuggestions`
    - `https://github.com/foo/bar.git` → `bar`
    - `https://github.com/foo/bar/` (trailing slash) → `bar`
2. Return `Err` if no name can be extracted (malformed URL with no path segments).
3. If `~/.oh-my-zsh/custom/plugins/{name}` does not exist, emit:
    ```
    atoms::git::Clone {
        repository: gix::url::parse(url),
        directory: PathBuf::from(shellexpand::tilde(&format!("~/.oh-my-zsh/custom/plugins/{name}")).into_owned()),
    }
    ```

### Idempotency

All checks are filesystem (`PathBuf::exists()`). No command execution in `plan()`. Re-running produces no steps when everything is already installed.

## Name Extraction

Extract as a standalone pure function for testability:

```rust
fn plugin_name_from_url(url: &str) -> Option<String> {
    // Strip scheme (e.g. "https://") so segments don't include the host as a valid name
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let segments: Vec<&str> = without_scheme
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    // Need at least host + one path component (e.g. "github.com/org/repo")
    if segments.len() < 2 {
        return None;
    }
    let name = segments.last()?.trim_end_matches(".git");
    if name.is_empty() { None } else { Some(name.to_string()) }
}
```

## Files

| File                               | Change                                                             |
| ---------------------------------- | ------------------------------------------------------------------ |
| `lib/src/actions/zsh/oh_my_zsh.rs` | New — `ZshOhMyZsh` struct + `impl Action` + `plugin_name_from_url` |
| `lib/src/actions/zsh/mod.rs`       | New — module declaration + export                                  |
| `lib/src/actions/mod.rs`           | Register `zsh.oh-my-zsh` variant (6 edits per CLAUDE.md checklist) |
| `examples/zsh/oh-my-zsh.yaml`      | New example                                                        |
| `docs/knowledge/action-catalog.md` | Add `zsh.oh-my-zsh` row                                            |
| `README.md`                        | Add `zsh.oh-my-zsh` to action catalog table                        |

## Tests

### Plan tests (require temp dirs for filesystem checks)

| Scenario                                  | Steps                   |
| ----------------------------------------- | ----------------------- |
| No plugins, `~/.oh-my-zsh` absent         | 1 (clone ohmyzsh)       |
| No plugins, `~/.oh-my-zsh` exists         | 0                       |
| 2 plugins, nothing installed              | 3 (ohmyzsh + 2 plugins) |
| 2 plugins, ohmyzsh exists, plugins absent | 2 (plugins only)        |
| 2 plugins, everything installed           | 0                       |
| Malformed URL (empty path)                | `Err`                   |

### `plugin_name_from_url` unit tests (pure, no filesystem)

| Input                                              | Expected                      |
| -------------------------------------------------- | ----------------------------- |
| `https://github.com/zsh-users/zsh-autosuggestions` | `Some("zsh-autosuggestions")` |
| `https://github.com/foo/bar.git`                   | `Some("bar")`                 |
| `https://github.com/foo/bar/`                      | `Some("bar")`                 |
| `https://example.com` (no path)                    | `None`                        |

### Deserialization tests

- Bare action (no `plugins` field) → `plugins: []`
- Action with two plugin URLs → `plugins` populated

## Out of Scope

- Shell RC sourcing (`.zshrc` plugins list) — left to `file.link`/`file.copy`
- oh-my-zsh update — already handled by `etch update --only git-tools` (via `update.git_tools.oh_my_zsh: true` in `etch.yaml`)
- Plugin removal
- Custom themes
