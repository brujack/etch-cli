# Dotfiles Symlinks Migration — Design Spec

**Date:** 2026-05-14
**Status:** Approved

## Context

Phase 2 of etch-cli development: migrate the `setup_dotfile_symlinks()` workflow from `~/git-repos/personal/dotfiles/lib/helpers.sh` into etch manifests. Goals are (a) to produce working, runnable manifests for both macOS and Linux, and (b) to surface rough edges in etch's action catalog.

## Scope

Four manifest files added to the dotfiles repo. One `etch.yaml` config file set up manually (one-time bootstrap). No changes to etch-cli source code in this phase — gaps are documented and added to the backlog.

## etch.yaml — Version-Controlled and Symlinked

`etch.yaml` is committed to the dotfiles repo at `~/git-repos/personal/dotfiles/etch.yaml` and symlinked to `~/.config/etch/etch.yaml` by `core.yaml`. This means it is version-controlled and available on every machine that clones dotfiles.

**Bootstrap catch:** `core.yaml` creates the symlink, but etch needs `~/.config/etch/etch.yaml` to find `core.yaml` in the first place. On a **fresh machine only**, one manual command is needed before etch can run:

```bash
mkdir -p ~/.config/etch
ln -s ~/git-repos/personal/dotfiles/etch.yaml ~/.config/etch/etch.yaml
```

After that, `etch apply` self-manages the symlink on every subsequent run.

**Contents of `dotfiles/etch.yaml`:**

```yaml
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests
```

## File Structure

```
~/git-repos/personal/dotfiles/
└── manifests/
    └── dotfiles/
        ├── tools.yaml        # oh-my-zsh, powerlevel10k, TPM
        ├── core.yaml         # depends: tools — core dotfile symlinks + credential dirs
        ├── gitconfig.yaml    # depends: core — platform-specific gitconfig symlinks
        └── ai-config.yaml    # depends: core — Claude/Cursor symlinks from ai-config repo
```

Dependency chain: `core` depends on `tools`; `gitconfig` and `ai-config` both depend on `core`.

## Platform Conditionals — use os.name, not os.name

`os.name` is `"unix"` on both macOS and Linux (Rust's `std::env::consts::FAMILY`). Use `os.name` instead: `"macos"` on macOS, `"linux"` on Linux. Every `where:` condition in `gitconfig.yaml` and `ai-config.yaml` uses `os.name`.

## Path Conventions

All paths use the `{{ user.home_dir }}` Tera variable so manifests work across users and platforms without hardcoded paths.

- Dotfiles sources: `{{ user.home_dir }}/git-repos/personal/dotfiles/<path>`
- AI config sources: `{{ user.home_dir }}/git-repos/personal/ai-config/<path>`
- Targets: `{{ user.home_dir }}/<path>`

## Manifest Contents

### tools.yaml

No dependencies.

| Action        | Detail                                                                                                                                                                                                                |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `command.run` | Install oh-my-zsh: `bash -c "[ -d ~/.oh-my-zsh ] \|\| RUNZSH=no KEEP_ZSHRC=yes sh -c \"$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)\""` — inline guard makes it idempotent |
| `git.clone`   | `https://github.com/romkatv/powerlevel10k.git` → `{{ user.home_dir }}/.oh-my-zsh/custom/themes/powerlevel10k`                                                                                                         |
| `git.clone`   | `https://github.com/tmux-plugins/tpm` → `{{ user.home_dir }}/.tmux/plugins/tpm`                                                                                                                                       |

`git.clone` is already idempotent (skips when target directory exists).

### core.yaml

Depends: `./tools`

**Credential directories** (4 × create + chmod):

| Action                                       | Path                            |
| -------------------------------------------- | ------------------------------- |
| `directory.create` + `command.run chmod 700` | `{{ user.home_dir }}/.ssh`      |
| `directory.create` + `command.run chmod 700` | `{{ user.home_dir }}/.warp`     |
| `directory.create` + `command.run chmod 700` | `{{ user.home_dir }}/.tf_creds` |
| `directory.create` + `command.run chmod 700` | `{{ user.home_dir }}/.tsh`      |

**Support directories** (no chmod):

| Action             | Path                                     |
| ------------------ | ---------------------------------------- |
| `directory.create` | `{{ user.home_dir }}/.config/etch`       |
| `directory.create` | `{{ user.home_dir }}/.config/powershell` |

**Core symlinks** (18 × `file.link`, including etch.yaml):

| Source (relative to dotfiles root) | Target                                       |
| ---------------------------------- | -------------------------------------------- |
| `.vimrc`                           | `~/.vimrc`                                   |
| `.p10k.zsh`                        | `~/.p10k.zsh`                                |
| `.tmux.conf`                       | `~/.tmux.conf`                               |
| `scripts`                          | `~/scripts`                                  |
| `.zshrc`                           | `~/.zshrc`                                   |
| `.zprofile`                        | `~/.zprofile`                                |
| `.config/.zshrc.d`                 | `~/.config/.zshrc.d`                         |
| `.config/ccstatusline`             | `~/.config/ccstatusline`                     |
| `bruce.zsh-theme`                  | `~/.oh-my-zsh/custom/themes/bruce.zsh-theme` |
| `.warp/themes`                     | `~/.warp/themes`                             |
| `.warp/launch_configurations`      | `~/.warp/launch_configurations`              |
| `.warp/settings.toml`              | `~/.warp/settings.toml`                      |
| `.ssh/config`                      | `~/.ssh/config`                              |
| `.ssh/teleport.cfg`                | `~/.ssh/teleport.cfg`                        |
| `profile.ps1`                      | `~/.config/powershell/profile.ps1`           |
| `bruce.omp.json`                   | `~/.config/powershell/bruce.omp.json`        |
| `starship.toml`                    | `~/.config/starship.toml`                    |
| `etch.yaml`                        | `~/.config/etch/etch.yaml`                   |

### gitconfig.yaml

Depends: `./core`

| Action             | Where                | Detail                                                               |
| ------------------ | -------------------- | -------------------------------------------------------------------- |
| `directory.create` | —                    | `{{ user.home_dir }}/git-repos/gitlab`                               |
| `file.link`        | `os.name == "macos"` | `dotfiles/.gitconfig_mac` → `~/.gitconfig`                           |
| `file.link`        | `os.name == "macos"` | `dotfiles/.gitconfig_mac_gitlab` → `~/git-repos/gitlab/.gitconfig`   |
| `file.link`        | `os.name == "linux"` | `dotfiles/.gitconfig_linux` → `~/.gitconfig`                         |
| `file.link`        | `os.name == "linux"` | `dotfiles/.gitconfig_linux_gitlab` → `~/git-repos/gitlab/.gitconfig` |

`~/git-repos/gitlab/` is always created (removes the directory-existence conditional from the shell script — creating an empty dir is harmless).

### ai-config.yaml

Depends: `./core`. Sources from `{{ user.home_dir }}/git-repos/personal/ai-config/`.

**`.claude/` symlinks** (9 items, `projects/` excluded):

| Source                                | Target                                  |
| ------------------------------------- | --------------------------------------- |
| `.claude/CLAUDE.md`                   | `~/.claude/CLAUDE.md`                   |
| `.claude/commands`                    | `~/.claude/commands`                    |
| `.claude/hooks`                       | `~/.claude/hooks`                       |
| `.claude/mcp.json.template`           | `~/.claude/mcp.json.template`           |
| `.claude/settings.json`               | `~/.claude/settings.json`               |
| `.claude/settings.local.json`         | `~/.claude/settings.local.json`         |
| `.claude/settings.local.json.example` | `~/.claude/settings.local.json.example` |
| `.claude/skills`                      | `~/.claude/skills`                      |
| `.claude/standards`                   | `~/.claude/standards`                   |

**`.cursor/` symlinks** (4 items, `User/` handled separately):

| Source                  | Target                    |
| ----------------------- | ------------------------- |
| `.cursor/.gitignore`    | `~/.cursor/.gitignore`    |
| `.cursor/plugins`       | `~/.cursor/plugins`       |
| `.cursor/rules`         | `~/.cursor/rules`         |
| `.cursor/skills-cursor` | `~/.cursor/skills-cursor` |

**`.cursor/User/` symlinks** — platform-split, no Cursor-installed check (dangling symlinks are harmless):

| Action             | Where                | Source                          | Target                                                       |
| ------------------ | -------------------- | ------------------------------- | ------------------------------------------------------------ |
| `directory.create` | `os.name == "linux"` | —                               | `~/.config/Cursor/User`                                      |
| `file.link`        | `os.name == "macos"` | `.cursor/User/settings.json`    | `~/Library/Application Support/Cursor/User/settings.json`    |
| `file.link`        | `os.name == "macos"` | `.cursor/User/keybindings.json` | `~/Library/Application Support/Cursor/User/keybindings.json` |
| `file.link`        | `os.name == "macos"` | `.cursor/User/snippets`         | `~/Library/Application Support/Cursor/User/snippets`         |
| `file.link`        | `os.name == "linux"` | `.cursor/User/settings.json`    | `~/.config/Cursor/User/settings.json`                        |
| `file.link`        | `os.name == "linux"` | `.cursor/User/keybindings.json` | `~/.config/Cursor/User/keybindings.json`                     |
| `file.link`        | `os.name == "linux"` | `.cursor/User/snippets`         | `~/.config/Cursor/User/snippets`                             |

## Gaps Surfaced and Backlog Entries

Three new rough edges discovered during this migration. Each gets a backlog entry in `docs/superpowers/README.md`:

| Gap                                   | Workaround in this manifest                              | Backlog entry                                                                  |
| ------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------ |
| No `file.chmod` action                | `command.run: chmod 700 <dir>` after `directory.create`  | `file.chmod` / `file.chown` action                                             |
| No idempotency guard on `command.run` | Inline shell `[ -d path ] \|\|` guard in the command arg | `command.run` skip-if condition (or `file.exists` initializer exposed in YAML) |
| No wildcard `file.link`               | Enumerate all ai-config items explicitly                 | Glob/wildcard support for `file.link`                                          |

The `file.chmod` backlog item already exists from the dotfiles gap analysis. The other two are new.

## What is NOT in scope

- Managing `~/.config/etch/etch.yaml` itself (bootstrap problem)
- The gitlab devtools conditional (`HAS_DEVTOOLS`) — always create the dir
- The Cursor-installed conditional — always symlink, dangling links are harmless
- Any oh-my-zsh plugin management beyond TPM
- Warp terminal or SSH key content (only configs are symlinked)
