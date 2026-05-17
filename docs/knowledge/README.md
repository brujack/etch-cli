# Knowledge Directory — etch-cli

Reference material for the etch-cli Rust codebase. Not instructions, not workflows, not coding conventions — reference documents for understanding the architecture, action system, and curated research.

## What belongs here

- Architecture overviews (action system, step initializer pipeline, FlowControl, atom execution)
- Reference sheets for the YAML manifest schema and action catalog
- Curated research findings from the web-research skill worth preserving across sessions
- Anything too detailed for a CLAUDE.md paragraph but useful to look up

## What does not belong here

| Content type                         | Where it lives                          |
| ------------------------------------ | --------------------------------------- |
| Instructions / behavioral directives | `CLAUDE.md`                             |
| Reusable workflows                   | `~/.claude/skills/etch-cli-new-action/` |
| Coding conventions                   | `~/.claude/standards/rust.md`           |
| Plans and specs                      | `docs/cursor/` or `docs/superpowers/`   |
| Architectural decisions              | `docs/adr/`                             |

## File naming

`<topic>.md` — lowercase with hyphens. One topic per file.

## Adding a file

Add a row to this table when you create a file:

| File         | Contents |
| ------------ | -------- |
| _(none yet)_ | —        |
