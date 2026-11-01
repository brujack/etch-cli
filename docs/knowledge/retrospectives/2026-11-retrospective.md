# 2026-11 Monthly Retrospective

**Period:** 2026-09-01 → 2026-10-31
**PRs merged:** 29
**Commits:** ~50
**Versions released:** none

---

## PRs Merged

### September 2026 (1 PR)

| # | Title | Merged |
|---|-------|--------|
| #130 | fix(deps): bump rustls to 0.23.45 for RUSTSEC-2026-0285 | 2026-09-28 |

### October 2026 (28 PRs)

| # | Title | Merged |
|---|-------|--------|
| #131 | fix(deps): clear rustls and zip advisories on main | 2026-10-17 |
| #132 | fix(file.flags): clear flags correctly on privileged path | 2026-10-17 |
| #133 | fix(brew.bundle): pass --force-cleanup instead of disabled --cleanup | 2026-10-17 |
| #134 | chore(settings): disable skill-creator plugin | 2026-10-18 |
| #135 | ci: restore requirements-ci-test.txt parity with dotfiles | 2026-10-19 |
| #136 | fix(ci): hold every bot PR on is_bot, not the Renovate login | 2026-10-19 |
| #137 | test(ci): pin multi-label automerge-ok matching in bot-hold test | 2026-10-19 |
| #138 | ci(renovate): enable held lockfile maintenance; schedule at any time | 2026-10-20 |
| #139 | fix(deps): hickory-resolver 0.26 and patched lockfiles | 2026-10-20 |
| #140 | docs(backlog): prioritize rows P1-P4 | 2026-10-25 |
| #141 | fix(deps): clear 2026-10-25 RustSec advisories | 2026-10-25 |
| #142 | docs(backlog): raise flatpak.install to P2 | 2026-10-25 |
| #143 | docs(backlog): add CI speed findings from math#174-177 | 2026-10-26 |
| #144 | docs(backlog): add plan-cost and claude.upgrade findings | 2026-10-26 |
| #145 | docs(backlog): add stale jsonschemagen schema row | 2026-10-26 |
| #146 | chore(renovate): add 14-day minimumReleaseAge | 2026-10-26 |
| #147 | docs(spec): add approved CI and release hardening spec | 2026-10-27 |
| #148 | fix(ci): unmask security scans and verify every download | 2026-10-27 |
| #149 | fix(release): validate version, scope notes, push only the tag | 2026-10-27 |
| #150 | ci: pin cargo tools through taiki-e/install-action | 2026-10-27 |
| #151 | fix(make): build into target/, install to XDG_BIN_HOME | 2026-10-27 |
| #152 | chore: drop vendored triage_log.py | 2026-10-28 |
| #153 | ci: raise Python coverage floor to 90 | 2026-10-28 |
| #154 | fix(binary): extract via temp file and rename | 2026-10-30 |
| #155 | fix(http): reject error statuses and write downloads atomically | 2026-10-30 |
| #156 | fix(claude): update each listed plugin once | 2026-10-30 |
| #157 | fix(schema): regenerate schema and fail CI when stale | 2026-10-30 |
| #158 | ci: run coverage in parallel jobs beside test | 2026-10-31 |
| #159 | feat(deps): cargo-deny gate, unsafe guard, command.run env fix, edition 2024 | 2026-10-31 |

### Detail

**#130 / #131 / #141 — Dependency hygiene: rustls, zip, hickory, RustSec**
Three advisory-clearing passes during the period. RUSTSEC-2026-0285 (rustls) triggered the first; a follow-up (#131) also cleared a zip advisory on main. October 20 saw hickory-resolver bump to 0.26 with patched lockfiles (#139). A batch of new 2026-10-25 advisories was cleared same-day (#141). Dependency hygiene is now a routine, fast operation — the cargo-deny workflow and patched deny.toml make triage a one-PR affair.

**#132 — fix(file.flags): clear flags correctly on privileged path**
`file.flags` had a logic bug: when run with `privileged: true`, the uchg/schg clear step was not correctly applied before setting new flags. Fixed by restructuring the atom to handle the privileged path explicitly.

**#133 — fix(brew.bundle): pass --force-cleanup**
`brew bundle cleanup` was invoked with `--cleanup`, which was silently ignored in recent Homebrew versions (the flag was renamed to `--force-cleanup`). The action was a no-op in practice. Fixed by passing the correct flag. Note: `cleanup: true` is still destructive — this fix means it will now actually remove unlisted packages.

**#135 / #136 / #137 — CI: requirements sync and Renovate bot-hold fix**
`requirements-ci-test.txt` had drifted from dotfiles master. Restored parity (#135). The bot-hold CI job had been matching only the Renovate login specifically rather than the generic `is_bot` flag, meaning other bot actors could bypass the hold (#136). Fixed and pinned the multi-label matching logic in the test (#137).

**#138 — ci(renovate): lockfile maintenance and schedule**
Enabled Renovate lockfile maintenance (previously disabled) so `Cargo.lock` gets periodic bump PRs for patch updates that do not change `Cargo.toml`. Scheduled to run at any time (removed the restricted window). This closes the gap where patch-level transitive updates could accumulate unseen.

**#147–#153 — CI and release hardening sprint**
Five PRs in a single day (Oct 27) landed the approved CI/release hardening spec:
- Unmasked secret scan and Snyk downloads; added checksum verification on every downloaded binary (#148).
- Release workflow now validates the version string, scopes release notes to the tag range, and pushes only the tag (not a force-push of main) (#149).
- Pinned all cargo tools through `taiki-e/install-action` so CI tool versions are deterministic (#150).
- Fixed `make build` / `make install` targets to put output in `target/` and respect `XDG_BIN_HOME` (#151).
- Dropped vendored `triage_log.py` that had been carried as dead code since the fork (#152).
- Raised the Python coverage floor from 87% to 90% (#153).

**#154 / #155 — Atomic file operations for binary and http**
`binary` action was writing extracted files directly, leaving a partial/corrupt file on failure. Now extracts to a temp file and renames atomically (#154). `http.download` was not checking HTTP error status codes and was also writing non-atomically; both fixed (#155). These were latent correctness bugs that would surface under network failure or interrupted writes.

**#157 — Schema staleness detection**
`jsonschemagen/tests/schema_current.rs` had always existed but was not running in CI in a way that caught stale `schema.json`. Fixed CI to regenerate and fail on diff. Added `make schema` to the Makefile's test surface.

**#158 — Coverage parallelization**
Coverage (`cargo tarpaulin`) was previously blocking the `test` job's wall-clock time. Extracted to its own parallel job so tarpaulin runs beside `test` instead of after it. This was the primary CI speed win from the math#174-177 findings.

**#159 — Edition 2024, cargo-deny gate, unsafe guard, command.run env fix**
Omnibus PR completing the Rust standards spec:
- Migrated all four crates to Edition 2024.
- Replaced the cargo-audit CI job with `cargo deny check` (advisories + licenses + bans + sources).
- Added `unsafe_code = "deny"` workspace lints; each existing `unsafe` block got `#[expect(unsafe_code, reason = ...)]`.
- Fixed `command.run env:` — the map was leaking into etch's own environment instead of being scoped to the child process; `privileged: true` with `env:` is now refused at the action level.

---

## August 2026 Action Items — Status

| Item | Status |
|------|--------|
| Ship a release | **Not done** — still open (open 3 months) |
| sh-c security review or close it | **Not done** — still open (open 5 months) |
| Review mutation testing results | **Not done** — still open (5 months of data unreviewed) |
| Add `flatpak.install` action | **Not done** — raised to P2 backlog priority (#142) |
| Wire CI test artifact upload | **Not done** — still open |
| Phase 2: migrate one shell script from dotfiles | **Not done** — still open |

Six-for-six deferrals again. September was nearly idle (1 PR); October was an intense hardening sprint with no feature work. The same six items have now been open 3-5 months without progress.

---

## Recurring Patterns / Gotchas

- **Late-month clustering.** September saw 1 PR; October saw 28, with 10 of those landing in the last two days of the month. This all-at-the-end pattern compresses review and risks rushing. The CI hardening sprint was deliberate, but the September dead zone was not — it reflects starting the work late.

- **cargo-deny vs. cargo audit naming confusion.** The CI job is named `cargo-audit` but runs `cargo deny`. This bit once (an advisory that cargo audit would have surfaced differently). CLAUDE.md documents this, but the job name still misleads. Adding it as a gotcha here in case the job name is revisited.

- **requirements-ci-test.txt drift recurs.** This is the second time (August was the first) that the requirements file drifted from dotfiles master. A `test_requirements_sync.py` now guards against this in CI, but the underlying cause (manual edits to the file) keeps creating the gap.

- **`brew bundle cleanup` was silent for months.** The `--cleanup` → `--force-cleanup` rename in Homebrew happened silently; there were no warnings, no errors, just no package removal. Any action that wraps a CLI tool's flags needs to be validated against the current CLI behavior occasionally.

- **Atomic writes matter more than they seem.** Both the binary (#154) and http (#155) actions had non-atomic writes. Neither had caused data loss in practice, but both would have on a crash or network timeout mid-write. Worth a periodic audit of any action that writes files: does it write to a temp path and rename, or write directly?

- **Most changed files this period:** `docs/superpowers/README.md` (24 touches — backlog grooming), `CLAUDE.md` (14), `.github/workflows/ci.yml` (9), the Rust standards spec (8), `README.md` and `Makefile` (6 each). Again a high documentation-to-code ratio — the edition 2024 and hardening work was specification-heavy.

---

## Test Health

| Metric | Status |
|--------|--------|
| Linux Rust coverage | ~82% (stable, no new uncoverable paths added) |
| Rust coverage floor (CI gate) | 81% |
| Python coverage floor (CI gate) | 90% (raised from 87% in #153) |
| Flaky tests | 0 observed |
| Mutation testing | Active since #118 (June 25); ~5 months of per-PR data unreviewed |
| Schema staleness gate | New in #157 — now blocking in CI |

Coverage is stable. The Python floor raise to 90% passed without any new test additions, confirming the existing suite was already at or above that threshold. The main test health concern remains the five months of unreviewed mutation testing output.

---

## What Went Well

- **October hardening sprint was highly productive.** 28 PRs in a month is exceptional for this repository. The CI/release hardening spec (approved and fully implemented in one sprint) was the most structurally significant work since Phase 1.
- **Spec-driven development continues to work.** The Rust standards spec went through three review rounds before approval; the implementation (#159) landed without surprises. The CI hardening spec (#147) was approved and implemented same-week.
- **Same-day advisory response.** The 2026-10-25 RustSec advisories were cleared same-day (#141). The advisory pipeline (cargo-deny → deny.toml → one PR) is now fast enough for reactive triage.
- **Edition 2024 migration complete.** Migrating four crates to Edition 2024 (including the `unsafe_code = "deny"` guard and rustfmt `style_edition`) was handled cleanly in a single PR with no regressions.
- **Atomic write fixes found and fixed proactively.** #154 and #155 found latent correctness bugs in two actions before they caused user-visible failures.

---

## What to Improve

- **Feature work remains frozen for 5 months.** The backlog (flatpak.install, sh-c review, Phase 2 migration, release) has been open since June. The CI hardening and security work was valuable, but there has been no net-new functionality shipped to end users in the entire post-fork period after Phase 1. This is the highest-priority process concern.
- **No release since v0.14.0 (June 29).** Main is now ~85 commits ahead of the last release. Users tracking releases see nothing from the past 4 months.
- **September was nearly idle.** One PR in a full month. This suggests the batch-at-end pattern is structural, not accidental — work gets deferred until a deadline forces action.
- **Mutation testing data still unreviewed.** Five months of cargo-mutants output exists and no one has read it. Either review it or deprioritize the gate to avoid the appearance of a gate that produces waste.
- **CI speed backlog added but not addressed.** Math#174-177 findings were recorded in the backlog (#143) and one win (coverage parallelization, #158) was landed, but the remaining CI speed improvements are still pending.

---

## Action Items for December

- [ ] **Ship a release** — v0.15.0 minimum; main is ~85 commits ahead of v0.14.0 (open 3 months; hard deadline: by end of November)
- [ ] **sh-c security review or close it** — five months open; set a final go/no-go decision (not just "still open") before this retro's next cycle
- [ ] **Review mutation testing results** — 5 months of reports; spend one session reading the output and record findings even if the conclusion is "surviving mutants are all in uncoverable paths"
- [ ] **Add `flatpak.install` action** — now P2 priority in the backlog; assign to a concrete sprint
- [ ] **CI speed remaining items** — address the remaining findings from math#174-177 beyond the coverage parallelization already landed
- [ ] **Phase 2 milestone: migrate one shell script from dotfiles** — still pending; no new deadline, but must not carry forward to a seventh month
