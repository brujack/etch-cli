# Dead Code Removal — Design Spec

**Date:** 2026-05-04
**Status:** Accepted

## Context

Five areas of confirmed dead code identified via coverage analysis:

1. **`values/mod.rs` NaN branches** — `NumberVariant::total_cmp()` has unreachable branches in cross-type comparisons (Signed↔Float, Unsigned↔Float, Float↔Signed, Float↔Unsigned). The `!(integer as f64).is_nan()` guards are always true because integers can never be NaN. The `else if` and `else Ordering::Equal` arms are dead. The code has FIXME comments pointing to `f64::total_cmp()` (stable since Rust 1.62).

2. **`values/mod.rs` impossible None branches** — `TryFrom<JsonValue>` for numbers has `match number.as_u64()/as_i64()/as_f64()` with `None => Err(...)` arms that can never fire: if `is_u64()` returns true, `as_u64()` always returns `Some`.

3. **`contexts/mod.rs` ListContext arm** — `build_contexts()` has a `Context::ListContext` match arm that is dead. No provider returns `ListContext`; all return `KeyValueContext`. The `Context::ListContext` variant exists but is never produced.

4. **`manifests/providers/git.rs` unused GitConfig fields** — `GitConfig.branch` and `GitConfig.path` are parsed by `parse_config_url()` but never read by `fetch_and_clone()` or `resolve()`. The git clone always uses the default branch.

5. **`utilities/lua.rs` unreachable object-table branch** — `lua_value_to_json()` tries to detect string-keyed tables using `pairs::<i64, LuaValue>().count() == 0`, but this count is never 0 for non-empty tables because conversion errors are counted too. The "treat as object" branch (lines 35-42) is unreachable and the detection logic is wrong.

## Decision

Remove or fix each item in order of safety:

- Items 1-2: Simplify to remove dead branches, behavior unchanged
- Item 3: Remove `ListContext` variant from `Context` enum (no callers produce it)
- Item 4: Remove unused fields from `GitConfig` (no external API)
- Item 5: Fix the object detection logic using `sequence_values()` instead of `pairs::<i64>()` count

## Step-by-Step Plan

1. **`values/mod.rs` NaN branches** — Replace each `partial_cmp(&b).unwrap_or_else(...)` cross-type arm with a simpler expression using `f64::total_cmp()`. Remove the FIXME comments.

2. **`values/mod.rs` TryFrom None branches** — Replace `match number.as_u64() { Some(n) => n.into(), None => Err(...) }` with `n.into()` directly using `unwrap_or_else` with a message, since the None path is provably unreachable.

3. **`contexts/mod.rs` ListContext** — Remove `ListContext` arm from `build_contexts()` match. Decide whether to keep the variant in the enum for future use (document it) or remove it.

4. **`manifests/providers/git.rs`** — Remove `branch` and `path` fields from `GitConfig`. Add a TODO comment noting these are parsed but not yet implemented.

5. **`utilities/lua.rs` object detection** — Write a failing test first (TDD), then fix the detection by replacing `pairs::<i64, LuaValue>().count() == 0` with `t.sequence_values::<LuaValue>().count() == 0` which correctly identifies non-sequential tables.

## Risk Assessment

- Items 1-2: No behavior change; existing `nan_comparison_tests` and `try_from_json_value_tests` verify correctness.
- Item 3: If a future provider returns `ListContext`, `build_contexts()` will silently drop it. Document this.
- Item 4: `parse_config_url()` still parses branch/path (useful once implemented); removing fields means they're discarded — benign.
- Item 5: Behavioral change — string-keyed Lua tables will now correctly serialize to JSON objects. New test required first.

## Consequences

- ~30 lines of dead code removed
- `values/mod.rs` FIXME comments resolved
- `lua_value_to_json()` correctly handles string-keyed tables
- Coverage ceiling raised slightly (previously unreachable branches removed)
