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
│       ├── actions/          # 9 action types (see Action Catalog below)
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
make build         # cargo build --release → target/release/etch
make install-hooks # install pre-commit and pre-push hooks (run once per checkout)
```

## Action Catalog

Manifest actions map to `lib/src/actions/<name>/`:

| Action             | Description                                     | Key fields                                                  |
| ------------------ | ----------------------------------------------- | ----------------------------------------------------------- |
| `command.run`      | Run shell commands                              | `command`, `args`, `privileged` (bool)                      |
| `directory.create` | Create a directory                              | `path`                                                      |
| `directory.copy`   | Copy a directory                                | `from`, `to`                                                |
| `file.copy`        | Copy a file; optionally render as Tera template | `from` (or `source`), `to` (or `target`), `template` (bool) |
| `file.link`        | Symlink a file                                  | `source`, `target` (`from`/`to` deprecated)                 |
| `git.clone`        | Clone a git repo                                | `repo_url`, `directory`                                     |
| `package.install`  | Install OS packages                             | `name` (single) or `list` (multiple)                        |
| `macos.defaults`   | Write macOS defaults                            | domain, key, type, value fields                             |
| `binary`           | Install a binary from a GitHub release          | `name`, `version`, `url`                                    |

Template engine is [Tera](https://keats.github.io/tera/). Available context variables: `user.username`, `user.home_dir`, `user.name`, `os.hostname`, `os.name`, `os.family`, `os.distribution`, `manifest_dir`.

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

The test suite is inherited from comtrya and covers unit tests in `lib/src/` and integration tests in `app/tests/`. Current coverage is approximately 28% — the floor for the tarpaulin CI gate is set at 25% and should be raised as tests are added.

```bash
cargo test                     # all tests
cargo test -p etch-lib         # lib tests only
cargo test -p etch-cli         # integration tests only
cargo tarpaulin --fail-under 25  # coverage check (matches CI)
```

**Coverage floor: 25%** (current baseline for inherited codebase — raise incrementally with new tests).

## CI

Single workflow `.github/workflows/ci.yml`, triggers on `pull_request` to `main`/`master` only.

| Job           | What it does                                                                    |
| ------------- | ------------------------------------------------------------------------------- |
| `test`        | `make test` (fmt check + clippy + cargo test) + tarpaulin ≥25%                  |
| `build`       | `cargo build --release` → uploads `etch-linux-amd64` artifact (7-day retention) |
| `secret-scan` | gitleaks v8.30.1 binary (advisory, non-blocking)                                |
| `snyk-scan`   | Snyk code test (advisory, non-blocking)                                         |
| `auto-merge`  | Squash-merges the PR when all jobs pass                                         |

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

## Smoke Tests

`smoke-tests/` contains five manifests for validating etch on the Proxmox VM. Run in order — snapshot before `03-packages.yaml`. See `smoke-tests/README.md` for transfer instructions and run order.

## Phase Roadmap

- **Phase 1** (done): fork, rename, security audit, CI, hooks, smoke test manifests
- **Phase 2** (pending): migrate one shell script from dotfiles into a manifest; identify rough edges
- **Phase 3** (pending): prune unused package manager providers (keep Homebrew + one Linux distro)
- **Later:** ntfy notification action; macOS defaults ergonomics improvements
