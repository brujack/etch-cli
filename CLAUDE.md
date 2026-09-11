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
│       ├── integration.rs    # e2e tests: file.link, file.copy, command.run, directory.create, file.flags, state+history
│       ├── snapshots.rs      # snapshot tests locking etch -h, etch apply --help, etch version, --dry-run output
│       ├── cli_commands.rs   # version, gen-completions, contexts, plugin, help-all
│       ├── status.rs         # etch status exit codes, --json, --missing-only, stdout structure
│       ├── rollback.rs       # etch rollback stash/prune/restore integration
│       ├── error_paths.rs    # error path integration tests
│       └── basic_usage.rs
├── lib/          # etch-lib — core engine (actions, atoms, contexts, manifests, steps)
│   └── src/
│       ├── actions/          # One directory per action type (see Action Catalog below)
│       ├── atoms/            # Low-level OS operations (file, dir, command, http, plugin)
│       ├── config/mod.rs     # Config struct (manifest_paths, variables, privilege, etc.)
│       ├── contexts/         # Context providers: user, os, variables, rhai engine
│       ├── manifests/        # YAML/TOML parsing, DAG dependency resolution (petgraph)
│       ├── state/            # State manifest behind etch history
│       ├── steps/            # Step execution with initializers and finalizers
│       ├── tera_functions/   # Custom Tera template functions (read_file_contents)
│       └── values/           # Context value type (string, list, map, bool, number)
├── jsonschemagen/ # Generates JSON schema for manifest editor autocomplete
├── smoke-tests/   # VM smoke test manifests (run on Proxmox VM, not in CI)
├── examples/      # Example manifests by action type
├── docs/          # mdbook documentation (inherited from comtrya, not built in CI)
│   ├── adr/           # Architectural Decision Records (repo-specific)
│   ├── knowledge/     # Pointer stub and retrospectives — knowledge lives in ai-config
│   ├── superpowers/   # Implementation plans
│   └── cursor/        # Cursor docs
├── Makefile       # lint, test, build, install-hooks
├── deny.toml      # cargo-deny config (license + advisory policy)
└── scripts/       # pre-commit, pre-push hooks
```

## 10-80-10 Execution Cycle

Sessions in this repo follow the 10-80-10 execution cycle defined in `ai-config` ADR-0009 (with the ADR-0010 wave-dispatch extension):

- **Phase 1 (10%) — Architect.** `brainstorming` → `writing-plans` (emit per-task YAML `yaml-task` blocks with `role`/`model`/`tdd`/`acceptance`/`max_retries`/`files_touched`/`depends_on`/`parallel_group`). Opus role.
- **Phase 2 (80%) — Execute.** `subagent-driven-development` runs iterate-until-green per task; FORBIDDEN list prevents gate cheating; wave-dispatch when `parallel_group` is declared. Sonnet/Haiku per task per the plan.
- **Phase 3 (10%) — Review.** `finishing-a-development-branch` chains `pr-review` → `security-review` → `bug-scan` → `docs` → `learnings` → finish. Opus role.

Validate a plan before dispatch:

```bash
make validate-plan PLAN=docs/superpowers/plans/<file>.md
```

The validator (`~/.claude/scripts/validate-plan.py`, shared from ai-config) enforces required fields, valid role/model/tdd values, haiku scope guard, and disjoint `files_touched` within each `parallel_group`.

## Knowledge Directory

Knowledge for this repo lives in `~/git-repos/personal/ai-config/docs/knowledge/etch-cli-<topic>.md` (ai-config ADR-0020). This repo's `docs/knowledge/README.md` is a pointer stub. Incident write-ups and tool reference go there; dated readings go in the commit or PR body; none of it goes in this file, except a gate or safety rule (ai-config ADR-0077 rule 5).

Read the matching file before the work it covers (paths relative to `~/git-repos/personal/ai-config/docs/knowledge/`):

- Before writing a manifest, or editing the `brew.bundle`, `package.install`, `package.repository` or `mas.install` actions, the config loader, or `lib/src/state/`: `etch-cli-manifest-authoring.md` (Homebrew/MAS workflow, machine-profile variables, `etch.yaml` keys, state manifest).
- Before adding, renaming or documenting an action: `etch-cli-action-catalog.md`.
- Before changing the Python coverage floor, the ruff scope, the `requirements-ci-test.txt` install, the `auto-merge` `needs:` list, `scripts/pre-push`, or `.claude/scripts/triage_log.py`: `etch-cli-ci-python-tooling.md`.
- Before triaging a `cargo audit` or `cargo deny` advisory: `etch-cli-cargo-deny-vs-audit.md`.

## Quick Reference

```bash
make lint          # cargo fmt --check + cargo clippy --all-targets -D warnings
make test          # lint, then cargo test
make build         # cargo build --release → target/release/etch (macOS aarch64)
make build-linux   # cargo zigbuild → target/x86_64-unknown-linux-gnu/release/etch + ~/Downloads/etch-linux
make docs-debt     # count undocumented public items (missing_docs is at allow, not warn)
make install-hooks # install pre-commit and pre-push hooks (run once per checkout)
```

**API-quality lints.** Each crate's `[lints]` table enables `missing_debug_implementations`
(`C-DEBUG`) and `clippy::wrong_self_convention` (`C-CONV`) at `warn`, which `-D warnings`
makes blocking. `missing_docs` (`C-DOCS`) sits at `allow` with a
backlog row — `make docs-debt` reports the current count. When adding a public type, derive `Debug` on it
or the build fails.

**Two types must keep a hand-written `Debug`.** `Decrypt` (holds a passphrase) and `Exec`
(holds an environment map) redact those fields manually. Replacing either with `#[derive(Debug)]`
puts a secret into every `{:?}` of that value, including transitively via `Step` and
`Box<dyn Atom>`. Regression tests: `debug_output_redacts_the_passphrase`,
`debug_output_redacts_environment_values`.

**Cross-compilation toolchain:** `cargo-zigbuild` + Zig (installed via `brew install zig` + `cargo install cargo-zigbuild`). Uses Zig's built-in C cross-compiler — no Docker required. `cross` (Docker-based) was attempted but has a known Apple Silicon bug in v0.2.5.

**Release binary:** GitHub releases ship a single Linux x86_64 binary named `etch` (no platform suffix). There is no macOS binary in releases — macOS users must build from source (`cargo build --release`).

## Action Catalog

Full field reference for every action: `~/git-repos/personal/ai-config/docs/knowledge/etch-cli-action-catalog.md`.

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
6. **Update the action table in `README.md`** and `etch-cli-action-catalog.md` in ai-config

**Auditing action names:** YAML action names come from `#[serde(rename = "...")]` in `lib/src/actions/mod.rs` — not from Rust struct names. When verifying that docs match implementation, always grep that file for the rename annotations; struct names and YAML names diverge (e.g. struct `GroupAdd` → YAML `group.add`).

Missing any step produces a compile error (missing match arm) or test failure (incorrect variant count). The `semver-check` CI job will always produce an advisory failure (`enum_variant_added`) — this is expected and non-blocking.

**TDD stub behavior:** `all_action_variants_inner_ref_and_deref` calls `inner.summarize()` on every registered variant in a loop. If the new action's `summarize()` is `todo!()`, this dispatch test also panics — expect N+1 failures (N unit tests + 1 dispatch test) in the RED phase, not just N. All are correct TDD RED state.

**Editing match arms: use `replace_all: true` for identical `=> a` patterns.** The `inner_ref`, `notify`, and `Deref` match blocks all contain arms like `Actions::MacOSDefault(a) => a,` — identical structure across blocks. When adding a new arm to each block, the Edit tool will refuse with "Found 2 matches" if you target a common pattern. Fix: use `replace_all: true` when both identical blocks need the same addition, or use enough unique surrounding context (the action above/below the insertion point) to disambiguate.

## Manifests and State

Manifest-authoring reference — the Homebrew/MAS workflow, machine-profile variables, `etch.yaml` keys and the state manifest — is in `etch-cli-manifest-authoring.md` (see Knowledge Directory). Two rules apply without reading it:

- **`brew.bundle cleanup: true` is destructive.** It removes every package not listed in the Brewfile.
- **Tests that run `etch apply` set `ETCH_STATE_DIR`** to a temporary directory. Without it, etch writes the operator's real `~/.local/share/etch/state.yaml`.

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

## Language Standards

Language-specific standards for this repo. These supplement the universal standards loaded
from `~/.claude/CLAUDE.md` (tdd, behavior, git-workflow, ci, code-standards, logic-review,
repo-structure, shell).

@~/.claude/standards/rust.md

## Testing

**Run tests:** `make test` — `cargo nextest run` plus `pytest tests/ -v`.

Unit tests in `lib/src/`, integration tests in `app/tests/` (assert_cmd + insta snapshots). Rust coverage differs by platform because macOS-only tests are gated with `#[cfg(target_os = "macos")]`, so read the gate's figure from Linux CI output, never a local macOS run.

To update insta snapshots: `INSTA_UPDATE=new cargo test --test snapshots`, then `cargo insta accept`.

**Adding a new subcommand changes `etch --help` output** — this breaks the `help` snapshot test. Always run `INSTA_UPDATE=new cargo test --test snapshots && cargo insta accept` after registering any new subcommand in `Commands`.

```bash
cargo test                                                          # all tests
cargo test -p etch-lib                                              # lib only
cargo test -p etch-cli                                              # integration only
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 81  # coverage (matches CI)
```

**Invoke the vendored `.claude/scripts/triage_log.py`, never `~/.claude/scripts/triage_log.py`.** The home-path copy writes this repo's triage log into ai-config. Paired suite: `tests/test_triage_log.py`. `cost_log.py`, `cost_summary.py` and `dod_log.py` are correct to invoke from `~/.claude/scripts/`.

**Benchmarks must name their Criterion target** — `cargo bench -p etch-lib --bench etch_lib`. A bare
`cargo bench` also runs the lib's default libtest harness, which rejects `--output-format bencher` and aborts
before any benchmark runs. See #124.

**Coverage floor: 81%** (Linux CI gate). **Exception to global ≥90% standard** — structurally uncoverable code: network ops, package managers, privilege escalation, CLI binary dispatch. Do not raise the gate above 81% without verifying actual CI output.

## CI

Single workflow `.github/workflows/ci.yml`, triggers on `pull_request` to `main`/`master` only.

| Job            | What it does                                                                                                                                                                                           |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test`         | `ruff check scripts/ tests/ .claude/scripts/` + `make test` (fmt check + clippy + cargo test) + `pytest` with Python coverage ≥87% + tarpaulin ≥81% (excluding jsonschemagen)                          |
| `cargo-audit`  | **`cargo deny check advisories`** — reads `deny.toml`'s ignore list. **Blocking** (in `auto-merge` `needs:`). Despite the job name it does _not_ run `cargo audit`, which ignores `deny.toml` entirely |
| `secret-scan`  | gitleaks v8.30.1 binary. **Blocking** (in `auto-merge` `needs:`)                                                                                                                                       |
| `snyk-scan`    | Snyk code test. **Blocking** (in `auto-merge` `needs:`)                                                                                                                                                |
| `docs-lint`    | Lints mdbook docs                                                                                                                                                                                      |
| `docs-build`   | Builds mdbook docs                                                                                                                                                                                     |
| `semver-check` | `cargo semver-checks` vs `origin/main` baseline (advisory, `continue-on-error: true`, not in auto-merge needs)                                                                                         |
| `auto-merge`   | Squash-merges the PR when all required jobs pass                                                                                                                                                       |

**Which jobs block.** Every job in `auto-merge`'s
`needs: [test, cargo-audit, secret-scan, snyk-scan, docs-lint, docs-build]` blocks the merge.
`semver-check` is the only non-blocking job: it needs both `continue-on-error: true` **and**
absence from `needs:`, and either alone is insufficient. If this table and the `needs:` list
disagree, trust the `needs:` list.

**Python tooling.**

- ruff, pytest and pytest-cov install from `requirements-ci-test.txt` **before** `Run tests`,
  because `make lint` invokes ruff. An install ordered after it fails every PR and blocks
  auto-merge.
- `ruff check` and `ruff format --check` both run, scoped to `scripts/ tests/ .claude/scripts/`
  and never the repo root. Rule set: `ruff.toml`.
- Keep `requirements-ci-test.txt` byte-identical to dotfiles master and never add a local
  header. `diff requirements-ci-test.txt ~/git-repos/personal/dotfiles/requirements-ci-test.txt`
  is the staleness check; `grep -cE '^[A-Za-z0-9._-]+==' requirements-ci-test.txt` gives the
  current package count.
- The hashed file cannot be mixed with extras: `pip install -r <hashed> extra-pkg` fails
  `--require-hashes`. A second dependency needs its own `pip install` line.
- The 87% Python floor was set from a macOS run, which is valid only while `scripts/` and
  `.claude/scripts/` contain no platform-conditional branch. If a `sys.platform` guard or
  similar enters either directory, re-derive the floor from CI output.
- `scripts/pre-push` does not trigger on `scripts/*.py`, `ruff.toml` or `Makefile`. A push
  touching only those skips the local hook, and CI is the only gate.

> **Note:** `build` job is temporarily disabled — restore when build times improve.

Pre-commit hook: `make lint` + `ggshield secret scan pre-commit`
Pre-push hook: `make test` (full suite before push reaches GitHub)

## cargo semver-checks

`make semver` runs `cargo semver-checks check-release -p etch-lib --baseline-rev origin/main`. Always use `--baseline-rev` — `etch-lib` is not published to crates.io, so the tool cannot auto-detect a registry baseline and will fail without it.

The release workflow (`release.yml`) checks against the previous git tag. The tag `v${VERSION}` is created **after** the semver check step (near the end of the workflow), so `git tag --sort=-version:refname | grep "^v" | head -1` correctly returns the previous release tag at check time — not the one being released. Do not change `head -1` to `sed -n '2p'`; that would skip a valid previous tag on future runs.

Adding a new variant to the `Actions` enum always triggers an `enum_variant_added` advisory failure on the `semver-check` CI job. This is expected — every new action adds a public enum variant, which is a semver-breaking change by the spec. The semver-check job is `continue-on-error: true` and is not in the auto-merge `needs:` list, so it never blocks the PR.

## Advisory Triage

Two advisory ignore lists exist and share nothing. `deny.toml`'s `[advisories].ignore` is read by `cargo deny check advisories`, which the blocking `cargo-audit` job runs. The `--ignore` flags in `.github/workflows/cargo-audit-scheduled.yml` are read by raw `cargo audit`, which never reads `deny.toml`. Triage every new advisory in both places.

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

- [ ] Rust coverage ≥81% on Linux CI — verify from CI output, not local macOS measurement (exception to global ≥90% — structurally uncoverable code)
- [ ] Python coverage ≥87% (`--cov-fail-under=87`) — measured floor, not the 90% target
- [ ] Plan index updated (`docs/cursor/README.md`) if this PR implements a tracked spec
- [ ] Action catalog updated in `README.md` if a new action was added
- [ ] `examples/<action>/` updated when a new action or field variant is added — at minimum one `.yaml` per option combination, with inline comments on every field
