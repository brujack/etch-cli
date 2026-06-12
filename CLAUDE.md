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
│       ├── integration.rs    # 20 e2e tests: file.link, file.copy, command.run, directory.create, file.flags, state+history
│       ├── snapshots.rs      # 5 snapshot tests locking etch -h, etch apply --help, etch version, --dry-run output
│       ├── cli_commands.rs   # 11 tests: version, gen-completions, contexts, plugin, help-all
│       ├── status.rs         # 7 tests: etch status exit codes, --json, --missing-only, stdout structure
│       ├── error_paths.rs    # error path integration tests
│       └── basic_usage.rs
├── lib/          # etch-lib — core engine (actions, atoms, contexts, manifests, steps)
│   └── src/
│       ├── actions/          # 50 action types (see Action Catalog below)
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

50 actions — full field reference in [`docs/knowledge/action-catalog.md`](docs/knowledge/action-catalog.md).

Actions map to `lib/src/actions/<name>/`. YAML names come from `#[serde(rename = "...")]` — not Rust struct names (e.g. struct `GroupAdd` → YAML `group.add`).

Template engine is [Tera](https://keats.github.io/tera/). Context variables: `user.username`, `user.home_dir`, `user.name`, `os.hostname`, `os.name`, `os.family`, `os.arch`, `os.distribution`, `manifest_dir`. **Gotcha:** bare `{{ my_var }}` silently renders empty — always namespace it (e.g. `{{ variables.my_var }}`).

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
4. **Update the three test YAML lists** in `all_major_action_variants_can_be_deserialized`, `all_action_variants_inner_ref_and_deref`, and `all_action_variants_display` — add a YAML entry for the new action to each, update the action count in the `assert_eq!`, and add a `names.contains` assertion in the display test
5. **Add `examples/<name>/<name>-install.yaml`** with one entry per option combination
6. **Update the Action Catalog table** in this file and in `README.md`

**Auditing action names:** YAML action names come from `#[serde(rename = "...")]` in `lib/src/actions/mod.rs` — not from Rust struct names. When verifying that docs match implementation, always grep that file for the rename annotations; struct names and YAML names diverge (e.g. struct `GroupAdd` → YAML `group.add`).

Missing any step produces a compile error (missing match arm) or test failure (incorrect variant count). The `semver-check` CI job will always produce an advisory failure (`enum_variant_added`) — this is expected and non-blocking.

**TDD stub behavior:** `all_action_variants_inner_ref_and_deref` calls `inner.summarize()` on every registered variant in a loop. If the new action's `summarize()` is `todo!()`, this dispatch test also panics — expect N+1 failures (N unit tests + 1 dispatch test) in the RED phase, not just N. All are correct TDD RED state.

**Editing match arms: use `replace_all: true` for identical `=> a` patterns.** The `inner_ref`, `notify`, and `Deref` match blocks all contain arms like `Actions::MacOSDefault(a) => a,` — identical structure across blocks. When adding a new arm to each block, the Edit tool will refuse with "Found 2 matches" if you target a common pattern. Fix: use `replace_all: true` when both identical blocks need the same addition, or use enough unique surrounding context (the action above/below the insertion point) to disambiguate.

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

## State Manifest

After each successful `etch apply`, etch writes `~/.local/share/etch/state.yaml` recording every atom executed: manifest name, action type, canonical key, applied-at timestamp, sha256 (file atoms only, currently always `null`), and whether the atom produced a change.

**`etch history`** reads the state file:

```
etch history                       # table of all recorded atoms
etch history --manifest <substr>   # filter by manifest name substring
etch history --json                # NDJSON, one object per atom
```

**State path override:** set `ETCH_STATE_DIR` env var to redirect state to a different directory (used by integration tests; `state.yaml` is always the filename within that dir).

**Implementation:** `lib/src/state/` — `StateStore::record()` merges on `(manifest, action, key)` triple so re-running the same action updates the row rather than appending.

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
- **lib.rs module order:** `pub mod` declarations in `lib/src/lib.rs` must be alphabetical — rustfmt enforces this. Out-of-order additions cause pre-commit failure.
- **etch doctor security pattern:** binary names from manifest YAML are passed to `Command::new(binary_name)` (no shell), not `sh -c "binary_name --version"`, to prevent injection from hostile `name:` fields. Only explicit `doctor.versions.command` uses `sh -c` (user-authored in etch.yaml).

## Testing

**Run tests:** `make test`

Unit tests in `lib/src/`, integration tests in `app/tests/` (assert_cmd + insta snapshots). Coverage ~86% macOS / ~81.48% Linux CI — gap is macOS-only tests gated with `#[cfg(target_os = "macos")]`.

To update insta snapshots: `INSTA_UPDATE=new cargo test --test snapshots`, then `cargo insta accept`.

**Adding a new subcommand changes `etch --help` output** — this breaks the `help` snapshot test. Always run `INSTA_UPDATE=new cargo test --test snapshots && cargo insta accept` after registering any new subcommand in `Commands`.

```bash
cargo test                                                          # all tests
cargo test -p etch-lib                                              # lib only
cargo test -p etch-cli                                              # integration only
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 81  # coverage (matches CI)
```

**Coverage floor: 81%** (Linux CI gate). **Exception to global ≥90% standard** — structurally uncoverable code: network ops, package managers, privilege escalation, CLI binary dispatch. Do not raise the gate above 81% without verifying actual CI output.

## CI

Single workflow `.github/workflows/ci.yml`, triggers on `pull_request` to `main`/`master` only.

| Job            | What it does                                                                                                   |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| `test`         | `make test` (fmt check + clippy + cargo test) + tarpaulin ≥81% (excluding jsonschemagen)                       |
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

The universal DoD in `behavior.md` applies. etch-cli adds:

- [ ] Coverage ≥81% on Linux CI — verify from CI output, not local macOS measurement (exception to global ≥90% — structurally uncoverable code)
- [ ] Plan index updated (`docs/cursor/README.md`) if this PR implements a tracked spec
- [ ] Action catalog updated in `README.md` if a new action was added
- [ ] `examples/<action>/` updated when a new action or field variant is added — at minimum one `.yaml` per option combination, with inline comments on every field
