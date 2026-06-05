# Knowledge Directory — etch-cli

Reference material for the etch-cli Rust codebase. Not instructions, not workflows, not coding conventions — reference documents for understanding the architecture, action system, and curated research.

## Categories

### Architecture docs (non-ADR)

Descriptions of how etch-cli works that are too detailed for CLAUDE.md but don't rise to the level of an architectural _decision_ record. Examples:

- How the step initializer pipeline evaluates FlowControl variants
- The atom execution lifecycle (plan → initializers → execute → finalizers)
- How manifests are loaded, parsed, and dependency-sorted via petgraph
- How contexts (user, OS, variables, Rhai) are built and passed to actions
- The ConditionalVariantAction wrapper and how OS/profile conditions are evaluated

ADRs (`docs/adr/`) record _decisions_. Architecture docs here describe _how things work_.

### Saved web research

Curated findings from the web-research skill (Exa + Firecrawl) worth preserving across sessions. Save here instead of re-fetching next time. Examples:

- Rust crate API notes (serde, schemars, mlua, normpath quirks)
- Tarpaulin instrumentation behavior discovered via research
- Upstream comtrya design notes relevant to etch-cli

Use file names like `research-<topic>.md` to distinguish from architecture docs.

### Retrospectives

Periodic process reviews covering trends, recurring gotchas, skill usage, and actions for the next period. Filed under `retrospectives/YYYY-MM-DD.md`.

### Other reference material

Reference sheets for the YAML manifest schema, action catalog summaries, or cross-cutting patterns that don't fit the above categories.

## What does not belong here

| Content type                         | Where it lives                          |
| ------------------------------------ | --------------------------------------- |
| Instructions / behavioral directives | `CLAUDE.md`                             |
| Reusable workflows                   | `~/.claude/skills/etch-cli-new-action/` |
| Coding conventions                   | `~/.claude/standards/rust.md`           |
| Plans and specs                      | `docs/cursor/` or `docs/superpowers/`   |
| Architectural decisions              | `docs/adr/`                             |

## File naming

`<topic>.md` or `research-<topic>.md` — lowercase with hyphens. One topic per file.

## Index

Add a row to this table when you create a file:

| File                                                                               | Category      | Contents                                                                                                |
| ---------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------- |
| [action-catalog.md](action-catalog.md)                                             | Reference     | Full field reference for all 40 actions                                                                 |
| [retrospectives/2026-05-17.md](retrospectives/2026-05-17.md)                       | Retrospective | Period: 2026-05-06 – 2026-05-17; PRs #1–#18; fork setup, CI, first actions                              |
| [retrospectives/2026-06-retrospective.md](retrospectives/2026-06-retrospective.md) | Retrospective | Period: 2026-05-17 – 2026-06-01; PRs #19–#69; testing infra, release pipeline, new actions              |
| [retrospectives/2026-06-05.md](retrospectives/2026-06-05.md)                       | Retrospective | Period: 2026-06-01 – 2026-06-05; PRs #70–#89; coverage sprint, streaming output, claude.install/upgrade |
