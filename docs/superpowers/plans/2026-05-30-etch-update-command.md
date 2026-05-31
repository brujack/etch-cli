# etch update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `etch update` as a built-in subcommand that replicates `dotfiles/setup_env.sh run_update()` — brew, mas, softwareupdate, claude plugins, npm globals, apt, snap, pip, rustup, git-based tools (oh-my-zsh, tpm, tfenv, ai-config, dotfiles), gems, and cheat.sh.

**Spec:** [etch-update-workflow-design](../specs/2026-05-30-etch-update-workflow-design.md)

**Architecture:** `app/src/commands/update.rs` holds all step logic. Config structs live in `lib/src/config/mod.rs` under a new `update: UpdateConfig` field. Platform gating is runtime (`std::env::consts::OS`). Steps collect `UpdateStepResult` values; `print_summary` renders a fixed-order table to stdout and optionally appends to a log file.

**Tech Stack:** Rust, clap (derive), `std::process::Command`, `std::io::Write`, anyhow.

---

### Task 1: Config structs in etch-lib

**Files:**

- Modify: `lib/src/config/mod.rs`

Add `UpdateConfig`, `GitToolsConfig`, and `ClaudeUpdateConfig` structs plus `pub update: UpdateConfig` field to `Config`.

- [x] Add `UpdateConfig { git_tools, claude, log_path }` with `Default`
- [x] Add `GitToolsConfig { ai_config, dotfiles, oh_my_zsh, tpm, tfenv }` with `Default`
- [x] Add `ClaudeUpdateConfig { plugins: Vec<String>, npm_globals: Vec<String> }` with `Default`
- [x] Add `pub update: UpdateConfig` field to `Config` with `#[serde(default)]`
- [x] Test: `update_config_default_has_none_fields`
- [x] Test: `claude_update_config_default_has_empty_vecs`
- [x] Test: `update_config_deserialize_full_yaml`
- [x] Test: `update_config_absent_defaults_to_empty`
- [x] Test: `git_tools_config_all_fields_optional`

---

### Task 2: `Update` command skeleton + helpers

**Files:**

- Create: `app/src/commands/update.rs`

Core types and helpers:

- [x] `SECTION_ORDER: &[&str]` — fixed 16-entry order: `brew`, `softwareupdate`, `mas`, `claude`, `npm`, `apt`, `snap`, `pip`, `rust`, `ai-config`, `dotfiles`, `oh-my-zsh`, `tpm`, `tfenv`, `gems`, `cheat.sh`
- [x] `StepStatus` enum: `Ok(String)`, `Fail(String)`, `Skip(String)`, `Warn(String)` — `.tag()` returns `[OK]  `, `[FAIL]`, `[SKIP]`, `[WARN]`; `.detail()` returns inner string
- [x] `UpdateStepResult { name: &'static str, status: StepStatus }`
- [x] `Update` struct: 10 `bool` flags (`brew`, `system`, `mas`, `claude`, `packages`, `pip`, `rust`, `git_tools`, `gems`, `cheatsh`) + `#[derive(Parser)]`
- [x] `Update::any_flag_set() -> bool`
- [x] Helpers: `step_should_run`, `run_cmd`, `capture`, `diff_lines`, `git_commit_count`, `home_dir`, `expand_tilde`, `has_cmd`, `skip_result`, `fail_result`
- [x] Test: `any_flag_set_false_when_no_flags`
- [x] Test: `any_flag_set_true_when_one_flag`
- [x] Test: `step_should_run_flag_true`
- [x] Test: `step_should_run_run_all`
- [x] Test: `step_should_run_false_when_no_flag_no_run_all`
- [x] Test: `diff_lines_returns_new_lines`
- [x] Test: `diff_lines_empty_when_unchanged`
- [x] Test: `expand_tilde_replaces_prefix`
- [x] Test: `expand_tilde_no_tilde`

---

### Task 3: `print_summary`

**Files:**

- Modify: `app/src/commands/update.rs`

- [x] `print_summary<W: Write>(w: &mut W, results: &[UpdateStepResult], log_path: Option<&Path>) -> anyhow::Result<()>`
- [x] Formats: `  [TAG] name   detail` aligned with fixed-width name column
- [x] Appends to log file (creates if absent); log includes timestamp header + same lines
- [x] Count line: `N sections: X ok, Y failed, Z warnings, W skipped`
- [x] Test: `print_summary_formats_ok`
- [x] Test: `print_summary_counts`
- [x] Test: `print_summary_section_order`
- [x] Test: `print_summary_omits_missing_sections`
- [x] Test: `print_summary_appends_to_log_file`

---

### Task 4: Step implementations

**Files:**

- Modify: `app/src/commands/update.rs`

- [x] `update_brew` — pre: `brew list`, run `brew upgrade [--greedy]` + `brew cleanup`, diff → "N formulae"
- [x] `update_softwareupdate` — macOS only; run `softwareupdate -ia --verbose`
- [x] `update_mas` — macOS only; `has_cmd("mas")`; run `mas upgrade`
- [x] `update_claude` — iterate `claude.plugins`, run `claude plugins install <p>` per plugin; iterate `npm_globals`, run `npm install -g <p>`
- [x] `update_npm` — run `npm update -g`
- [x] `update_apt` — Linux only; run `apt-get update -y` + `apt-get upgrade -y` + `apt-get autoremove -y`
- [x] `update_snap` — Linux only + `has_snap`; run `snap refresh`
- [x] `update_pip` — `has_cmd("pip3")` or `has_cmd("pip")`; run `pip install --upgrade pip`
- [x] `update_rust` — `has_cmd("rustup")`; pre: `rustup show`, run `rustup update`, diff → "N toolchains updated"
- [x] `update_git_repo(name, dir)` — captures pre HEAD, runs `git -C dir pull`, diff → commit count
- [x] `update_gems` — `has_cmd("gem")`; run `gem update`
- [x] `update_cheatsh` — `~/bin/cht.sh` exists; run curl + chmod

---

### Task 5: Wire into app

**Files:**

- Modify: `app/src/commands/mod.rs`
- Modify: `app/src/config/mod.rs`
- Modify: `app/src/main.rs`

- [ ] `mod update; pub(crate) use update::Update;` in commands/mod.rs
- [ ] Add `Update(commands::Update)` variant to `Commands` enum
- [ ] Add dispatch arm `Commands::Update(cmd) => cmd.execute(&runtime)` in main.rs
- [ ] Verify `etch update --help` renders correctly

---

### Task 6: `skip_if_not_exists` on `git.pull`

**Files:**

- Modify: `lib/src/actions/git/pull.rs`

- [ ] Add `pub skip_if_not_exists: Option<String>` field to `GitPull`
- [ ] In `plan()`: if `skip_if_not_exists` path is set and does not exist → return `[Step::skip()]`
- [ ] Test: skip when path absent
- [ ] Test: run when path present

---

### Task 7: Compile, test, snapshot update

- [ ] `make test` passes
- [ ] Update `app/tests/snapshots.rs` if `etch --help` or `etch update --help` snapshot changes
- [ ] Run `INSTA_UPDATE=new cargo test --test snapshots && cargo insta accept` if needed
- [ ] Coverage ≥70% on Linux CI

---

### Task 8: PR review + push

- [ ] `pr-review` skill PASS verdict
- [ ] Push + `gh pr create --repo brujack/etch-cli`
- [ ] Monitor CI with `gh pr checks --repo brujack/etch-cli <N> --watch`
- [ ] Post-merge cleanup (worktree remove, branch delete, main sync)
- [ ] Update `docs/superpowers/README.md` status → Done on main
- [ ] Run `learnings` skill
