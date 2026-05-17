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
│       ├── actions/          # 14 documented action types (see Action Catalog below)
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
│   ├── superpowers/   # Implementation plans
│   └── cursor/        # Cursor docs
├── Makefile       # lint, test, build, install-hooks
├── deny.toml      # cargo-deny config (license + advisory policy)
└── scripts/       # pre-commit, pre-push hooks
```

## Quick Reference

```bash
make lint          # cargo fmt --check + cargo clippy --all-targets -D warnings
make test          # lint, then cargo test
make build         # cargo build --release → target/release/etch (macOS aarch64)
make build-linux   # cargo zigbuild → target/x86_64-unknown-linux-gnu/release/etch + ~/Downloads/etch-linux
make install-hooks # install pre-commit and pre-push hooks (run once per checkout)
```

**Cross-compilation toolchain:** `cargo-zigbuild` + Zig (installed via `brew install zig` + `cargo install cargo-zigbuild`). Uses Zig's built-in C cross-compiler — no Docker required. `cross` (Docker-based) was attempted but has a known Apple Silicon bug in v0.2.5.

## Action Catalog

Manifest actions map to `lib/src/actions/<name>/`:

| Action               | Description                                     | Key fields                                                                                                                                                                                                                                           |
| -------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `command.run`        | Run shell commands                              | `command`, `args`, `privileged` (bool)                                                                                                                                                                                                               |
| `directory.create`   | Create a directory                              | `path`                                                                                                                                                                                                                                               |
| `directory.copy`     | Copy a directory                                | `from`, `to`                                                                                                                                                                                                                                         |
| `file.chmod`         | Set file/directory permissions                  | `path`, `mode` (string: `"700"`, `"0o700"`), `privileged` (bool)                                                                                                                                                                                     |
| `file.chown`         | Change file/directory ownership                 | `path`, `user`, `group`, `privileged` (bool)                                                                                                                                                                                                         |
| `file.copy`          | Copy a file; optionally render as Tera template | `from` (or `source`), `to` (or `target`), `template` (bool), `privileged` (bool)                                                                                                                                                                     |
| `file.link`          | Symlink a file                                  | `source`, `target` (`from`/`to` deprecated), `privileged` (bool)                                                                                                                                                                                     |
| `brew.bundle`        | Install packages from a Brewfile (macOS)        | `file` (path), `no_upgrade` (bool, default false), `cleanup` (bool, default false — removes packages not in Brewfile)                                                                                                                                |
| `brew.upgrade`       | Upgrade installed Homebrew formulae and casks   | `greedy` (bool, default false — also upgrades auto-update casks)                                                                                                                                                                                     |
| `brew.cleanup`       | Remove old Homebrew versions and cache          | `prune` (u32 days, optional — only remove versions older than N days; omit for brew default)                                                                                                                                                         |
| `mas.install`        | Install a Mac App Store app (macOS only)        | `name` (string, for readability), `id` (u64, App Store numeric ID). Requires `mas` CLI (`brew install mas`). Use `where: 'os.name == "macos"'` in manifests.                                                                                         |
| `mas.upgrade`        | Upgrade Mac App Store apps (macOS only)         | `id` (u64, optional — omit to upgrade all; requires `mas` CLI). Use `where: 'os.name == "macos"'`.                                                                                                                                                   |
| `git.clone`          | Clone a git repo                                | `repo_url`, `directory`                                                                                                                                                                                                                              |
| `package.install`    | Install OS packages                             | `name` (single) or `list` (multiple); `provider` (`apt`, `snap`, `brew`); `cask` (bool, Homebrew only — passes `--cask`). If a base package has `cask: true` but an OS variant exists without setting `cask`, the variant defaults to `cask: false`. |
| `package.repository` | Add a package repository or Homebrew tap        | `name` (repo URL or tap name e.g. `go-task/tap`), `provider` (`apt`, `brew`). For Homebrew: idempotent — re-tapping is fast so `has_repository()` always returns false.                                                                              |
| `macos.defaults`     | Write macOS defaults                            | domain, key, type, value fields                                                                                                                                                                                                                      |
| `binary`             | Install a binary from a GitHub release          | `name`, `version`, `url`                                                                                                                                                                                                                             |

Template engine is [Tera](https://keats.github.io/tera/). Available context variables: `user.username`, `user.home_dir`, `user.name`, `os.hostname`, `os.name`, `os.family`, `os.distribution`, `manifest_dir`.

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

## Config File

etch reads `etch.yaml` from the current directory or `~/.config/etch/etch.yaml`. Key fields:

```yaml
manifest_paths: [] # override manifest search paths
variables: {} # static key/value context variables
include_variables: [] # pull variables from DNS TXT or file
disable_update_check: false # suppress crates.io update check at startup
privilege: sudo # sudo | doas | run0
```

## Key Architectural Notes

- **DAG execution:** manifests can declare `depends:` on other manifests; petgraph resolves the topological sort before execution.
- **rhai scripting:** `Engine::new()` (fully open — no sandbox) evaluates `where:` conditions and variant expressions. Manifests from untrusted sources can run arbitrary rhai.
- **Binary downloads:** the `binary` action trusts GitHub TLS only — no checksum verification on downloaded binaries.
- **Privilege escalation:** declared per-action via `privileged: true`; provider defaults to `sudo`, configurable via `etch.yaml`.
- **update-informer:** checks crates.io at startup; disable via `disable_update_check: true` in config or `--no-color` flag has no effect on this.

## Testing

**Run tests:** `make test`

The test suite covers unit tests in `lib/src/` and integration tests in `app/tests/`. Current coverage is ~82% locally (macOS) and ~72% on Linux CI — the gap is macOS provider tests gated with `#[cfg(target_os = "macos")]` that don't run on ubuntu-latest. Practical ceiling is ~83% due to network operations, package managers, privilege escalation, and dead code that cannot be unit-tested.

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

**Coverage floor: 70%** (Linux CI gate; Linux measures ~72%, local macOS measures ~82% due to platform-specific tests).

## CI

Single workflow `.github/workflows/ci.yml`, triggers on `pull_request` to `main`/`master` only.

| Job           | What it does                                                                             |
| ------------- | ---------------------------------------------------------------------------------------- |
| `test`        | `make test` (fmt check + clippy + cargo test) + tarpaulin ≥70% (excluding jsonschemagen) |
| `secret-scan` | gitleaks v8.30.1 binary (advisory, non-blocking)                                         |
| `snyk-scan`   | Snyk code test (advisory, non-blocking)                                                  |
| `auto-merge`  | Squash-merges the PR when all jobs pass                                                  |

> **Note:** `build` job is temporarily disabled — restore when build times improve.

Pre-commit hook: `make lint` + `ggshield secret scan pre-commit`
Pre-push hook: `make test` (full suite before push reaches GitHub)

## Security Baseline (captured Phase 1)

- **cargo audit:** 3 unfixable advisories remain — hickory-proto ×2 (DNS DoS, no server surface), rsa (Marvin timing, not a signing oracle). All documented in `deny.toml`.
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
