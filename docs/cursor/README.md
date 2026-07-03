# Cursor Specs and Plans

| Date       | Plan                                                             | Spec                                                             | Status |
| ---------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- | ------ |
| 2026-06-04 | [package streaming](plans/2026-06-04-package-streaming-plan.md)  | [package streaming](specs/2026-06-04-package-streaming-spec.md)  | Done   |
| 2026-06-05 | [claude.install/upgrade](plans/2026-06-05-claude-plugin-plan.md) | [claude.install/upgrade](specs/2026-06-05-claude-plugin-spec.md) | Done   |

## Backlog

| Feature                                                                | Notes                                                                                     |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Property tests for rollback: idempotency, partial failure, permissions | Rollback took 3 PRs (#107→#113→#114) to stabilize; missing property tests were root cause |
| Wire CI test artifact upload                                           | No visibility into flaky/slow tests; `test_health.py` has no data                         |
| Evaluate perf-regression gate (Criterion benchmarks)                   | Gate exists in SDLC; no Criterion benchmarks wired in etch-cli yet                        |
