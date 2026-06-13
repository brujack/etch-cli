# DEBIAN_FRONTEND Propagation Implementation Plan

> **Status: DONE** — merged as etch-cli#108 (2026-06-13)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `DEBCONF_NONINTERACTIVE_SEEN=true` and `NEEDRESTART_MODE=a` to every apt invocation so dpkg post-invoke hooks run non-interactively.

**Architecture:** One function change in `Aptitude::env()`. The existing `elevate_if_required()` in `exec.rs` already injects all environment entries as `env KEY=VAL` arguments before the apt command when running via sudo — no other files need changing.

**Tech Stack:** Rust, `lib/src/actions/package/providers/aptitude.rs`.

---

## File Map

| File                                            | Change                           |
| ----------------------------------------------- | -------------------------------- |
| `lib/src/actions/package/providers/aptitude.rs` | Expand `env()` + add 1 unit test |

---

## Task 1: Expand `Aptitude::env()` and add test

**Files:**

- Modify: `lib/src/actions/package/providers/aptitude.rs:18-24` (env function)
- Test: `lib/src/actions/package/providers/aptitude.rs` (append to existing `#[cfg(test)]` block)

- [ ] **Step 1: Write the failing test**

Append inside the `mod test { ... }` block at the bottom of
`lib/src/actions/package/providers/aptitude.rs` (before the closing `}`):

```rust
    #[test]
    fn env_contains_all_noninteractive_vars() {
        let apt = Aptitude {};
        let env = apt.env();
        let map: std::collections::HashMap<&str, &str> =
            env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(map.get("DEBIAN_FRONTEND"), Some(&"noninteractive"), "DEBIAN_FRONTEND must be noninteractive");
        assert_eq!(map.get("DEBCONF_NONINTERACTIVE_SEEN"), Some(&"true"), "DEBCONF_NONINTERACTIVE_SEEN must be true");
        assert_eq!(map.get("NEEDRESTART_MODE"), Some(&"a"), "NEEDRESTART_MODE must be a");
    }
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo nextest run -p etch-lib -E 'test(env_contains_all_noninteractive_vars)' 2>&1 | tail -10
```

Expected: `FAIL` — the test sees only `DEBIAN_FRONTEND` (the other two keys are absent).

- [ ] **Step 3: Expand `Aptitude::env()`**

Replace the current `env()` body (lines 18–24 of `aptitude.rs`):

```rust
    fn env(&self) -> Vec<(String, String)> {
        vec![(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        )]
    }
```

With:

```rust
    fn env(&self) -> Vec<(String, String)> {
        vec![
            (String::from("DEBIAN_FRONTEND"), String::from("noninteractive")),
            (String::from("DEBCONF_NONINTERACTIVE_SEEN"), String::from("true")),
            (String::from("NEEDRESTART_MODE"), String::from("a")),
        ]
    }
```

- [ ] **Step 4: Run the new test and the full suite**

```bash
cargo nextest run -p etch-lib -E 'test(env_contains_all_noninteractive_vars)' 2>&1 | tail -5
cargo nextest run -p etch-lib 2>&1 | tail -5
```

Expected: all pass. The existing `apt_version_step_*` tests pass unchanged because they already pass `env` through — the extra vars ride along without breaking anything.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/package/providers/aptitude.rs
git commit -m "feat(apt): propagate NEEDRESTART_MODE and DEBCONF_NONINTERACTIVE_SEEN"
```

---

## Self-Review

**Spec coverage:**

- ✅ `DEBIAN_FRONTEND=noninteractive` — already present, unchanged
- ✅ `DEBCONF_NONINTERACTIVE_SEEN=true` — added in Task 1
- ✅ `NEEDRESTART_MODE=a` — added in Task 1
- ✅ Unit test asserting all three vars — added in Task 1
- ✅ Existing `apt_version_step_*` tests validate env flows into steps — no gap

**Placeholder scan:** None.

**Type consistency:** `Vec<(String, String)>` matches the existing `env()` return type and all call sites (`self.env()` → `environment: self.env()`).

---

> _Docs status update (Pending→Done in `docs/superpowers/README.md`) must happen on main after the PR merges — not inside the worktree._
