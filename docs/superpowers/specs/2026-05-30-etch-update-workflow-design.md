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

Mirrors the dotfiles `update_summary.sh` pattern: pre-snapshot before each step, post-snapshot + diff after, then a formatted table + log append.

### Rust types

```rust
// app/src/commands/update.rs

pub(crate) enum StepStatus {
    Ok(String),    // detail: "3 formulae (vim, git, curl)"
    Fail(String),  // detail: "exit 1 — see output above"
    Skip(String),  // reason: "not applicable" / "not installed"
    Warn(String),  // detail: for advisory findings (brew-drift future)
}

pub(crate) struct UpdateStepResult {
    pub name:   &'static str,
    pub status: StepStatus,
    pub detail: Option<String>,  // multiline block for drift-style output
}
```

Each step function returns `UpdateStepResult`. The orchestrator collects all results and calls `print_summary()` at the end.

### Pre-snapshot logic per step

Before running each step, capture the pre-state:

| Step           | Pre-snapshot command                                               |
| -------------- | ------------------------------------------------------------------ |
| brew           | `brew list --formula --versions` + `--cask --versions` (two files) |
| softwareupdate | `softwareupdate -l` → grep `^\* Label:` lines                      |
| mas            | `mas list`                                                         |
| claude         | `claude plugins list` → grep `Version:` lines                      |
| npm            | `npm list -g --depth=0`                                            |
| apt            | `dpkg-query -W -f='${Package} ${Version}\n'`                       |
| snap           | `snap list --color=never` → `awk 'NR>1 {print $1, $2}'`            |
| pip            | `pip list --outdated --format=columns` → package names             |
| rust           | `rustup toolchain list`                                            |
| ai-config      | `git -C <dir> rev-parse HEAD`                                      |
| dotfiles       | `git -C <dir> rev-parse HEAD`                                      |
| oh-my-zsh      | `git -C ~/.oh-my-zsh rev-parse HEAD`                               |
| tpm            | `git -C ~/.tmux/plugins/tpm rev-parse HEAD`                        |
| tfenv          | `git -C ~/.tfenv rev-parse HEAD`                                   |
| gems           | `gem list`                                                         |
| cheat.sh       | none (binary replace — record checksum or skip)                    |

If the pre-snapshot command fails (tool not installed), record a `Skip` immediately and don't run the step.

### Post-snapshot + diff to produce detail string

After running each step (exit code captured):

**Non-zero exit** → `Fail("exit N — see output above")`.

**Zero exit** → diff pre vs post to build the detail string:

| Step           | Detail string                                                                    |
| -------------- | -------------------------------------------------------------------------------- |
| brew           | `"3 formulae (vim, git, curl)"` / `"1 cask(s) (iterm2)"` / `"no changes"`        |
| softwareupdate | `"2 update(s) (macOS 15.5, Safari)"` / `"no changes"`                            |
| mas            | parse `==> Updated` lines from captured mas output                               |
| claude         | diff pre/post plugin versions → `"N plugin(s) updated"` / `"no changes"`         |
| npm            | diff pre/post `npm list -g` → `"N package(s) (name)"` / `"no changes"`           |
| apt            | diff pre/post `dpkg-query` → `"N package(s) (name, ...)"` / `"no changes"`       |
| snap           | diff pre/post `snap list` → `"N package(s) (name, ...)"` / `"no changes"`        |
| pip            | count packages listed in pre-snapshot (outdated before run) → `"N package(s)"`   |
| rust           | diff pre/post `rustup toolchain list` → `"1 toolchain updated"` / `"no changes"` |
| git repos      | `git log OLD..HEAD --oneline` → `"N commit(s)"` / `"no changes"`                 |
| gems           | diff pre/post `gem list` → `"N gem(s) (name, ...)"` / `"no changes"`             |
| cheat.sh       | `"updated"` (binary replacement, no reliable diff)                               |

### Section order (fixed)

```rust
const SECTION_ORDER: &[&str] = &[
    "brew", "softwareupdate", "mas", "claude", "npm",
    "apt", "snap", "pip", "rust",
    "ai-config", "dotfiles", "oh-my-zsh", "tpm", "tfenv",
    "gems", "cheat.sh",
];
```

### Output format

```
=== Update Summary — 2026-05-30 14:32:11 ===

[OK]   brew             3 formulae (vim, git, curl)
[OK]   softwareupdate   no changes
[OK]   mas              2 app(s) (Xcode, 1Password)
[OK]   claude           1 plugin(s) updated
[SKIP] npm              not installed
[OK]   apt              no changes
[SKIP] snap             not applicable
[OK]   pip              4 package(s)
[OK]   rust             no changes
[OK]   ai-config        3 commit(s)
[OK]   dotfiles         1 commit(s)
[OK]   oh-my-zsh        no changes
[SKIP] tpm              directory not found
[SKIP] tfenv            directory not found
[OK]   gems             no changes
[SKIP] cheat.sh         ~/bin/cht.sh not found

16 sections: 9 OK, 0 failed, 0 warnings, 7 skipped
Log appended: /Users/bruce/.etch-update.log
```

Format: `printf "[%-4s] %-16s %s\n", status, name, detail`

Section name column is 16 chars (pad right). Status prefix is always 4 chars: `OK  `, `FAIL`, `SKIP`, `WARN`.

### Log file

Append each run (separator + timestamp block + same table content) to `~/.etch-update.log`. Configurable via `etch.yaml`:

```yaml
update:
    log_path: "~/.etch-update.log" # optional, this is the default
```

`~` is expanded at runtime via `shellexpand` (already a dependency). File is created if absent. Write errors are non-fatal (warn to stderr, continue).

### `print_summary` function signature

```rust
fn print_summary(results: &[UpdateStepResult], log_path: &Path) -> anyhow::Result<()>
```

Iterates `SECTION_ORDER`, looks up each result by name (results may be sparse if some steps never ran due to platform gates), prints the table, then appends to the log file.

## Out of scope

- Brewfile drift check — future work
- AWS CLI update — `binary.url` action in a separate manifest
- Interactive pip virtualenv selection (just use the default python3)
- Windows / PowerShell path

## Implementation order

1. `Config` struct: add `update: UpdateConfig` + parse from etch.yaml
2. `Commands` enum: add `Update` variant + dispatch in `execute()`
3. `app/src/commands/update.rs`: skeleton + `StepStatus`/`UpdateStepResult` types + `any_flag_set()` logic
4. Steps (in order of safety): git-tools → brew → mas → claude → rust → packages → system → pip → gems → cheatsh
5. `print_summary()` + log append
6. Add `skip_if_not_exists` to `git.pull` (needed for optional tool pulls)
