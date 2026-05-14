# Dotfiles Symlinks Migration Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `setup_dotfile_symlinks()` from dotfiles shell scripts into four etch manifests that run on both macOS and Linux.

**Architecture:** Four YAML manifest files in `~/git-repos/personal/dotfiles/manifests/dotfiles/` plus `etch.yaml` at the dotfiles repo root. `etch.yaml` is symlinked to `~/.config/etch/etch.yaml` by `core.yaml`. Dependency chain: `tools` ← `core` ← `{gitconfig, ai-config}`. Platform conditionals use `os.name` (`"macos"` / `"linux"`). The `file.chmod` gap is worked around with `command.run`.

**Tech Stack:** etch-cli YAML manifests, Tera templates (`{{ user.home_dir }}`), rhai conditions (`where:`), etch actions: `file.link`, `directory.create`, `command.run`, `git.clone`

---

## Files

All manifest files live in the **dotfiles repo** (`~/git-repos/personal/dotfiles/`), not etch-cli.

| File                                | Repo          | Responsibility                               |
| ----------------------------------- | ------------- | -------------------------------------------- |
| `etch.yaml`                         | dotfiles root | Tells etch where manifests live              |
| `manifests/dotfiles/tools.yaml`     | dotfiles      | oh-my-zsh, powerlevel10k, TPM                |
| `manifests/dotfiles/core.yaml`      | dotfiles      | Credential dirs + 18 core symlinks           |
| `manifests/dotfiles/gitconfig.yaml` | dotfiles      | Platform-specific gitconfig symlinks         |
| `manifests/dotfiles/ai-config.yaml` | dotfiles      | Claude + Cursor symlinks from ai-config repo |

The etch-cli `docs/superpowers/README.md` is updated once at the end to mark the plan In Progress and add a status banner.

**Important — bootstrap:** On a fresh machine, before `etch apply` can self-manage `etch.yaml`, run once manually:

```bash
mkdir -p ~/.config/etch
ln -s ~/git-repos/personal/dotfiles/etch.yaml ~/.config/etch/etch.yaml
```

**Verify each manifest with dry-run before applying:**

```bash
etch apply --dry-run -v -m dotfiles.<name>
```

Expected dry-run output format: `[dry run] <action summary>: N step(s) would run`

---

### Task 1: etch.yaml + manifests directory scaffold

**Files:**

- Create: `~/git-repos/personal/dotfiles/etch.yaml`
- Create: `~/git-repos/personal/dotfiles/manifests/dotfiles/` (directory)

- [ ] **Step 1: Create the manifests directory**

```bash
mkdir -p ~/git-repos/personal/dotfiles/manifests/dotfiles
```

- [ ] **Step 2: Create etch.yaml**

Write `~/git-repos/personal/dotfiles/etch.yaml`:

```yaml
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests
```

- [ ] **Step 3: Verify etch picks up the config**

```bash
etch --config ~/git-repos/personal/dotfiles/etch.yaml contexts
```

Expected: prints context variables including `user.home_dir`, `os.name`, `os.family`.
Check that `os.name` shows `"macos"` on macOS or `"linux"` on Linux.

- [ ] **Step 4: Commit to dotfiles**

```bash
cd ~/git-repos/personal/dotfiles
git add etch.yaml manifests/
git commit -m "feat(etch): add etch.yaml and manifests directory scaffold"
```

---

### Task 2: tools.yaml — oh-my-zsh, powerlevel10k, TPM

**Files:**

- Create: `~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml`

- [ ] **Step 1: Create tools.yaml**

Write `~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml`:

```yaml
actions:
    - action: command.run
      command: bash
      args:
          - "-c"
          - '[ -d {{ user.home_dir }}/.oh-my-zsh ] || RUNZSH=no KEEP_ZSHRC=yes sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"'

    - action: git.clone
      repo_url: https://github.com/romkatv/powerlevel10k.git
      directory: "{{ user.home_dir }}/.oh-my-zsh/custom/themes/powerlevel10k"

    - action: git.clone
      repo_url: https://github.com/tmux-plugins/tpm
      directory: "{{ user.home_dir }}/.tmux/plugins/tpm"
```

- [ ] **Step 2: Dry-run and inspect**

```bash
etch apply --dry-run -v -m dotfiles.tools
```

Expected output:

- `[dry run] Run bash: 1 step(s) would run` (or similar summarize text)
- `[dry run] Clone …powerlevel10k: 1 step(s) would run` (if not already cloned; 0 if dir exists)
- `[dry run] Clone …tpm: 1 step(s) would run` (if not already cloned)

If `.oh-my-zsh`, `powerlevel10k`, or `tpm` directories already exist, those actions show 0 steps — that is correct idempotent behaviour.

- [ ] **Step 3: Apply**

```bash
etch apply -m dotfiles.tools
```

- [ ] **Step 4: Verify**

```bash
ls ~/.oh-my-zsh
ls ~/.oh-my-zsh/custom/themes/powerlevel10k
ls ~/.tmux/plugins/tpm
```

Expected: all three directories exist.

- [ ] **Step 5: Commit**

```bash
cd ~/git-repos/personal/dotfiles
git add manifests/dotfiles/tools.yaml
git commit -m "feat(etch): add tools manifest — oh-my-zsh, p10k, TPM"
```

---

### Task 3: core.yaml — credential dirs + 18 core symlinks

**Files:**

- Create: `~/git-repos/personal/dotfiles/manifests/dotfiles/core.yaml`

- [ ] **Step 1: Create core.yaml**

Write `~/git-repos/personal/dotfiles/manifests/dotfiles/core.yaml`:

```yaml
depends:
    - ./tools

actions:
    # ── credential directories ──────────────────────────────────────────────
    - action: directory.create
      path: "{{ user.home_dir }}/.ssh"
    - action: command.run
      command: chmod
      args:
          - "700"
          - "{{ user.home_dir }}/.ssh"

    - action: directory.create
      path: "{{ user.home_dir }}/.warp"
    - action: command.run
      command: chmod
      args:
          - "700"
          - "{{ user.home_dir }}/.warp"

    - action: directory.create
      path: "{{ user.home_dir }}/.tf_creds"
    - action: command.run
      command: chmod
      args:
          - "700"
          - "{{ user.home_dir }}/.tf_creds"

    - action: directory.create
      path: "{{ user.home_dir }}/.tsh"
    - action: command.run
      command: chmod
      args:
          - "700"
          - "{{ user.home_dir }}/.tsh"

    # ── support directories ─────────────────────────────────────────────────
    - action: directory.create
      path: "{{ user.home_dir }}/.config/etch"

    - action: directory.create
      path: "{{ user.home_dir }}/.config/powershell"

    # ── core symlinks ───────────────────────────────────────────────────────
    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.vimrc"
      target: "{{ user.home_dir }}/.vimrc"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.p10k.zsh"
      target: "{{ user.home_dir }}/.p10k.zsh"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.tmux.conf"
      target: "{{ user.home_dir }}/.tmux.conf"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/scripts"
      target: "{{ user.home_dir }}/scripts"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.zshrc"
      target: "{{ user.home_dir }}/.zshrc"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.zprofile"
      target: "{{ user.home_dir }}/.zprofile"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.config/.zshrc.d"
      target: "{{ user.home_dir }}/.config/.zshrc.d"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.config/ccstatusline"
      target: "{{ user.home_dir }}/.config/ccstatusline"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/bruce.zsh-theme"
      target: "{{ user.home_dir }}/.oh-my-zsh/custom/themes/bruce.zsh-theme"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.warp/themes"
      target: "{{ user.home_dir }}/.warp/themes"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.warp/launch_configurations"
      target: "{{ user.home_dir }}/.warp/launch_configurations"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.warp/settings.toml"
      target: "{{ user.home_dir }}/.warp/settings.toml"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.ssh/config"
      target: "{{ user.home_dir }}/.ssh/config"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.ssh/teleport.cfg"
      target: "{{ user.home_dir }}/.ssh/teleport.cfg"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/profile.ps1"
      target: "{{ user.home_dir }}/.config/powershell/profile.ps1"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/bruce.omp.json"
      target: "{{ user.home_dir }}/.config/powershell/bruce.omp.json"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/starship.toml"
      target: "{{ user.home_dir }}/.config/starship.toml"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/etch.yaml"
      target: "{{ user.home_dir }}/.config/etch/etch.yaml"
```

- [ ] **Step 2: Dry-run and inspect**

```bash
etch apply --dry-run -v -m dotfiles.core
```

Review every `  would:` line. Symlinks that already point to the correct source show 0 steps — correct. Any existing file that would be backed up will show as a step. Review before proceeding.

- [ ] **Step 3: Apply**

```bash
etch apply -m dotfiles.core
```

Any existing file at a target path is automatically backed up to `<path>.bak` before the symlink is created.

- [ ] **Step 4: Verify**

```bash
# Credential dirs exist and are 700
stat -f "%Op %N" ~/.ssh ~/.warp ~/.tf_creds ~/.tsh   # macOS (expects 700)
stat -c "%a %n" ~/.ssh ~/.warp ~/.tf_creds ~/.tsh     # Linux (expects 700)

# Core symlinks resolve correctly
ls -la ~/.vimrc ~/.p10k.zsh ~/.tmux.conf ~/.zshrc ~/.zprofile
ls -la ~/.config/.zshrc.d ~/.config/etch/etch.yaml
```

Expected: `~/.config/etch/etch.yaml` is a symlink to `…/dotfiles/etch.yaml`. Credential dirs have mode `700`.

- [ ] **Step 5: Commit**

```bash
cd ~/git-repos/personal/dotfiles
git add manifests/dotfiles/core.yaml
git commit -m "feat(etch): add core manifest — credential dirs and 18 dotfile symlinks"
```

---

### Task 4: gitconfig.yaml — platform-specific gitconfig symlinks

**Files:**

- Create: `~/git-repos/personal/dotfiles/manifests/dotfiles/gitconfig.yaml`

- [ ] **Step 1: Create gitconfig.yaml**

Write `~/git-repos/personal/dotfiles/manifests/dotfiles/gitconfig.yaml`:

```yaml
depends:
    - ./core

actions:
    - action: directory.create
      path: "{{ user.home_dir }}/git-repos/gitlab"

    - action: file.link
      where: 'os.name == "macos"'
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.gitconfig_mac"
      target: "{{ user.home_dir }}/.gitconfig"

    - action: file.link
      where: 'os.name == "macos"'
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.gitconfig_mac_gitlab"
      target: "{{ user.home_dir }}/git-repos/gitlab/.gitconfig"

    - action: file.link
      where: 'os.name == "linux"'
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.gitconfig_linux"
      target: "{{ user.home_dir }}/.gitconfig"

    - action: file.link
      where: 'os.name == "linux"'
      source: "{{ user.home_dir }}/git-repos/personal/dotfiles/.gitconfig_linux_gitlab"
      target: "{{ user.home_dir }}/git-repos/gitlab/.gitconfig"
```

- [ ] **Step 2: Dry-run and inspect**

```bash
etch apply --dry-run -v -m dotfiles.gitconfig
```

On macOS: only the `macos` actions should show steps. On Linux: only the `linux` actions. The `linux` actions on macOS and vice versa should either show 0 steps or be absent.

- [ ] **Step 3: Apply**

```bash
etch apply -m dotfiles.gitconfig
```

- [ ] **Step 4: Verify**

```bash
ls -la ~/.gitconfig
readlink ~/.gitconfig   # should end in .gitconfig_mac or .gitconfig_linux
ls -la ~/git-repos/gitlab/.gitconfig
```

- [ ] **Step 5: Commit**

```bash
cd ~/git-repos/personal/dotfiles
git add manifests/dotfiles/gitconfig.yaml
git commit -m "feat(etch): add gitconfig manifest — platform-specific gitconfig symlinks"
```

---

### Task 5: ai-config.yaml — Claude and Cursor symlinks

**Files:**

- Create: `~/git-repos/personal/dotfiles/manifests/dotfiles/ai-config.yaml`

> **Caution:** This manifest creates symlinks inside `~/.claude/` and `~/.cursor/`. Review the dry-run carefully — existing files will be backed up to `.bak`. The `projects/` directory inside `~/.claude/` is intentionally excluded.

- [ ] **Step 1: Create ai-config.yaml**

Write `~/git-repos/personal/dotfiles/manifests/dotfiles/ai-config.yaml`:

```yaml
depends:
    - ./core

actions:
    # ── .claude/ symlinks (projects/ excluded) ─────────────────────────────
    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/CLAUDE.md"
      target: "{{ user.home_dir }}/.claude/CLAUDE.md"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/commands"
      target: "{{ user.home_dir }}/.claude/commands"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/hooks"
      target: "{{ user.home_dir }}/.claude/hooks"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/mcp.json.template"
      target: "{{ user.home_dir }}/.claude/mcp.json.template"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/settings.json"
      target: "{{ user.home_dir }}/.claude/settings.json"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/settings.local.json"
      target: "{{ user.home_dir }}/.claude/settings.local.json"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/settings.local.json.example"
      target: "{{ user.home_dir }}/.claude/settings.local.json.example"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/skills"
      target: "{{ user.home_dir }}/.claude/skills"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.claude/standards"
      target: "{{ user.home_dir }}/.claude/standards"

    # ── .cursor/ symlinks (User/ handled separately) ───────────────────────
    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/.gitignore"
      target: "{{ user.home_dir }}/.cursor/.gitignore"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/plugins"
      target: "{{ user.home_dir }}/.cursor/plugins"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/rules"
      target: "{{ user.home_dir }}/.cursor/rules"

    - action: file.link
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/skills-cursor"
      target: "{{ user.home_dir }}/.cursor/skills-cursor"

    # ── .cursor/User/ — macOS ───────────────────────────────────────────────
    - action: file.link
      where: 'os.name == "macos"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/settings.json"
      target: "{{ user.home_dir }}/Library/Application Support/Cursor/User/settings.json"

    - action: file.link
      where: 'os.name == "macos"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/keybindings.json"
      target: "{{ user.home_dir }}/Library/Application Support/Cursor/User/keybindings.json"

    - action: file.link
      where: 'os.name == "macos"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/snippets"
      target: "{{ user.home_dir }}/Library/Application Support/Cursor/User/snippets"

    # ── .cursor/User/ — Linux ───────────────────────────────────────────────
    - action: directory.create
      where: 'os.name == "linux"'
      path: "{{ user.home_dir }}/.config/Cursor/User"

    - action: file.link
      where: 'os.name == "linux"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/settings.json"
      target: "{{ user.home_dir }}/.config/Cursor/User/settings.json"

    - action: file.link
      where: 'os.name == "linux"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/keybindings.json"
      target: "{{ user.home_dir }}/.config/Cursor/User/keybindings.json"

    - action: file.link
      where: 'os.name == "linux"'
      source: "{{ user.home_dir }}/git-repos/personal/ai-config/.cursor/User/snippets"
      target: "{{ user.home_dir }}/.config/Cursor/User/snippets"
```

- [ ] **Step 2: Dry-run and inspect**

```bash
etch apply --dry-run -v -m dotfiles.ai-config
```

Review each `  would:` line. Any `~/.claude/<item>` that currently exists as a real file/directory will be backed up — confirm this is acceptable before applying.

- [ ] **Step 3: Apply**

```bash
etch apply -m dotfiles.ai-config
```

- [ ] **Step 4: Verify**

```bash
# .claude symlinks resolve to ai-config
readlink ~/.claude/settings.json     # should end in ai-config/.claude/settings.json
readlink ~/.claude/skills            # should end in ai-config/.claude/skills
readlink ~/.claude/standards         # should end in ai-config/.claude/standards

# .cursor symlinks
readlink ~/.cursor/rules

# macOS Cursor User settings
readlink ~/Library/Application\ Support/Cursor/User/settings.json
```

- [ ] **Step 5: Commit**

```bash
cd ~/git-repos/personal/dotfiles
git add manifests/dotfiles/ai-config.yaml
git commit -m "feat(etch): add ai-config manifest — Claude and Cursor symlinks"
```

---

### Task 6: Update etch-cli docs

**Files:**

- Modify: `~/git-repos/personal/etch-cli/docs/superpowers/README.md`
- Modify: `~/git-repos/personal/etch-cli/docs/superpowers/plans/2026-05-14-dotfiles-symlinks.md` (this file — add status banner)

- [ ] **Step 1: Mark plan In Progress in README**

In `~/git-repos/personal/etch-cli/docs/superpowers/README.md`, the row for `dotfiles-symlinks` currently shows `Pending`. Change to `In Progress` (or `Done` if all tasks above are complete).

- [ ] **Step 2: Add status banner to this plan file**

Add `> **Status: DONE**` immediately after the plan header (before the Goal line) when work starts. Change to `> **Status: DONE**` once all tasks are verified.

- [ ] **Step 3: Commit etch-cli docs**

```bash
cd ~/git-repos/personal/etch-cli
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-14-dotfiles-symlinks.md
git commit -m "docs: update dotfiles-symlinks plan status"
```

- [ ] **Step 4: Push dotfiles to remote**

```bash
cd ~/git-repos/personal/dotfiles
git push
```
