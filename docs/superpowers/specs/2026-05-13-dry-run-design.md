# Dry-Run Mode — Design Spec

**Date:** 2026-05-13
**Status:** Approved

## Context

`etch apply --dry-run` already exists as a CLI flag and the core skip-execute guard is in place (`apply.rs:275`). What is missing is output: currently dry-run silently skips execution with no indication of what would have run. A user running `--dry-run` has no way to verify what etch would do.

## Goal

When `--dry-run` is passed, etch should:

- Print a clear banner so the user knows nothing will be changed
- Report each action with a count of steps that would run
- Optionally show individual atom-level descriptions when `-v` is passed
- Leave the system completely unmodified

## Scope

**One file changed:** `app/src/commands/apply.rs`

No changes to the `Atom` trait, `Action` trait, step initializers/finalizers, or any `lib/` code.

## Design

### 1. Banner

Immediately after `let dry_run = self.dry_run;` (line 166), print once per `execute()` call:

```rust
if dry_run {
    println!("[dry run] no changes will be made\n");
}
```

Uses `println!` (not `info!`) so it is always visible regardless of tracing configuration.

### 2. Step loop

Replace the current silent `if dry_run { continue; }` (lines 275–277) with:

```rust
let mut dry_run_count = 0usize;

for mut step in steps {
    if dry_run {
        dry_run_count += 1;
        if runtime.args.verbose > 0 {
            info!("  would: {}", step.atom);
        }
        continue;
    }

    match step.atom.execute() { ... }         // unchanged
    if !step.do_finalizers_allow_us_to_continue() { ... }  // unchanged
}
```

`runtime.args.verbose` is the existing `-v` flag on `GlobalArgs` (`u8`; 0 = default, 1 = `-v`, 2 = `-vv`). Checking `> 0` enables atom-level detail.

### 3. Action summary line

Replace the unconditional `info!("{}", action.summarize())` (line 294) with:

```rust
if dry_run {
    info!("[dry run] {}: {} step(s) would run", action.summarize(), dry_run_count);
} else {
    info!("{}", action.summarize());
}
```

When there is nothing to do, the existing `info!("nothing to be done to reconcile action")` path fires first and the action loop `continue`s — the step count is never reached, so `dry_run_count` stays 0 and this branch is never hit.

### 4. Outer manifest-level dry_run block

The existing `if dry_run { span_manifest.exit(); continue; }` (lines 298–301) is left unchanged. It correctly suppresses the success/failure check and `info!("Completed")` message for the manifest.

## Output Examples

**Default (`etch apply --dry-run`):**

```
[dry run] no changes will be made

[dry run] Link ~/.zshrc to ~/dotfiles/.zshrc: 1 step(s) would run
[dry run] Install packages [git, curl, jq]: 3 step(s) would run
nothing to be done to reconcile action
```

**Verbose (`etch apply --dry-run -v`):**

```
[dry run] no changes will be made

  would: <atom Display string>
[dry run] Link ~/.zshrc to ~/dotfiles/.zshrc: 1 step(s) would run
  would: <atom Display string>
  would: <atom Display string>
  would: <atom Display string>
[dry run] Install packages [git, curl, jq]: 3 step(s) would run
```

Note: atom `Display` string quality varies by implementation. If any are unclear, that is a follow-up improvement — not in scope here.

## Testing

Integration tests in `app/tests/basic_usage.rs` using `assert_cmd` against the `etch` binary with fixture manifests.

| Test                           | Assertion                                                                                     |
| ------------------------------ | --------------------------------------------------------------------------------------------- |
| `dry_run_prints_banner`        | stdout contains `[dry run] no changes will be made`; linked file does not exist after run     |
| `dry_run_shows_action_summary` | stdout contains `[dry run]` and `step(s) would run` for an action with work to do             |
| `dry_run_verbose_shows_atoms`  | `--dry-run -v` stdout contains `  would:` lines                                               |
| `dry_run_nothing_to_do`        | manifest already in desired state; `nothing to be done` appears; no `step(s) would run` lines |

## What is NOT in scope

- Changes to `Atom`, `Action`, `Initializer`, or `Finalizer` traits
- A dedicated `--verbose` flag on the `Apply` subcommand (the existing global `-v` is used)
- Structured (JSON/table) dry-run output
- Initializer/finalizer dry-run awareness (they may have read-only side effects; acceptable)
