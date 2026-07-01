# 2026-06 Monthly Retrospective

**Period:** 2026-06-01 → 2026-06-30
**PRs merged:** 49
**Commits:** 50
**Versions released:** v0.13.0, v0.14.0

---

## PRs Merged

### New Actions (18 features)
| # | Title | Merged |
|---|-------|--------|
| #71 | feat(ruby.install): add version_manager field for rbenv post-install steps | 2026-06-01 |
| #72 | feat(ruby.install): add compile_flags field | 2026-06-02 |
| #74 | feat(package): add package.upgrade action for apt and snap | 2026-06-02 |
| #75 | feat(package): add version: field to package.install for version pinning | 2026-06-02 |
| #76 | feat(macos): add macos.rosetta action | 2026-06-02 |
| #77 | feat(contexts): add os.arch field to OS context provider | 2026-06-03 |
| #82 | feat(mas): add list: field to mas.install for multiple apps | 2026-06-03 |
| #89 | feat: add claude.install and claude.upgrade actions | 2026-06-05 |
| #91 | feat: add claude.marketplace, claude.marketplace.remove, and package.remove actions | 2026-06-06 |
| #92 | feat: add claude.plugin.update action | 2026-06-06 |
| #96 | feat(ruby): add ruby.chruby action and extend version_manager: chruby | 2026-06-07 |
| #98 | feat(zsh): add zsh.oh-my-zsh action | 2026-06-08 |
| #99 | feat(macos): add macos.softwareupdate action | 2026-06-08 |
| #100 | feat(terraform): add terraform.tfenv action | 2026-06-09 |
| #101 | feat(binary): add version drift detection to binary.github | 2026-06-09 |
| #102 | feat(pyenv): add recreate: field to pyenv.virtualenv | 2026-06-09 |
| #103 | feat(git.clone): add update_existing field for clone-or-pull | 2026-06-10 |
| #104 | feat(user): add user.default_shell action | 2026-06-12 |
| #108 | feat(apt): propagate DEBCONF_NONINTERACTIVE_SEEN and NEEDRESTART_MODE | 2026-06-13 |
| #116 | feat(powershell): add powershell.module action | 2026-06-22 |

### Infrastructure & UX
| # | Title | Merged |
|---|-------|--------|
| #78 | feat(plugin): re-register plugin subcommand in CLI | 2026-06-03 |
| #79 | feat(apply): add --verbose flag; suppress nothing-to-be-done by default | 2026-06-03 |
| #81 | feat(cli): add help-all subcommand to show all subcommand flags | 2026-06-03 |
| #86 | feat(apply): embed error in summary line on action failure | 2026-06-04 |
| #87 | feat(exec): stream package manager output in real time | 2026-06-05 |
| #93 | feat: add etch doctor subcommand | 2026-06-07 |
| #95 | feat(update): replace per-category flags with --only/--skip | 2026-06-07 |
| #105 | feat: add state manifest and etch history command | 2026-06-12 |
| #106 | feat: adopt 10-80-10 execution cycle (ai-config ADR-0009/0010) | 2026-06-13 |
| #107 | feat(rollback): etch rollback subcommand with pre-apply file stash | 2026-06-13 |

### Fixes
| # | Title | Merged |
|---|-------|--------|
| #88 | fix(plugin): fix etch plugin update — inverted guard, missing fetch calls, wrong remote | 2026-06-05 |
| #90 | fix(snap): remove --yes from snap install invocation | 2026-06-05 |
| #97 | fix(homebrew): detect installed casks in installed_version() | 2026-06-08 |
| #113 | fix(rollback): atomic restore and skip orphaned stash files | 2026-06-15 |
| #114 | fix(rollback): preserve original file permissions on restore | 2026-06-16 |
| #117 | fix(powershell): prevent command injection in powershell.module | 2026-06-25 |
| #119 | fix(deps): upgrade quinn-proto to 0.11.15 (RUSTSEC-2026-0185) | 2026-06-29 |

### Tests & Quality
| # | Title | Merged |
|---|-------|--------|
| #73 | test(status): add 7 integration tests for etch status | 2026-06-02 |
| #80 | test: add coverage for 7 backlog gap items | 2026-06-03 |
| #83 | test(coverage): cover actions/mod.rs false-condition and apply.rs error paths | 2026-06-03 |
| #84 | test(coverage): fix Linux tarpaulin gaps in atoms, actions, and values | 2026-06-04 |
| #85 | test(actions): cover all 40 dispatch arms in inner_ref/notify/Deref | 2026-06-04 |
| #94 | chore(lint): add cargo-machete; remove unused deps | 2026-06-07 |
| #115 | test(binary.github): replace flaky ignored test with wiremock mock | 2026-06-16 |
| #118 | ci: add mutation-pr per-PR gate workflow | 2026-06-25 |

### Docs & Housekeeping
| # | Title | Merged |
|---|-------|--------|
| #109 | docs: rename ansible-cop-review → ansible-good-practices in etch spec | 2026-06-13 |
| #110 | refactor(memory): adopt canonical .claude/memory + .claude/retrospectives layout (ADR-0014) | 2026-06-14 |
| #111 | chore: remove per-repo memory/retrospective plumbing (PR #37 follow-up) | 2026-06-14 |
| #112 | docs(knowledge): pointer stub per ADR-0020 | 2026-06-15 |

---

## Recurring Patterns / Gotchas

- **Rollback needed immediate follow-up fixes.** #107 (feature) was followed by #113 (atomic restore, orphan stash skip) and #114 (permission preservation) within 3 days — a pattern where a new subcommand lands correct-but-incomplete and edge-case hardening follows in the same period. Plan for a follow-up PR when shipping new stateful subcommands.

- **New features often need injection-guard fixes shortly after.** #116 (powershell.module) landed June 22; #117 (command-injection guard in `module_installed`) followed June 25. Security review of new shell-invoking actions should happen before merge, not after.

- **Flaky test tech debt resolved.** The `binary.github` test had been marked `#[ignore]` for some time; #115 replaced it with a wiremock stub that runs in CI. If an `#[ignore]` marker persists longer than one PR cycle, treat it as a P2 task.

- **Coverage drifted slightly during heavy feature addition.** Linux coverage went: 81.33% → 81.38% → 81.48% → 81.01% (state manifest) → 81.84% → 82.64%. The dip at #105 (state manifest) shows that large new modules can push coverage below the floor before tests catch up — worth landing tests in the same PR or immediately after.

- **Most changed files:** `CLAUDE.md` (17 touches), `docs/superpowers/README.md` (15), `README.md` (10). The high `CLAUDE.md` churn reflects ongoing calibration of the AI-assisted workflow rather than instability in production code.

---

## Test Health

| Metric | Start of period | End of period |
|--------|----------------|---------------|
| Linux coverage | ~81.3% | 82.64% |
| macOS coverage | ~85% | 86.47% |
| Coverage floor (CI gate) | 81% | 81% |
| Flaky tests | 1 (`#[ignore]` binary.github) | 0 |
| New integration test files | — | `status.rs` (7), `rollback.rs` (7) |

Coverage improved +1.3 percentage points on Linux and +1.5 on macOS over the month. The `status.rs` and `rollback.rs` test suites were added from scratch to cover the two new subcommands. Mutation testing gate added via #118 (per-PR, advisory).

---

## What Went Well

- **Velocity was very high.** 49 PRs in 30 days — roughly 1.6 per day — with two releases shipped (v0.13.0, v0.14.0). The 10-80-10 execution cycle (adopted mid-month via #106) contributed structure to the second half of the month.
- **Feature breadth.** The action catalog grew substantially: `powershell.module`, `claude.*` family (4 actions), `etch doctor`, `etch rollback`, `etch history`, `macos.rosetta`, `macos.softwareupdate`, `terraform.tfenv`, `zsh.oh-my-zsh`, and several field extensions on existing actions.
- **Security fix turn-around.** RUSTSEC-2026-0185 (quinn-proto) was patched (#119) the week it appeared.
- **Flaky test eliminated.** The long-standing `#[ignore]` on the binary.github test was replaced with a proper mock, cleaning up the test suite.
- **cargo-machete added.** Unused-dependency lint now enforced at CI time (#94).

## What to Improve

- **Security review before merge for shell-invoking actions.** The powershell injection gap (#117, 3 days after #116) was a near-miss. Any action that calls `Command::new` or `sh -c` with user-supplied strings needs explicit injection analysis before the PR merges.
- **Land tests in the same PR as the feature.** The coverage dip at #105 (state manifest) and the rollback fix series show that deferring tests to follow-up PRs creates a small window where CI passes on a coverage floor that's been temporarily broken.
- **Snapshot tests need updating discipline.** `etch --help` snapshots break whenever a new subcommand is added. The fix (`INSTA_UPDATE=new`) is documented but was applied as a separate commit several times this period. Consider adding a CI lint step or Makefile target that detects pending snapshot updates.
- **`docs/superpowers/README.md` is the most volatile doc file.** 15 touches in one month. That's healthy (plans being closed), but it means the index is near-perpetually out of date between edits. Consider automated generation from plan file frontmatter.

---

## Action Items for July

- [ ] Run security review on all actions that invoke `sh -c` with manifest-supplied strings — establish a checklist so the review happens before merge
- [ ] Open a tracking issue for snapshot-update automation (fail CI if `insta` has pending snapshots)
- [ ] Add `flatpak.install` action (backlog item added June 23)
- [ ] Phase 2 goal: migrate one real shell script from dotfiles into a manifest; use the new `etch rollback` + `etch history` to validate the result
- [ ] Investigate mutation testing results once #118 has a full period of data to review
