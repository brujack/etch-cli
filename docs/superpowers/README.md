# Superpowers Specs and Plans

Master status index for all specs and implementation plans in this directory.

## Status Key

| Status      | Meaning                          |
| ----------- | -------------------------------- |
| Done        | Implemented and merged to master |
| In Progress | Currently being implemented      |
| Pending     | Not yet started                  |

---

## All Plans

| Date       | Plan                                                     | Spec                                                            | Status      |
| ---------- | -------------------------------------------------------- | --------------------------------------------------------------- | ----------- |
| 2026-05-02 | [etch-cli-phase1](plans/2026-05-02-etch-cli-phase1.md)   | —                                                               | In Progress |
| 2026-05-02 | [platform-pruning](plans/2026-05-02-platform-pruning.md) | [platform-pruning](specs/2026-05-02-platform-pruning-design.md) | Done        |
| 2026-05-02 | [test-coverage](plans/2026-05-02-test-coverage.md)       | [test-coverage](specs/2026-05-02-test-coverage-design.md)       | Pending     |

---

## Backlog

| Feature                         | Notes                                                                                     |
| ------------------------------- | ----------------------------------------------------------------------------------------- |
| ntfy notification action        | Matches existing notification infra                                                       |
| macOS defaults write ergonomics | If etch-cli's current API is rough                                                        |

---

## Adding a new entry

When a new spec or plan is created, add a row to the All Plans table. Set status to **In Progress** when implementation starts, **Done** when the PR merges. Also add a `> **Status: DONE**` banner at the top of the plan file once complete. Move backlog items to the All Plans table when their spec is written (remove the backlog row).
