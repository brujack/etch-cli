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

| File         | Category | Contents |
| ------------ | -------- | -------- |
| _(none yet)_ | —        | —        |
