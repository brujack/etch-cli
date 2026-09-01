# 2026-08 Monthly Retrospective

**Period:** 2026-08-01 → 2026-08-31
**PRs merged:** 8
**Commits:** 24
**Versions released:** none

---

## PRs Merged

| # | Title | Merged |
|---|-------|--------|
| #122 | feat(lints): enforce C-DEBUG and C-CONV, record the C-DOCS debt | 2026-08-02 |
| #123 | feat(lint): add ruff gate and adopt the shared rule set | 2026-08-04 |
| #124 | fix(ci): scope benchmarks to the Criterion target, stop masking its failure | 2026-08-06 |
| #125 | fix(ci): gate all four tracked shell files, fix a fail-open hook | 2026-08-08 |
| #126 | ci: adopt the shared pinned Python rendering, pytest, and a coverage gate | 2026-08-21 |
| #127 | ci: scope the vendored rendering to ci-test (80 pins to 11) | 2026-08-22 |
| #128 | fix(deps): bump gix 0.80 to 0.87, clearing 23 advisories | 2026-08-22 |
| #129 | ci: hold unlabelled Renovate PRs for triage | 2026-08-24 |

### Detail

**#122 — Enforce C-DEBUG and C-CONV, record C-DOCS debt**
Enabled `missing_debug_implementations` and `clippy::wrong_self_convention` at `warn` (made blocking by `-D warnings`) in both crate `[lints]` tables. Recorded 402 undocumented public items as a known backlog via `make docs-debt`. Two hand-written `Debug` impls (`Decrypt`, `Exec`) were preserved as intentional redaction — captured in CLAUDE.md and regression tests (`debug_output_redacts_the_passphrase`, `debug_output_redacts_environment_values`). Added ADR-0015.

**#123 — Add ruff gate and adopt the shared rule set**
Introduced `ruff check` (and `ruff format --check`) scoped to `scripts/ tests/ .claude/scripts/`, wired into `make lint`. Adopted the shared `ruff.toml` rule set from ai-config (ADR-0058). Moved ruff 0.16.1 → 0.16.4.

**#124 — Scope benchmarks to the Criterion target**
`cargo bench` without a `--bench` flag was also running the lib's default libtest harness, which rejects `--output-format bencher` and aborted before any benchmark ran. Fixed by pinning the bench invocation to `--bench etch_lib`. Also added the Python test step (`pytest tests/ -v`) to `make test` — 42 Python tests that had only run in CI now run locally too.

**#125 — Gate all four tracked shell files, fix fail-open hook**
The pre-push hook's trigger pattern had a gap: it only matched some of the shell files tracked by `make lint`. Extended the trigger to cover all four. Fixed a fail-open condition in the hook (hook could exit 0 even when lint failed).

**#126 — Adopt shared pinned Python rendering, pytest, and coverage gate**
Switched CI Python install from hand-pinned `ruff==0.16.1` to `requirements-ci-test.txt` — the hash-verified rendering of the shared dev-venv package set. Changed test runner from `unittest discover` to `pytest` (picks up existing `unittest.TestCase` classes natively). Added Python coverage gate at 87% (`--cov-fail-under=87`). Reduced install from 80 packages to 11 (split by CI purpose, dropping cosmic-ray closure). Added ADR-0016.

**#127 — Scope vendored rendering to ci-test (80 pins → 11)**
After #126 landed, the `requirements-ci.txt` was still the wider set (80 packages). Narrowed it to the `ci-test` subset (11 packages: ruff, pytest, pytest-cov and their closures). A stale `requirements-ci.txt` artifact was also cleaned up.

**#128 — Bump gix 0.80 → 0.87, clearing 23 advisories**
The `gix` family was at 0.80, carrying 23 open security advisories in the `deny.toml` ignore list. Bumped to 0.87, which resolved all 23. The remaining advisories (hickory-proto ×2, rsa) are pre-existing and structurally unfixable. Required a spec/plan cycle (`docs/superpowers/plans/2026-08-22-gix-bump.md`) due to breaking API changes across 7 minor versions. Renovate's digest-pinning and pip-enabling were also evaluated here — pinDigests was enabled per ADR-0006; pip was deliberately excluded (recorded in docs).

**#129 — Hold unlabelled Renovate PRs for triage**
Renovate was merging all PRs automatically (including potential majors). Added a Renovate label strategy (`patch`/`minor` auto-merge; `major` held for manual review) and a CI job that blocks auto-merge on PRs with no `patch`/`minor`/`major` label. Prevents surprise major bumps from slipping through auto-merge.

---

## July 2026 Action Items — Status

| Item | Status |
|------|--------|
| Schedule sh -c security review | **Not done** — still open (open since June) |
| Ship a release | **Not done** — still open |
| Review mutation testing results from #118 | **Not done** — still open (3 months of data now) |
| Add `flatpak.install` action | **Not done** — still open (open since June) |
| Wire CI test artifact upload | **Not done** — still open |
| Phase 2: migrate one shell script from dotfiles | **Not done** — still open |

All six July action items deferred. August was a CI/toolchain consolidation month with no feature work. The sh-c review and flatpak action have now been open since June — three months without progress.

---

## Recurring Patterns / Gotchas

- **CI hardening is the dominant theme.** All 8 PRs touched CI, linting, or dependency hygiene — zero feature PRs. This was a deliberate consolidation sprint clearing technical debt accumulated since the fork.

- **The "23 advisories" backstory.** gix 0.80 had been in place since the fork (Phase 1 audit). The 23 advisories were always known but blocked behind API churn across 7 minor gix releases. The spec/plan cycle (#128) was the right call: the API surface was non-trivial. The lesson: advisory debt that requires a major refactor needs a formal plan, not just a `cargo update`.

- **Renovate without triage gates is risky.** Before #129, Renovate was implicitly trusted to auto-merge everything, including potential major bumps. The label/triage gate is now the standard pattern (matching ai-config fleet practice). Any project using Renovate auto-merge should have this gate.

- **Requirements-rendering drift was a real gap.** The hand-pinned `ruff==0.16.1` in CI vs 0.16.4 in the shared rendering meant CI and local runs could diverge silently. The shared rendering (#126/#127) closes this for the three tools this project actually uses.

- **Benchmark masking.** A bare `cargo bench` that silently exits 0 without running any benchmarks is a subtle failure — it looks green in CI. The fix (#124) was a one-liner, but the root cause (cargo's bench dispatch behavior) is non-obvious. Documented in CLAUDE.md.

- **Most changed files:** `CLAUDE.md` (7 touches — toolchain, linting, CI doc updates), `docs/superpowers/README.md` (6 touches), `renovate.json` (5 touches), `Makefile` (5 touches), `.github/workflows/ci.yml` (5 touches). High documentation-to-code ratio — expected for a toolchain sprint.

---

## Test Health

| Metric | Status |
|--------|--------|
| Linux Rust coverage | ~82.64% (unchanged — no new code paths) |
| macOS Rust coverage | ~86.47% (unchanged) |
| Rust coverage floor (CI gate) | 81% |
| Python coverage floor (CI gate) | 87% (new in #126) |
| Flaky tests | 0 |
| Mutation testing | Active since #118 (June 25); ~3 months of per-PR data not yet reviewed |

No Rust test regressions. The 42 Python tests that had only run in CI (via `ci.yml`) now also run locally via `make test` (#124). The Python coverage gate (87%) was added in #126 and is now blocking. Mutation testing data continues to accumulate unreviewed — now three months of reports.

---

## What Went Well

- **CI/toolchain debt cleared systematically.** Eight PRs in a focused sprint resolved the ruff gate, Python rendering, benchmark masking, hook gaps, 23 gix advisories, and Renovate triage. The backlog that had been accumulating since the fork is meaningfully smaller.
- **Spec/plan discipline on the hard PR.** The gix bump (#128) went through a design spec and implementation plan before any code was touched. The API breakage surface was correctly predicted in the plan, preventing surprises.
- **gix bump cleared 23 advisories without any advisory carry-forward.** The remaining advisories (hickory-proto ×2, rsa) were already known and pre-existing. Net advisory count held flat.
- **Renovate triage gate is future-proof.** The label strategy in #129 is the correct permanent baseline — not a workaround that needs revisiting.

---

## What to Improve

- **Feature backlog is frozen.** No feature PRs in August. `flatpak.install`, the sh-c security review, Phase 2 dotfiles migration, and the release are all still pending — now 2-3 months old. CI quality is in good shape; it's time to ship something.
- **No release in August** — `v0.14.0` (June 29) is still the current release. The tip of `main` is ~60 commits ahead of the last release. Users tracking releases see nothing from the past two months.
- **Mutation testing data is wasted value.** Three months of per-PR cargo-mutants reports exist and have not been reviewed. The gate does nothing if the output is never acted on.
- **CI test artifact upload still unwired.** Flaky/slow test visibility remains zero. This has been on the action items list since at least July.
- **sh-c review is the longest-running open item.** Open since June retrospective, now in its third month. Either schedule a concrete date to do it or close it as won't-do and document why.

---

## Action Items for September

- [ ] **Ship a release** — cut at least v0.15.0 from the ~60 commits ahead of v0.14.0 in `main` (open 2 months; concrete target: by end of September)
- [ ] **sh-c security review or close it** — all actions calling `Command::new` with manifest-supplied strings; either do the review or close as won't-do with a documented rationale (open 3 months; set a final deadline)
- [ ] **Review mutation testing results** — read the per-PR cargo-mutants reports from the past 3 months; identify any surviving mutants that reveal real coverage gaps (open 3 months)
- [ ] **Add `flatpak.install` action** — open from June backlog (open 3 months)
- [ ] **Wire CI test artifact upload** — so `test_health.py` has data and flaky/slow tests become visible (open 2 months)
- [ ] **Phase 2 milestone: migrate one shell script from dotfiles** — still pending; concrete step toward validating etch on a real workstation
