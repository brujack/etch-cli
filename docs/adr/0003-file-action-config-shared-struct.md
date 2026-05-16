# ADR-0003: FileActionConfig Shared Struct for File Action Privileged Support

**Date:** 2026-05-15
**Status:** Accepted

## Context

Adding `privileged: bool` / `sudo: true` to file actions (`file.chmod`, `file.chown`, `file.link`, `file.copy`) required a mechanism to enforce that all future file actions include the field. Without enforcement, new file actions could silently omit privilege escalation support.

Options considered:

1. Add `privileged: bool` to each action struct independently — requires manual discipline, no compiler enforcement.
2. Add `privileged` to `ConditionalVariantAction<T>` — generic wrapper sees it but can't dispatch action-specific sudo behavior.
3. Shared `FileActionConfig` struct embedded via `#[serde(flatten)]` + required `FileAction` trait method — compiler enforces it.

## Decision

Create `FileActionConfig { privileged: bool }` in `lib/src/actions/file/mod.rs`. Add a required `fn file_action_config(&self) -> &FileActionConfig` method to the `FileAction` trait. Every file action embeds `FileActionConfig` via `#[serde(flatten)]` and implements the trait method.

The compiler makes it impossible to create a file action without implementing the method — forgetting to support privilege escalation produces a compile error, not a silent omission.

Actions that don't yet support privileged mode (`file.remove`, `file.download`, `file.unarchive`) implement the method and return `Err("file.X does not support privileged mode")` from `plan()` when `privileged: true`.

## Consequences

- Compile-time guarantee that all file actions expose a `privileged` field in YAML.
- Clippy flags the trait method as `dead_code` (it exists for convention enforcement, not immediate callers) — suppressed with `#[allow(dead_code)]` and a comment explaining why.
- Non-Homebrew providers silently ignore `cask: true` on `package.install` (a parallel decision) because `cask` is on the package struct, not on `FileActionConfig` — the file action pattern doesn't apply to package actions.

## Related

- [File action privileged spec](../superpowers/specs/2026-05-15-file-action-privileged-design.md)
- [File action privileged plan](../superpowers/plans/2026-05-15-file-action-privileged.md)
