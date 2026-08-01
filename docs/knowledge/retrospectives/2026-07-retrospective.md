# 2026-07 Monthly Retrospective

**Period:** 2026-07-01 → 2026-07-31
**PRs merged:** 2
**Commits:** 9
**Versions released:** none

---

## PRs Merged

| # | Title | Merged |
|---|-------|--------|
| #120 | chore: scope enabledPlugins per ADR-0046 | 2026-07-10 |
| #121 | feat(ci): add release SBOM vulnerability monitor | 2026-07-16 |

### Detail

**#120 — Scope enabledPlugins per ADR-0046**
Updated `.claude/settings.json` to scope the `enabledPlugins` list per ADR-0046 (per-repo plugin scoping rather than global). Minor Cargo.lock refresh. Housekeeping required by a platform policy change.

**#121 — Release SBOM vulnerability monitor**
Added two GitHub Actions workflows:
- `release-sbom-monitor.yml` — reusable workflow: resolves the latest matching release tag, fetches its binary, generates a CycloneDX SBOM with `cargo-cyclonedx`, scans with `grype`, and opens a GitHub issue if new CVEs are found.
- `release-sbom-monitor-schedule.yml` — scheduled trigger (runs weekly) that calls the reusable workflow for the `etch` binary on the `v` tag pattern.

Closes a gap from the security audit: release binaries were not being scanned post-publish, so a new advisory could go undetected indefinitely between releases.

---

## June 2026 Action Items — Status

| Item | Status |
|------|--------|
| Security review: all `sh -c` invocations with manifest-supplied strings | **Not done** — still open |
| Tracking issue for snapshot-update CI automation | **Not done** — still open |
| Add `flatpak.install` action | **Not done** — still open |
| Phase 2: migrate one shell script from dotfiles into a manifest | **Not done** — still open |
| Review mutation testing results from #118 (#118 merged June 25) | **Not done** — 6 weeks of data now available |

All five June action items were deferred. The SBOM monitor (#121) advances the security theme but does not satisfy the `sh -c` review item directly.

---

## Recurring Patterns / Gotchas

- **Post-sprint deceleration is normal but June items must not carry indefinitely.** June merged 49 PRs; July merged 2. A cooldown is expected after a high-velocity sprint, but three of the June action items (#flatpak, snapshot-automation, sh-c review) are now 5+ weeks old — they risk becoming permanent backlog debt.

- **Implementation plan discipline held.** The SBOM monitor PR was preceded by a detailed implementation plan (`docs/superpowers/plans/2026-07-16-release-sbom-monitor.md`, 295 lines) and the plan index was updated on merge. The 10-80-10 cycle is sticking.

- **Most changed files:** `docs/superpowers/README.md` (3 touches — plan lifecycle updates), `docs/superpowers/plans/2026-07-16-release-sbom-monitor.md` (2 touches — plan + done marker), `.gitignore` (2 touches). No churn on `CLAUDE.md` this month (17 touches in June vs 0 in July) — AI workflow configuration has stabilized.

- **No release shipped.** The last release was v0.14.0 (June 29). The 10 features added in June are in `main` but not in a tagged release. Users tracking releases are running 5+ weeks behind the tip.

---

## Test Health

| Metric | Status |
|--------|--------|
| Linux coverage | ~82.64% (unchanged — no new code paths added) |
| macOS coverage | ~86.47% (unchanged) |
| Coverage floor (CI gate) | 81% |
| Flaky tests | 0 |
| Mutation testing | Active since #118 (June 25); ~6 weeks of data not yet reviewed |

No test regressions. No new test files added (no new features to cover). Mutation testing CI has been running for 6 weeks on each PR; the results have not been reviewed yet — a concrete action item for August.

---

## What Went Well

- **Security posture improved proactively.** The SBOM monitor means advisory drift on release binaries will surface within a week rather than at the next manual audit. This directly addresses a gap found during Phase 1.
- **Low noise.** Nine commits, two PRs, zero churn on CLAUDE.md. The codebase did not regress; nothing was reverted; no hotfixes were required.
- **Plugin scoping landed cleanly.** The ADR-0046 settings change had no downstream breakage.

---

## What to Improve

- **Carry-forward action items need a deadline, not just a mention.** All five June action items slipped to July and are now slipping to August. If an item isn't started within two weeks of the retrospective, either schedule it explicitly or close it as won't-do.
- **No release in July despite 10+ merged features.** Users on stable releases are running `v0.14.0` (June 29). Consider a lightweight release cadence — even a patch bump — when a meaningful accumulation of features sit in `main` for more than 3 weeks.
- **Mutation testing data sitting unused.** #118 added a mutation gate in June. Six weeks of per-PR reports exist and have not been reviewed. The value of the gate depends on acting on what it surfaces.
- **CI test artifact upload still unwired.** This backlog item has been open since at least mid-June (`Wire CI test artifact upload` — `test_health.py` has no data). Without it there is no visibility into test timing or flakiness trends.

---

## Action Items for August

- [ ] **Schedule sh -c security review** — all actions calling `Command::new` with manifest-supplied strings (open from June, now 5+ weeks old; set a target date)
- [ ] **Ship a release** — cut at least one release from the 10+ features sitting in `main` since June
- [ ] **Review mutation testing results from #118** — 6 weeks of data; identify any surviving mutants that reveal real coverage gaps
- [ ] **Add `flatpak.install` action** (open from June backlog)
- [ ] **Wire CI test artifact upload** — so `test_health.py` has data and flaky/slow tests become visible
- [ ] **Phase 2 milestone: migrate one shell script from dotfiles** — still pending; concrete step toward validating etch on a real workstation
