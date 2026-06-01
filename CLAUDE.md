# CLAUDE.md — etch-cli

## Repository Overview

etch-cli is a personal fork of [comtrya](https://github.com/comtrya/comtrya) (archived April 2026, MIT). It is a declarative, manifest-driven configuration management tool for personal workstations — think "single-host Ansible" without the overhead. The binary is `etch`; the crate is `etch-cli`.

**Target machines:** Mac Studio M1 Ultra (macOS aarch64) and a Linux workstation (AMD Ryzen 9 7950X, RTX 4070 Ti Super, x86_64).

## Workspace Layout

```
etch-cli/
├── app/          # CLI binary crate (etch-cli) — entry point, clap commands, config loading
│   ├── src/
│   │   ├── main.rs           # Entrypoint: parse args, load config, build contexts, dispatch
│   │   ├── config/mod.rs     # GlobalArgs (clap), Commands enum, load_config()
│   │   └── commands/         # One file per subcommand
│   │       ├── mod.rs        # EtchCommand trait
│   │       ├── apply.rs      # etch apply — core manifest execution
│   │       ├── contexts.rs   # etch contexts
│   │       ├── gen_completions.rs
│   │       ├── plugin.rs
│   │       └── version.rs
│   └── tests/                # Integration tests (assert_cmd)
│       └── basic_usage.rs
├── lib/          # etch-lib — core engine (actions, atoms, contexts, manifests, steps)
│   └── src/
│       ├── actions/          # 38 action types (see Action Catalog below)
│       ├── atoms/            # Low-level OS operations (file, dir, command, http, plugin)
│       ├── config/mod.rs     # Config struct (manifest_paths, variables, privilege, etc.)
│       ├── contexts/         # Context providers: user, os, variables, rhai engine
│       ├── manifests/        # YAML/TOML parsing, DAG dependency resolution (petgraph)
│       ├── steps/            # Step execution with initializers and finalizers
│       ├── tera_functions/   # Custom Tera template functions (read_file_contents)
│       └── values/           # Context value type (string, list, map, bool, number)
├── jsonschemagen/ # Generates JSON schema for manifest editor autocomplete
├── smoke-tests/   # VM smoke test manifests (run on Proxmox VM, not in CI)
├── examples/      # Example manifests by action type
├── docs/          # mdbook documentation (inherited from comtrya, not built in CI)
│   ├── adr/           # Architectural Decision Records (repo-specific)
│   ├── knowledge/     # Reference material (architecture, domain docs, curated research)
│   ├── superpowers/   # Implementation plans
│   └── cursor/        # Cursor docs
├── Makefile       # lint, test, build, install-hooks
├── deny.toml      # cargo-deny config (license + advisory policy)
└── scripts/       # pre-commit, pre-push hooks
```

## Knowledge Directory

Reference material lives in `docs/knowledge/`. These documents capture architecture overviews, domain reference sheets, and curated research findings — things too detailed for CLAUDE.md but useful to look up. See `docs/knowledge/README.md` for what belongs there and what doesn't.

When web research (web-research skill) or context-mode fetches produce findings worth preserving, save them to `docs/knowledge/<topic>.md`.

## Quick Reference

```bash
make lint          # cargo fmt --check + cargo clippy --all-targets -D warnings
make test          # lint, then cargo test
make build         # cargo build --release → target/release/etch (macOS aarch64)
make build-linux   # cargo zigbuild → target/x86_64-unknown-linux-gnu/release/etch + ~/Downloads/etch-linux
make install-hooks # install pre-commit and pre-push hooks (run once per checkout)
```

**Cross-compilation toolchain:** `cargo-zigbuild` + Zig (installed via `brew install zig` + `cargo install cargo-zigbuild`). Uses Zig's built-in C cross-compiler — no Docker required. `cross` (Docker-based) was attempted but has a known Apple Silicon bug in v0.2.5.

**Release binary:** GitHub releases ship a single Linux x86_64 binary named `etch` (no platform suffix). There is no macOS binary in releases — macOS users must build from source (`cargo build --release`).

## Action Catalog

Manifest actions map to `lib/src/actions/<name>/`:

| Action               | Description                                     | Key fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| -------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `command.run`        | Run shell commands                              | `command`, `args`, `privileged` (bool), `skip_if_exists` (path — skip step if path exists)                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `directory.create`   | Create a directory                              | `path`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `directory.copy`     | Copy a directory                                | `from`, `to`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `directory.remove`   | Remove a directory                              | `target` (path to remove)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `file.chmod`         | Set file/directory permissions                  | `path`, `mode` (string: `"700"`, `"0o700"`), `privileged` (bool)                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `file.chown`         | Change file/directory ownership                 | `path`, `user`, `group`, `privileged` (bool)                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `file.flags`         | Set/clear BSD file flags (macOS only)           | `path`, `flags` (list of: `hidden`/`nohidden`/`uchg`/`nouchg`), `privileged` (bool)                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `file.copy`          | Copy a file; optionally render as Tera template | `from` (or `source`), `to` (or `target`), `template` (bool), `privileged` (bool). When `template: true`, uses Tera (Jinja2-compatible). Variables are namespaced: `{{ user.username }}`, `{{ variables.my_var }}`, `{{ env.HOME }}`, `{{ os.name }}`, `{{ manifest_dir }}`. **Gotcha:** bare `{{ my_var }}` silently renders empty — always prefix with the namespace.                                                                                                                                          |
| `file.link`          | Symlink a file                                  | `source`, `target` (`from`/`to` deprecated), `privileged` (bool), `glob` (glob pattern relative to `files/` dir — expands to one symlink per matched file, preserves subdirectory structure; mutually exclusive with `source`)                                                                                                                                                                                                                                                                                  |
| `file.download`      | Download a file from a URL                      | `from` (URL), `to` (destination path), `chmod` (u32 octal e.g. `755`, default `644`), `template` (bool, default false — render downloaded content as Tera template before writing), `owned_by_user` (Option string), `owned_by_group` (Option string), `privileged` (bool, default false — not supported: errors if true)                                                                                                                                                                                       |
| `file.remove`        | Remove a file                                   | `target` (path to remove), `privileged` (bool, default false — not supported: errors if true)                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `file.unarchive`     | Extract an archive                              | `from` (or `source` — archive path), `to` (or `target` — destination dir), `force` (Option bool, default true — overwrite existing), `privileged` (bool, default false — not supported: errors if true)                                                                                                                                                                                                                                                                                                         |
| `brew.bundle`        | Install packages from a Brewfile (macOS)        | `file` (path), `no_upgrade` (bool, default false), `cleanup` (bool, default false — removes packages not in Brewfile)                                                                                                                                                                                                                                                                                                                                                                                           |
| `brew.upgrade`       | Upgrade installed Homebrew formulae and casks   | `greedy` (bool, default false — also upgrades auto-update casks)                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `brew.cleanup`       | Remove old Homebrew versions and cache          | `prune` (u32 days, optional — only remove versions older than N days; omit for brew default)                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `mas.install`        | Install a Mac App Store app (macOS only)        | `name` (string, for readability), `id` (u64, App Store numeric ID). Requires `mas` CLI (`brew install mas`). Use `where: 'os.name == "macos"'` in manifests.                                                                                                                                                                                                                                                                                                                                                    |
| `mas.upgrade`        | Upgrade Mac App Store apps (macOS only)         | `id` (u64, optional — omit to upgrade all; requires `mas` CLI). Use `where: 'os.name == "macos"'`.                                                                                                                                                                                                                                                                                                                                                                                                              |
| `git.clone`          | Clone a git repo                                | `repo_url`, `directory`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `git.pull`           | Clone repo if missing, pull if present          | `repo_url`, `directory`, `skip_if_not_exists` (Option — skip entire action if this path is absent; useful for optional tool directories). Always executes (can't check network at plan time). Pull behavior (merge/rebase) respects existing git config.                                                                                                                                                                                                                                                        |
| `git.config`         | Set gitconfig values (alias `git.cfg`)          | `scope` (`global`/`local`/`system`, default `global`), `key` (Option), `value` (Option), `unset` (Option bool — unset a key; mutually exclusive with `value`), `settings` (Option IndexMap — bulk multi-key set, preserves insertion order), `directory` (required when `scope: local`). System scope auto-elevated via privilege infrastructure.                                                                                                                                                               |
| `package.install`    | Install OS packages                             | `name` (single) or `list` (multiple); `provider` (`apt`, `snap`, `brew`); `cask` (bool, Homebrew only — passes `--cask`). If a base package has `cask: true` but an OS variant exists without setting `cask`, the variant defaults to `cask: false`.                                                                                                                                                                                                                                                            |
| `package.autoremove` | Remove unused apt dependencies                  | No fields. Runs `apt autoremove --yes` with `DEBIAN_FRONTEND=noninteractive` and privilege escalation. Linux/apt only.                                                                                                                                                                                                                                                                                                                                                                                          |
| `npm.install`        | Install npm package(s) globally                 | `name` (single package) or `list` (multiple — mutually exclusive with `name`). Idempotent: checks `npm list -g --depth=0 <pkg>` per package; only installs missing ones. Requires `npm` on PATH.                                                                                                                                                                                                                                                                                                                |
| `package.repository` | Add a package repository or Homebrew tap        | `name` (repo URL or tap name e.g. `go-task/tap`), `provider` (`apt`, `brew`). For Homebrew: idempotent — re-tapping is fast so `has_repository()` always returns false.                                                                                                                                                                                                                                                                                                                                         |
| `macos.defaults`     | Write macOS defaults (write/array-add/delete)   | `domain`, `key`, `operation` (`write` default \| `array-add` \| `delete`), `kind` (required for write/array-add; must be one of: `string`, `integer`, `int`, `float`, `bool`, `boolean`, `date`, `data`, `array`, `dict`), `value` (required for write/array-add)                                                                                                                                                                                                                                               |
| `macos.service`      | Load or unload a macOS LaunchDaemon/LaunchAgent | `plist` (path to .plist file, tilde expanded), `state` (`loaded` \| `unloaded`), `label` (optional — service label for idempotency check; extracted via `defaults read` if omitted), `privileged` (bool, default `false` — required for system daemons in `/Library/LaunchDaemons/`)                                                                                                                                                                                                                            |
| `systemd.service`    | Enable/disable/start/stop a systemd unit        | `unit`, `enabled` (Option bool), `started` (Option bool), `privileged` (bool, default `false`). At least one of `enabled`/`started` required. Use `where: 'os.family == "linux"'`.                                                                                                                                                                                                                                                                                                                              |
| `binary.github`      | Install a binary from a GitHub release          | `name` (binary filename), `repository` (`"owner/repo"`), `directory` (destination path), `version` (Option string — pin to a release tag; omit for latest). Idempotent: skips if `{directory}/{name}` already exists. Auto-detects platform/arch from release asset names.                                                                                                                                                                                                                                      |
| `binary.url`         | Install a binary from an arbitrary URL          | `name`, `url`, `directory`, `version` (Option — injected as `{{ version }}` in URL template), `file` (Option — path inside archive to extract; required for non-raw archives), `sha256` (Option — expected hex digest), `privileged` (Option, ignored)                                                                                                                                                                                                                                                          |
| `ruby.install`       | Install a Ruby version via ruby-install         | `version` (string e.g. `"3.3.0"`), `implementation` (Option string — default `"ruby"`; also `"jruby"`, `"truffleruby"`), `rubies_dir` (Option string — default `~/.rubies`; passed as `--rubies-dir`). Idempotent: skips if `{rubies_dir}/{impl}-{version}` already exists. Requires `ruby-install` on PATH., `version_manager` (Option `"rbenv"` \| `"chruby"` — when `"rbenv"`, appends `rbenv global <version>` and `rbenv rehash` steps after installation; `"chruby"` accepted but no extra steps emitted) |
| `gem.install`        | Install Ruby gem(s)                             | `name` (single gem name) or `list` (multiple gem names — mutually exclusive with `name`; `list` takes priority if both set), `version` (Option string — version constraint, meaningful only with `name`). Idempotent: `gem list --installed <name>` checked at plan time; skips gems already installed. Requires `gem` on PATH (provided by the active Ruby).                                                                                                                                                   |
| `pip.install`        | Install Python package(s)                       | `name` (single package) or `list` (multiple packages — mutually exclusive with `name`; `list` takes priority if both set), `version` (Option string — exact version e.g. `"2.28.0"`, meaningful only with `name`; rendered as `name==version`), `virtualenv` (Option path — if set, uses `<virtualenv>/bin/pip` instead of `pip3`). Idempotent: `pip3 show <name>` checked at plan time. Requires `pip3` on PATH (or `<virtualenv>/bin/pip` when virtualenv set).                                               |
| `pyenv.install`      | Install a Python version via pyenv              | `version` (string e.g. `"3.12.0"` — required), `configure_opts` (Option string — value for `PYTHON_CONFIGURE_OPTS` env var before invoking `pyenv install`; on macOS with Homebrew use `"--with-system-libmpdec=no"` to avoid libmpdec conflict). Idempotent: skips if version already listed in `pyenv versions --bare`. Requires `pyenv` on PATH.                                                                                                                                                             |
| `pyenv.virtualenv`   | Create a pyenv virtualenv                       | `python_version` (string e.g. `"3.12.0"` — required), `name` (string — required; the virtualenv name). Idempotent: skips if name already listed in `pyenv virtualenvs --bare`. Requires pyenv-virtualenv plugin on PATH.                                                                                                                                                                                                                                                                                        |
| `group.add`          | Create a system group                           | `group_name` (string). Idempotent: skips if group already exists. Linux: `groupadd`. macOS: `dscl`.                                                                                                                                                                                                                                                                                                                                                                                                             |
| `user.add`           | Create a system user                            | `username`, `fullname`, `home_dir`, `shell` (path e.g. `/bin/bash`), `group` (Vec string — groups to add the user to), `provider` (auto-detected). Idempotent: skips if username already exists. Linux: `useradd`. macOS: `dscl`.                                                                                                                                                                                                                                                                               |
| `user.group`         | Add an existing user to groups                  | `username`, `group` (Vec string — groups to add to), `provider` (auto-detected). Use when the user already exists and you only need to modify group membership.                                                                                                                                                                                                                                                                                                                                                 |

Template engine is [Tera](https://keats.github.io/tera/). Available context variables: `user.username`, `user.home_dir`, `user.name`, `os.hostname`, `os.name`, `os.family`, `os.distribution`, `manifest_dir`.

## Adding a New Action

Every new action requires changes in exactly these places:

1. **Create `lib/src/actions/<name>/install.rs`** — the action struct + `impl Action`
2. **Create `lib/src/actions/<name>/mod.rs`** — re-export: `mod install; pub use install::ActionType;`
3. **Register in `lib/src/actions/mod.rs`** (6 edits):
    - `mod <name>;` — module declaration
    - `use <name>::ActionType;` — import
    - Enum variant with serde rename: `ActionType(ConditionalVariantAction<ActionType>)` + `#[serde(rename = "name.action")]`
    - Match arm in `inner_ref()` impl
    - Match arm in `notify` accessor
    - Match arm in `Deref` impl
    - Match arm in `Display` impl (`=> "name.action"`)
4. **Update the two test YAML lists** in `all_major_action_variants_can_be_deserialized` and `all_action_variants_inner_ref_and_deref` — add a YAML entry for the new action to each
5. **Add `examples/<name>/<name>-install.yaml`** with one entry per option combination
6. **Update the Action Catalog table** in this file and in `README.md`

**Auditing action names:** YAML action names come from `#[serde(rename = "...")]` in `lib/src/actions/mod.rs` — not from Rust struct names. When verifying that docs match implementation, always grep that file for the rename annotations; struct names and YAML names diverge (e.g. struct `GroupAdd` → YAML `group.add`).

Missing any step produces a compile error (missing match arm) or test failure (incorrect variant count). The `semver-check` CI job will always produce an advisory failure (`enum_variant_added`) — this is expected and non-blocking.

## Homebrew macOS Workflow

All four Homebrew install mechanisms are supported. **Recommended approach for dotfiles migration: use `brew.bundle` with the existing Brewfile** — it handles taps, formulae, casks, and MAS apps in one action.

```yaml
# All-in-one: delegates to the Brewfile (covers taps + formulae + casks + MAS)
- action: brew.bundle
  file: "{{ user.home_dir }}/git-repos/personal/dotfiles/Brewfile"
```

Piece-by-piece alternative (when you need per-app `where:` conditions):

```yaml
# 1. Add a custom tap first — required before installing formulae from that tap
- action: package.repository
  name: go-task/tap
  provider: homebrew

# 2. Install a formula from the tap (or any formula)
- action: package.install
  name: go-task/tap/go-task
  provider: homebrew

# 3. Install a cask (GUI app)
- action: package.install
  name: alfred
  provider: homebrew
  cask: true

# 4. Install a Mac App Store app (requires `mas` CLI: brew install mas)
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
  where: 'os.name == "macos"'
```

**Key gotchas:**

- `mas.install` requires the `mas` CLI to be installed first (`brew install mas`). Always pair with `where: 'os.name == "macos"'` since `mas` is macOS-only.
- `brew.bundle cleanup: true` removes all packages NOT listed in the Brewfile — destructive, use carefully.
- `package.repository` for Homebrew taps is always idempotent (re-tapping is fast; etch always runs `brew tap` rather than checking first).
- `package.install cask: true` is Homebrew-only; other providers silently ignore the field. If a base package has `cask: true` but an OS variant exists without explicitly setting `cask:`, the variant defaults to `cask: false`.

## Machine Profiles

etch-cli does not have built-in profile concepts — use the `variables:` section of `etch.yaml` to define a machine's profile and capabilities. Manifests use `where:` conditions to apply actions selectively.

**Convention:** define `profile` (a human-readable name) and one `has_<capability>: true` boolean per capability your machine supports.

```yaml
# Mac Studio — ~/.config/etch/etch.yaml
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests

variables:
    profile: "mac_workstation"
    has_gui: true
    has_devtools: true
    has_k8s: true
    has_docker: true
    has_rust: true
    has_printing: true
```

```yaml
# Linux workstation — ~/.config/etch/etch.yaml
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests

variables:
    profile: "linux_workstation"
    has_gui: true
    has_devtools: true
    has_k8s: true
    has_docker: true
    has_rust: true
    has_snap: true
```

**Manifest usage:**

```yaml
# Entire manifest skips on machines without k8s capability
where: "variables.has_k8s"

actions:
    - action: package.install
      list: [kubectl, helm, k9s]
      provider: homebrew
```

```yaml
# Per-action capability guard
actions:
    - action: package.install
      name: gh
      provider: homebrew
      where: "variables.has_devtools"
```

**Capability naming convention:**

| Variable       | Meaning                                         |
| -------------- | ----------------------------------------------- |
| `has_gui`      | Machine runs a graphical desktop                |
| `has_devtools` | Install developer tools (gh, jq, etc.)          |
| `has_k8s`      | Install Kubernetes tooling (kubectl, helm, k9s) |
| `has_docker`   | Install Docker and container tools              |
| `has_rust`     | Install Rust toolchain                          |
| `has_printing` | Install printer drivers                         |
| `has_snap`     | Use snap package manager (Linux only)           |

New capabilities can be added freely — the convention is the only constraint. See `examples/machine-profiles/` for complete example files.

## Config File

etch reads `etch.yaml` from the current directory or `~/.config/etch/etch.yaml`. Key fields:

```yaml
manifest_paths: [] # override manifest search paths
variables: {} # static key/value context variables
include_variables: [] # pull variables from DNS TXT or file
disable_update_check: false # suppress crates.io update check at startup
privilege: sudo # sudo | doas | run0
```

## Committing Work

Invoke `caveman:caveman-commit` skill to generate the commit message before running `git commit`. Full format and rules in `~/.claude/CLAUDE.md`.

## Key Architectural Notes

- **DAG execution:** manifests can declare `depends:` on other manifests; petgraph resolves the topological sort before execution.
- **rhai scripting:** `Engine::new()` (fully open — no sandbox) evaluates `where:` conditions and variant expressions. Manifests from untrusted sources can run arbitrary rhai.
- **Binary downloads:** the `binary` action trusts GitHub TLS only — no checksum verification on downloaded binaries.
- **Privilege escalation:** declared per-action via `privileged: true`; provider defaults to `sudo`, configurable via `etch.yaml`.
- **update-informer:** checks crates.io at startup; disable via `disable_update_check: true` in config or `--no-color` flag has no effect on this.

## Testing

**Run tests:** `make test`

The test suite covers unit tests in `lib/src/` and integration tests in `app/tests/`. Current coverage is ~82% locally (macOS) and ~75% on Linux CI — the gap is macOS provider tests gated with `#[cfg(target_os = "macos")]` that don't run on ubuntu-latest. Practical ceiling is ~82% due to network operations, package managers, privilege escalation, and dead code that cannot be unit-tested.

`app/tests/integration.rs` — 11 end-to-end tests spawning the real `etch` binary. Covers the core `etch apply` path for `file.link`, `file.copy`, `command.run`, `directory.create` (happy path + idempotency each), and `file.flags` (macOS only: set hidden, idempotent, clear hidden). These do not contribute to tarpaulin coverage (subprocess invocation) but verify behavioral correctness.

`app/tests/snapshots.rs` — 5 snapshot tests using `insta` that lock the exact stdout format of `etch -h`, `etch apply --help`, `etch version`, `etch apply --dry-run`, and `etch apply -v --dry-run`. Version strings and tmpdir paths are scrubbed with filters. Any accidental format change fails CI. To update snapshots intentionally: run `INSTA_UPDATE=new cargo test --test snapshots`, then `cargo insta accept`, commit updated `.snap` files.

`app/tests/cli_commands.rs` — 7 tests exercising CLI commands end-to-end: `version`, `gen-completions` (bash/zsh/fish), `contexts` (exits + contains "os"), and `plugin` (fails without subcommand). Uses `assert_cmd`.

Coverage ceiling is approximately 83% due to hard-to-cover code:

- Network operations (GitHub API, git clone, DNS lookups)
- CLI binary dispatch (`app/src/commands/apply.rs`, etc. — only coverable via binary test harnesses)
- Privilege-escalation atoms (`sudo`/`root` required)
- Package manager operations (requires `apt`/`brew` to be installed and functional)

```bash
cargo test                                                          # all tests
cargo test -p etch-lib                                              # lib tests only
cargo test -p etch-cli                                              # integration tests only
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 70  # coverage check (matches CI)
```

**Coverage floor: 70%** (Linux CI gate; Linux measures ~75%, local macOS measures ~82% due to platform-specific tests).

**Exception to the global tdd.md ≥90% standard:** The global standard (`~/.claude/standards/tdd.md`) requires ≥90% line coverage. This repo operates at 70% CI gate due to structurally uncoverable code (network ops, package manager calls, privilege escalation, CLI binary dispatch). This is a documented exception — not a gap. Do not attempt to raise the gate above 76% (Linux ceiling) without verifying actual CI output first.

## CI

Single workflow `.github/workflows/ci.yml`, triggers on `pull_request` to `main`/`master` only.

| Job            | What it does                                                                                                   |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| `test`         | `make test` (fmt check + clippy + cargo test) + tarpaulin ≥70% (excluding jsonschemagen)                       |
| `cargo-audit`  | `cargo audit` — advisory scan (non-blocking)                                                                   |
| `secret-scan`  | gitleaks v8.30.1 binary (advisory, non-blocking)                                                               |
| `snyk-scan`    | Snyk code test (advisory, non-blocking)                                                                        |
| `docs-lint`    | Lints mdbook docs                                                                                              |
| `docs-build`   | Builds mdbook docs                                                                                             |
| `semver-check` | `cargo semver-checks` vs `origin/main` baseline (advisory, `continue-on-error: true`, not in auto-merge needs) |
| `auto-merge`   | Squash-merges the PR when all required jobs pass                                                               |

> **Note:** `build` job is temporarily disabled — restore when build times improve.

Pre-commit hook: `make lint` + `ggshield secret scan pre-commit`
Pre-push hook: `make test` (full suite before push reaches GitHub)

## cargo semver-checks

`make semver` runs `cargo semver-checks check-release -p etch-lib --baseline-rev origin/main`. Always use `--baseline-rev` — `etch-lib` is not published to crates.io, so the tool cannot auto-detect a registry baseline and will fail without it.

The release workflow (`release.yml`) checks against the previous git tag. The tag `v${VERSION}` is created **after** the semver check step (near the end of the workflow), so `git tag --sort=-version:refname | grep "^v" | head -1` correctly returns the previous release tag at check time — not the one being released. Do not change `head -1` to `sed -n '2p'`; that would skip a valid previous tag on future runs.

Adding a new variant to the `Actions` enum always triggers an `enum_variant_added` advisory failure on the `semver-check` CI job. This is expected — every new action adds a public enum variant, which is a semver-breaking change by the spec. The semver-check job is `continue-on-error: true` and is not in the auto-merge `needs:` list, so it never blocks the PR.

## Security Baseline (captured Phase 1)

- **cargo audit:** 3 unfixable advisories remain — hickory-proto ×2 (DNS DoS, no server surface), rsa (Marvin timing, not a signing oracle). Ignored via `--ignore` flags in `.github/workflows/cargo-audit-scheduled.yml`. **`cargo audit` does NOT read `deny.toml`** — that file is for `cargo deny` only. New advisories must be triaged in both places independently.
- **Dependency drift:** ran `cargo update` post-fork, resolving 13 of 16 original advisories.

## Branch Workflow

Never commit directly to `main`. All changes go through a feature branch and PR. The auto-merge job merges on CI pass.

```bash
git checkout -b feat/my-change
# ... work ...
git push -u origin feat/my-change
gh pr create --repo brujack/etch-cli
```

**`gh` commands always need `--repo brujack/etch-cli`** — the `upstream` remote points to `comtrya/comtrya` (archived), and `gh` resolves its default repo from the first matching remote, picking the archived upstream instead of `origin`. Without the flag, `gh pr create`, `gh pr view`, `gh pr checks`, etc. silently target the wrong repo.

## Smoke Tests

`smoke-tests/` contains five manifests for validating etch on the Proxmox VM. Run in order — snapshot before `03-packages.yaml`. See `smoke-tests/README.md` for transfer instructions and run order.

## Phase Roadmap

- **Phase 1** (done): fork, rename, security audit, CI, hooks, smoke test manifests
- **Phase 2** (pending): migrate one shell script from dotfiles into a manifest; identify rough edges
- **Phase 3** (done): pare down to Ubuntu 24.04/26.04 and macOS only; removed 11 provider files
- **Later:** ntfy notification action; macOS defaults ergonomics improvements

## Definition of Done

A PR or direct master commit is complete when **all** of the following are true:

- [ ] `make test` passes (`cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`)
- [ ] Coverage ≥70% on Linux CI — verify from CI output, not local macOS measurement
- [ ] `gh pr checks --repo brujack/etch-cli <number> --watch` passes (or commit is docs-only)
- [ ] `pr-review` skill PASS verdict obtained before push
- [ ] Plan index updated (`docs/cursor/README.md`) if this PR implements a tracked spec
- [ ] Action catalog updated in `README.md` if a new action was added
- [ ] `examples/<action>/` updated when a new action or field variant is added — at minimum one `.yaml` per option combination, with inline comments on every field
- [ ] Learning analysis complete (session-end or post-merge)
