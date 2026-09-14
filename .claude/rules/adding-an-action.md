---
paths:
    - "lib/src/actions/**"
    - "examples/**/*.yaml"
---

# Adding a new action

Full workflow: the `etch-cli-new-action` skill. Field reference for every existing action:
`~/git-repos/personal/ai-config/docs/knowledge/etch-cli-action-catalog.md`. This file is the
registration checklist and the traps that cost a cycle each.

Every new action requires changes in exactly these places:

1. **Create `lib/src/actions/<name>/install.rs`** — the action struct + `impl Action`
2. **Create `lib/src/actions/<name>/mod.rs`** — re-export: `mod install; pub use install::ActionType;`
3. **Register in `lib/src/actions/mod.rs`** (6 edits):
    - `mod <name>;` — module declaration
    - `use <name>::ActionType;` — import
    - Enum variant with serde rename: `ActionType(ConditionalVariantAction<ActionType>)` + `#[serde(rename = "name.action")]`
    - Match arm in `inner_ref()` impl
    - Match arm in `notify` accessor
    - Match arm in `Deref` impl
    - Match arm in `Display` impl (`=> "name.action"`)
4. **Update the three test YAML lists** in `all_major_action_variants_can_be_deserialized`,
   `all_action_variants_inner_ref_and_deref`, and `all_action_variants_display` — add a YAML
   entry for the new action to each, update the action count in the `assert_eq!`, and add a
   `names.contains` assertion in the display test
5. **Add `examples/<name>/<name>-install.yaml`** with one entry per option combination
6. **Update the action table in `README.md`** and `etch-cli-action-catalog.md` in ai-config

Missing any step produces a compile error (missing match arm) or a test failure (incorrect
variant count).

**YAML names come from `#[serde(rename = "...")]`, not from Rust struct names.** When
checking docs against the implementation, grep `lib/src/actions/mod.rs` for the rename
annotations — struct names and YAML names diverge (struct `GroupAdd` → YAML `group.add`).

**TDD stub behavior:** `all_action_variants_inner_ref_and_deref` calls `inner.summarize()` on
every registered variant in a loop. If the new action's `summarize()` is `todo!()`, that
dispatch test panics too — expect N+1 failures (N unit tests + 1 dispatch test) in the RED
phase, not just N. All are correct TDD RED state.

**Editing match arms: use `replace_all: true` for identical `=> a` patterns.** The
`inner_ref`, `notify` and `Deref` match blocks all contain arms like
`Actions::MacOSDefault(a) => a,` — identical structure across blocks — so the Edit tool
refuses with "Found 2 matches" when you target a common pattern. Either pass
`replace_all: true` when both blocks need the same addition, or include enough unique
surrounding context (the action above or below the insertion point) to disambiguate.

**`semver-check` always fails advisory on a new action.** A new enum variant is
`enum_variant_added`, semver-breaking by the spec. That job is `continue-on-error: true` and
absent from `auto-merge`'s `needs:`, so it never blocks the PR.
