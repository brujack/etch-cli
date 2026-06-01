# Retrospective — 2026-06 (etch-cli)

**Period:** 2026-05-17 to 2026-06-01
**Repo(s):** etch-cli
**PRs merged:** 39 (PRs #29–#67)
**Direct master commits:** ~11 (version bumps, docs, backlog edits, gitignore)
**Total commits:** 50

---

## Summary

A two-phase period. The **first half (May 17–25)** was almost entirely quality infrastructure — nextest, proptest, CodeQL, SBOM/cosign, integration tests, mutation CI, fuzz targets, benchmarks, semver-checks, and snapshot tests all landed. The **second half (May 25–June 1)** shifted to feature delivery: 14 new action features/variants (file.link glob, binary.url, git.config, macos.defaults extended, git.pull, macos.service, systemd.service, handler/notify, file.flags, ruby.install, gem.install, pip.install, package.autoremove, npm.install, pyenv.install), two new CLI subcommands (etch status / etch update), plus five targeted bug fixes. The repo went from a well-tested action executor to something approaching a full personal workstation manager.

---

## PRs Merged This Period

### Testing & Quality Infrastructure
| PR | Title |
|----|-------|
| #29 | Adopt cargo-nextest as test runner |
| #30 | Add proptest property-based tests |
| #31 | Add coverage badge |
| #32 | Add CodeQL SAST workflow |
| #33 | Flaky-test tracking via nextest CI profile and test-metrics artifact |
| #34 | SBOM generation and cosign signing for releases |
| #35 | Integration tests for file.link, file.copy, command.run, directory.create |
| #36 | Monthly mutation testing workflow for etch-lib |
| #38 | Exit nonzero on manifest parse errors and step failures |
| #39 | cargo-fuzz targets for manifest parsing and path resolution |
| #40 | Criterion benchmarks for etch-lib |
| #41 | cargo-semver-checks for etch-lib API compatibility |
| #42 | insta snapshot tests for CLI output |
| #52 | Add missing fish/contexts/plugin CLI tests |
| #53 | Mutation score gate at 60% |

### Release Pipeline
| PR | Title |
|----|-------|
| #37 | git-cliff changelog generation in release workflow |
| #58 | Scope release notes to latest tag via git-cliff-action |
| (direct) | Generate and publish SHA256 checksum |

### New Actions / Features
| PR | Title |
|----|-------|
| #43 | file.link: glob/wildcard pattern support |
| #44 | binary.url: install binary from arbitrary URL |
| #45 | git.config: declarative gitconfig management |
| #46 | macos.defaults: array-add and delete operations |
| #47 | git.pull: clone if missing, pull if present |
| #48 | macos.service: declarative launchctl management |
| #49 | systemd.service action |
| #50 | Handler/notify pattern (Ansible-style) |
| #51 | file.flags: BSD file flags action (macOS) |
| #54 | personal-workstation machine setup template |
| #56 | Drift detection via etch status command |
| #59 | etch update subcommand |
| #61 | ruby.install action via ruby-install |
| #62 | gem.install action |
| #63 | pip.install action |
| #65 | package.autoremove action for apt orphan cleanup |
| #66 | npm.install action for global npm packages |
| #67 | pyenv.install action |

### Bug Fixes
| PR | Title |
|----|-------|
| #55 | Clear GIT_DIR in git-config-unset execute() to prevent hook repo corruption |
| #57 | Resolve three known bugs and improve apply output detail |
| #60 | Scope pip list to user packages; extract testable helpers |
| #64 | make add_to_group idempotent via id -nG membership check |
| (direct) | Expand ~ in file.link source paths before resolving |

---

## Recurring Patterns & Gotchas

**GIT_DIR contamination** — `git-config-unset` atom inherited `GIT_DIR` from the process environment when run inside an etch session that itself was invoked from a git hook. This caused the atom's `git config` calls to operate on the hook repo rather than the target repo. Fix: `unset GIT_DIR` in `execute()` before invoking git. **Takeaway:** atoms that invoke git must always clear `GIT_DIR` and `GIT_WORK_TREE`.

**pip user-scope drift** — `pip list` returns system packages when run as root. The `etch update` subcommand's update-check used `pip list` without `--user`, which overstated what needed upgrading. Fixed by scoping to `pip list --user`. **Takeaway:** when writing update logic for Python packages, always scope to the user install unless explicitly managing system Python.

**Exit code propagation gap** — `command.run` was not setting `status.code` from the actual shell exit code on failure. This meant failed commands appeared as "unknown exit code" in structured output. Found and fixed in PR #57.

**user.group idempotency** — `add_to_group` would fail with an error if the user was already a member of the group (instead of being a no-op). Fixed in #64 via `id -nG` membership check at plan time.

**handler/notify ordering** — adding an Ansible-style notify pattern to the step executor required care not to double-run handlers when multiple actions in a manifest notify the same handler. The deduplication logic is the load-bearing constraint — worth reviewing if the pattern sees heavy use.

**semver-check advisory on every new action** — adding a public `Actions` enum variant always triggers `enum_variant_added` from cargo-semver-checks. This is expected (job is `continue-on-error: true`, not in `needs:` for auto-merge). Every action-shipping PR will produce this advisory indefinitely.

---

## Test Health

| Metric | Start of period | End of period |
|--------|----------------|---------------|
| Coverage (Linux CI) | ~75% | ~75% (stable) |
| Coverage (macOS local) | ~75% | ~82% |
| Test runner | cargo test | cargo-nextest |
| Mutation gate | none | 60% score threshold |
| Snapshot tests | none | 5 (CLI output) |
| Integration tests | 0 | 11 (apply path) |
| Fuzz targets | none | 2 (manifest parsing, path resolution) |
| Benchmarks | none | Criterion suite in etch-lib |

No flaky tests detected. The nextest retry profile and test-metrics artifact are wired up but have not needed to catch anything yet.

**Remaining coverage gaps (per backlog):** `etch status` (0%), `etch plugin list/remove` (0%), `apply.rs` error paths (69%), `actions/mod.rs` variant condition branch (79%), privileged paths in file/copy and file/link.

---

## What Went Well

- **Quality infrastructure landed in one sprint.** Seven testing/quality PRs (#29–#36) merged in four days (May 18–21) with no regressions. The repo went from bare cargo-test to nextest + proptest + fuzz + benchmarks + mutation CI in a single push.
- **Release pipeline is now production-grade.** SBOM, cosign signing, SHA256 checksums, git-cliff CHANGELOG, and scoped release notes all shipped as a coherent suite. The binary is now auditable end-to-end.
- **Action velocity held through the quality sprint.** 14 new action variants shipped in the second half without sacrificing the spec → plan → TDD → catalog update cycle.
- **handler/notify is a genuine architecture upgrade.** The Ansible-style notify pattern means manifests can declare reactive steps (e.g. restart a service only when a config file changed) without chaining command.run hacks.
- **etch update + etch status** complete the "daily driver" loop: status shows drift, update brings packages and dotfiles current.
- **Bugs surfaced organically during real-use testing.** The GIT_DIR, pip scoping, exit code, and user.group bugs were all found by running real manifests, not via CI. The integration tests added this period would now catch three of the four.

---

## What to Improve

- **`etch status` coverage is 0%** — the new drift-detection command has no integration tests despite being a prime candidate (deterministic output, no side effects). This is the biggest single CI coverage gap on Linux.
- **Backlog coverage items not advancing** — the backlog has eight specific coverage gap entries (etch status, plugin list/remove, apply.rs error paths, etc.) written up with exact line counts and test approaches. None were implemented this period. They remain well-specified and low-risk.
- **Pending specs still pending** — four Nix-parity specs (state-manifest, version-pinning, file-rollback, package-upgrade) were written 2026-05-29 and have not moved to plans. `etch-update-command` spec from the same date was completed, but the others sit untouched.
- **docs/superpowers/README.md is the most-touched file again (25 changes)** — same pattern as last period (61 changes). The status-update cadence (Pending → In Progress → Done × per action) is the right behavior, but it still means ~2–3 commits per action just for the index. No fix proposed — the table is the source of truth and it's working.
- **rbenv post-install gap** — `ruby.install` doesn't run `rbenv global` or `rbenv rehash` after `ruby-install`. This is the only open bug and requires follow-on `command.run` workaround in manifests. Documented in bugs table; low-effort fix would be an optional `set_global: true` field.

---

## Actions for Next Period

- [ ] Add integration tests for `etch status` output format — deterministic, no side effects, highest coverage bang-for-buck on Linux
- [ ] Fix `ruby.install` rbenv gap — add optional `set_global: true` field (or follow-up `command.run` example in docs/examples/)
- [ ] Move pending specs to plans: pick one of state-manifest, version-pinning, or file-rollback
- [ ] Fix DEBIAN_FRONTEND/needrestart bug (documented in bugs table since May 30) — likely a `DEBIAN_FRONTEND=noninteractive` env var on the apt atom or a needrestart kill step
- [ ] Address verbose apply output backlog item — "nothing to be done" messages without context are the most common daily-use friction
- [ ] `pyenv.install configure_opts` backlog item — the macOS Homebrew interference issue will bite on any macOS setup with Homebrew-managed libmpdec

---

## Calibration Note

39 PRs in 15 days while maintaining spec → plan → TDD discipline is the highest sustained velocity of the project. The quality sprint (PRs #29–#42) is particularly notable: it took ~6 days to add the full testing infrastructure that most projects never get around to, and it was done before the action sprint rather than after. The payoff is immediate — the etch-status and etch-update features were built against a test suite that includes nextest, proptest, snapshot locking, and integration tests. The mutation gate at 60% is conservative but honest given the structural uncoverability ceiling.
