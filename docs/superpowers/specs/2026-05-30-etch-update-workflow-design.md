# Spec: etch update workflow

**Date:** 2026-05-30
**Status:** Pending

## Problem

`dotfiles/setup_env.sh -t update` (i.e. `run_update()`) is the single authoritative updater for all tools and packages on both Mac and Linux. It handles brew, mas, claude plugins, apt/snap, pip, npm, rustup, git-based tools (oh-my-zsh, tpm, tfenv, ai-config, dotfiles), cheat.sh, and gems.

There is no etch equivalent. The shell script must be kept alive solely for this. Goal: replicate the full update workflow in etch-cli so etch becomes the single way to update the machine.

## Approach

**`etch update` subcommand** — not manifest-driven. The update workflow is built into the etch binary and invoked as `etch update`. Selective updates via flags (mirrors `UPDATE_BREW=1`, etc.).

Machine-specific config (which update steps are enabled, repo URLs, tool paths) is expressed in `etch.yaml` via the existing `variables:` section and a new top-level `update:` stanza (see below).

**Machine config lives in `etch-config` repo** (`~/git-repos/personal/etch-config`), with per-machine directories: `studio/` (macOS, Mac Studio M1 Ultra) and `workstation/` (Linux, AMD Ryzen 9). Each machine's `etch.yaml` sets `manifest_paths` to its own directory. The `update:` stanza is added to each machine's `etch.yaml` there.

## CLI design

```
etch update [flags]

Flags:
  --brew          Update brew formulae and casks
  --system        Run softwareupdate (macOS)
  --mas           Upgrade Mac App Store apps
  --claude        Update Claude plugins + npm globals
  --packages      Upgrade apt/snap packages (Linux)
  --pip           Upgrade pip packages
  --rust          Update rustup toolchain + cargo-nextest
  --git-tools     Pull ai-config, dotfiles, oh-my-zsh, tpm, tfenv
  --gems          Update Ruby gems
  --cheatsh       Update cheat.sh binary

(no flags) = run all enabled steps for this machine
```

All flags are opt-in overrides — if none are passed, every step whose `where:` condition matches runs.

## etch.yaml update stanza

New top-level key `update:` in `~/.config/etch/etch.yaml`:

```yaml
update:
    git_tools:
        ai_config: "git@github.com:brujack/ai-config.git"
        dotfiles: "git@github.com:brujack/dotfiles.git"
        oh_my_zsh: true # pull if directory exists
        tpm: true
        tfenv: true
    claude:
        plugins:
            - superpowers@claude-plugins-official
            - code-review@claude-plugins-official
            - context7@claude-plugins-official
            - context-mode@context-mode
            - rust-analyzer-lsp@claude-plugins-official
            - pyright-lsp@claude-plugins-official
            - caveman@caveman
            - firecrawl@firecrawl
            - skill-creator@claude-plugins-official
            - frontend-design@claude-plugins-official
            - security-guidance@claude-plugins-official
            - ansible-cop-review@claude-ansible-skills
            - warp@claude-code-warp
        npm_globals:
            - firecrawl-cli
```

Platform steps (brew, apt, mas, softwareupdate, snap) are auto-detected from OS context — no config needed for those.

`pip`, `rust`, `gems` are enabled based on `variables.has_devtools` / `variables.has_rust` in the existing variables block.

## Component map

| Step                        | Mechanism                                                   | Platform gate    | Variable gate                    |
| --------------------------- | ----------------------------------------------------------- | ---------------- | -------------------------------- |
| brew.upgrade + brew.cleanup | etch-lib `brew.upgrade`/`brew.cleanup` atoms directly       | macOS + Linux    | —                                |
| softwareupdate              | `Command::new("softwareupdate")` privileged                 | macOS only       | —                                |
| mas.upgrade                 | etch-lib `mas.upgrade` atom                                 | macOS only       | —                                |
| claude plugins              | `Command::new("claude") args ["plugins","update",name]` × N | all              | `update.claude.plugins` list     |
| npm globals                 | `Command::new("npm") args ["install","-g",pkg]` × N         | all              | `update.claude.npm_globals` list |
| terraform-skill             | `Command::new("bash")` inline install script                | all              | —                                |
| apt upgrade                 | `Command::new("apt-get")` privileged                        | Linux only       | —                                |
| snap refresh                | `Command::new("snap")` privileged                           | Linux + has_snap | —                                |
| mas.upgrade                 | etch-lib `mas.upgrade` atom                                 | macOS only       | —                                |
| pip upgrade                 | `Command::new("python3")` calling update-pip logic          | all              | has_devtools                     |
| rustup update               | `Command::new("rustup")`                                    | all              | has_rust                         |
| cargo-nextest               | `Command::new("cargo")`                                     | all              | has_rust                         |
| git.pull repos              | etch-lib `git.pull` atom for each repo                      | all              | update.git_tools config          |
| gem update                  | `Command::new("gem")`                                       | all              | —                                |
| cheat.sh                    | `Command::new("curl")` + chmod                              | all              | ~/bin/cht.sh exists              |

## Implementation — app/src/commands/update.rs

```rust
#[derive(Parser, Debug)]
pub(crate) struct Update {
    #[arg(long)] pub brew: bool,
    #[arg(long)] pub system: bool,
    #[arg(long)] pub mas: bool,
    #[arg(long)] pub claude: bool,
    #[arg(long)] pub packages: bool,
    #[arg(long)] pub pip: bool,
    #[arg(long)] pub rust: bool,
    #[arg(long)] pub git_tools: bool,
    #[arg(long)] pub gems: bool,
    #[arg(long)] pub cheatsh: bool,
}

impl EtchCommand for Update {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let run_all = !self.any_flag_set();
        // run steps in order, gated by flag or run_all, plus OS/variable checks
    }
}
```

Each step is a function `update_brew(&runtime) -> anyhow::Result<()>` etc. Steps run sequentially; failure of one logs an error but doesn't stop the rest (matches shell script behavior). Final summary prints which steps succeeded/failed.

## etch.yaml update config parsing

New field in `lib/src/config/mod.rs`:

```rust
#[derive(Deserialize, Default)]
pub struct UpdateConfig {
    pub git_tools: Option<GitToolsConfig>,
    pub claude:    Option<ClaudeUpdateConfig>,
}

#[derive(Deserialize, Default)]
pub struct GitToolsConfig {
    pub ai_config:  Option<String>,   // repo URL
    pub dotfiles:   Option<String>,
    pub oh_my_zsh:  Option<bool>,
    pub tpm:        Option<bool>,
    pub tfenv:      Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct ClaudeUpdateConfig {
    pub plugins:     Vec<String>,
    pub npm_globals: Vec<String>,
}
```

Added to existing `Config` struct as `pub update: UpdateConfig`.

## Variables — existing vs new

These `variables:` keys already exist in `etch-config/studio/etch.yaml` and `etch-config/workstation/etch.yaml` and are read directly by the update command:

| Variable       | studio             | workstation       | Used by step |
| -------------- | ------------------ | ----------------- | ------------ |
| `has_rust`     | true               | true              | rust         |
| `has_devtools` | true               | true              | pip          |
| `has_snap`     | false              | true              | packages     |
| `dotfiles_dir` | `/Users/bruce/...` | `/home/bruce/...` | git-tools    |

New `update:` stanza to add to each machine's `etch.yaml` (repo URLs, plugin list):

```yaml
# studio/etch.yaml and workstation/etch.yaml
update:
    git_tools:
        ai_config: "git@github.com:brujack/ai-config.git"
        dotfiles: "git@github.com:brujack/dotfiles.git"
        oh_my_zsh: true
        tpm: true
        tfenv: false # true on workstation
    claude:
        plugins:
            - superpowers@claude-plugins-official
            - code-review@claude-plugins-official
            - context7@claude-plugins-official
            - context-mode@context-mode
            - rust-analyzer-lsp@claude-plugins-official
            - pyright-lsp@claude-plugins-official
            - caveman@caveman
            - firecrawl@firecrawl
            - skill-creator@claude-plugins-official
            - frontend-design@claude-plugins-official
            - security-guidance@claude-plugins-official
            - ansible-cop-review@claude-ansible-skills
            - warp@claude-code-warp
        npm_globals:
            - firecrawl-cli
```

## Etch-lib gaps

| Gap                                    | Fix                                                                                        |
| -------------------------------------- | ------------------------------------------------------------------------------------------ |
| `git.pull` skip when directory absent  | Add `skip_if_not_exists: Option<String>` to `GitPull` — skip entire action if path missing |
| No `Update` command in `Commands` enum | Add `Update(Update)` to `app/src/config/mod.rs` Commands                                   |

## Summary output

After all steps, print a one-line status per step:

```
  brew          ✓
  softwareupdate ✓
  mas           ✓
  claude        ✓
  rust          ✓ (1 toolchain updated)
  git-tools     ✓
  gems          skipped (not installed)
  pip           ✗ (exit 1)
```

## Out of scope

- Brewfile drift check — future work
- AWS CLI update — `binary.url` action in a separate manifest
- Interactive pip virtualenv selection (just use the default python3)
- Windows / PowerShell path

## Implementation order

1. `Config` struct: add `update: UpdateConfig` + parse from etch.yaml
2. `Commands` enum: add `Update` variant + dispatch in `execute()`
3. `app/src/commands/update.rs`: skeleton + `any_flag_set()` logic
4. Steps (in order of safety): git-tools → brew → mas → claude → rust → packages → system → pip → gems → cheatsh
5. Summary output
6. Add `skip_if_not_exists` to `git.pull` (needed for optional tool pulls)
