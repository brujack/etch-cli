# etch update --only/--skip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 10 individual per-category bool flags on `etch update` with `--only <categories>` and `--skip <categories>` comma-separated filter flags.

**Architecture:** Add `only: Vec<String>` and `skip: Vec<String>` to `Update` while keeping the bool fields, implement and test `should_run()`/`validate_categories()`, then remove the old fields and dead code in a second pass.

**Tech Stack:** Rust, clap (already in use), anyhow (already in use)

---

### Task 1: Add fields, VALID_CATEGORIES, and selection/validation logic (TDD)

**Files:**

- Modify: `app/src/commands/update.rs`

- [ ] **Step 1: Add `only` and `skip` fields to `Update` struct**

In `app/src/commands/update.rs`, find the `pub(crate) struct Update` block (around line 69) and add these two fields at the end, before the closing `}`:

```rust
    /// Run only these categories (comma-separated: brew,rust)
    #[arg(long, value_delimiter = ',', conflicts_with = "skip")]
    pub only: Vec<String>,

    /// Skip these categories (comma-separated: pip,gems)
    #[arg(long, value_delimiter = ',', conflicts_with = "only")]
    pub skip: Vec<String>,
```

`value_delimiter = ','` makes clap split `--only brew,rust` into `["brew", "rust"]`. `conflicts_with = "skip"` / `conflicts_with = "only"` makes clap error if both are provided.

- [ ] **Step 2: Add `VALID_CATEGORIES` const**

Add this const immediately before the `impl Update` block (around line 103):

```rust
const VALID_CATEGORIES: &[&str] = &[
    "brew", "system", "mas", "claude", "packages",
    "pip", "rust", "git-tools", "gems", "cheatsh",
];
```

- [ ] **Step 3: Write failing tests**

In the `#[cfg(test)] mod tests` block at the bottom of the file, add:

```rust
    #[test]
    fn should_run_all_when_no_filter() {
        let u = Update::default();
        for cat in VALID_CATEGORIES {
            assert!(u.should_run(cat), "expected {cat} to run with no filter");
        }
    }

    #[test]
    fn should_run_only_selected() {
        let u = Update {
            only: vec!["brew".to_string(), "rust".to_string()],
            ..Default::default()
        };
        assert!(u.should_run("brew"));
        assert!(u.should_run("rust"));
        assert!(!u.should_run("pip"));
    }

    #[test]
    fn should_run_skips_excluded() {
        let u = Update {
            skip: vec!["pip".to_string(), "gems".to_string()],
            ..Default::default()
        };
        assert!(u.should_run("brew"));
        assert!(!u.should_run("pip"));
        assert!(!u.should_run("gems"));
    }

    #[test]
    fn validate_categories_accepts_valid() {
        let u = Update {
            only: vec!["brew".to_string(), "rust".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_ok());
    }

    #[test]
    fn validate_categories_rejects_unknown_in_only() {
        let u = Update {
            only: vec!["foobar".to_string()],
            ..Default::default()
        };
        let err = u.validate_categories().unwrap_err();
        assert!(err.to_string().contains("unknown category 'foobar'"));
        assert!(err.to_string().contains("valid:"));
    }

    #[test]
    fn validate_categories_rejects_mixed_valid_invalid() {
        let u = Update {
            only: vec!["brew".to_string(), "badname".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_err());
    }

    #[test]
    fn validate_categories_rejects_unknown_in_skip() {
        let u = Update {
            skip: vec!["notreal".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_err());
    }
```

- [ ] **Step 4: Run tests, confirm they fail**

```bash
cargo test -p etch-cli should_run validate_categories 2>&1 | tail -20
```

Expected: compile errors — `should_run` and `validate_categories` not found.

- [ ] **Step 5: Add `should_run` and `validate_categories` to `impl Update`**

Inside the existing `impl Update { ... }` block (after `any_flag_set`), add:

```rust
    fn should_run(&self, category: &str) -> bool {
        if !self.only.is_empty() {
            self.only.iter().any(|c| c == category)
        } else if !self.skip.is_empty() {
            !self.skip.iter().any(|c| c == category)
        } else {
            true
        }
    }

    fn validate_categories(&self) -> anyhow::Result<()> {
        let all = self.only.iter().chain(self.skip.iter());
        for name in all {
            if !VALID_CATEGORIES.contains(&name.as_str()) {
                anyhow::bail!(
                    "unknown category '{name}'; valid: {}",
                    VALID_CATEGORIES.join(", ")
                );
            }
        }
        Ok(())
    }
```

- [ ] **Step 6: Run tests, confirm they pass**

```bash
cargo test -p etch-cli should_run validate_categories 2>&1 | tail -20
```

Expected: 7 tests pass.

- [ ] **Step 7: Run full suite**

```bash
make test
```

Expected: all tests pass (existing bool-field tests still compile since those fields still exist).

- [ ] **Step 8: Commit**

```bash
git add app/src/commands/update.rs
git commit -m "feat(update): add should_run/validate_categories with --only/--skip fields"
```

---

### Task 2: Migrate call sites, remove dead code and old tests

**Files:**

- Modify: `app/src/commands/update.rs`

- [ ] **Step 1: Update `execute()` — add validation, remove `run_all`, replace call sites**

In the `#[cfg(not(tarpaulin_include))] fn execute` method (around line 870):

**Replace** this line:

```rust
        let run_all = !self.any_flag_set();
```

**with:**

```rust
        self.validate_categories()?;
```

This puts validation first, exactly where `run_all` was.

**Replace** all 10 `step_should_run` calls with `self.should_run`:

| Old                                           | New                               |
| --------------------------------------------- | --------------------------------- |
| `if step_should_run(self.git_tools, run_all)` | `if self.should_run("git-tools")` |
| `if step_should_run(self.brew, run_all)`      | `if self.should_run("brew")`      |
| `if step_should_run(self.mas, run_all)`       | `if self.should_run("mas")`       |
| `if step_should_run(self.claude, run_all)`    | `if self.should_run("claude")`    |
| `if step_should_run(self.rust, run_all)`      | `if self.should_run("rust")`      |
| `if step_should_run(self.packages, run_all)`  | `if self.should_run("packages")`  |
| `if step_should_run(self.system, run_all)`    | `if self.should_run("system")`    |
| `if step_should_run(self.pip, run_all)`       | `if self.should_run("pip")`       |
| `if step_should_run(self.gems, run_all)`      | `if self.should_run("gems")`      |
| `if step_should_run(self.cheatsh, run_all)`   | `if self.should_run("cheatsh")`   |

- [ ] **Step 2: Remove the 10 bool fields from `Update` struct**

Replace the entire `pub(crate) struct Update { ... }` block with:

```rust
#[derive(Parser, Debug, Default)]
pub(crate) struct Update {
    /// Run only these categories (comma-separated: brew,rust)
    #[arg(long, value_delimiter = ',', conflicts_with = "skip")]
    pub only: Vec<String>,

    /// Skip these categories (comma-separated: pip,gems)
    #[arg(long, value_delimiter = ',', conflicts_with = "only")]
    pub skip: Vec<String>,
}
```

- [ ] **Step 3: Remove `any_flag_set()` from `impl Update`**

Delete the entire `fn any_flag_set` method from `impl Update`:

```rust
    fn any_flag_set(&self) -> bool {
        self.brew
            || self.system
            || self.mas
            || self.claude
            || self.packages
            || self.pip
            || self.rust
            || self.git_tools
            || self.gems
            || self.cheatsh
    }
```

- [ ] **Step 4: Remove `step_should_run` free function**

Delete this function entirely:

```rust
fn step_should_run(flag: bool, run_all: bool) -> bool {
    flag || run_all
}
```

- [ ] **Step 5: Delete the 7 old unit tests**

In `mod tests`, delete these test functions:

- `any_flag_set_false_when_all_default`
- `any_flag_set_true_with_brew`
- `any_flag_set_true_with_cheatsh`
- `any_flag_set_true_with_git_tools`
- `step_should_run_when_run_all`
- `step_should_run_when_flag_set`
- `step_should_not_run_when_neither`

- [ ] **Step 6: Compile check**

```bash
cargo check -p etch-cli 2>&1 | tail -20
```

Expected: no errors. Fix any remaining references to removed fields or functions.

- [ ] **Step 7: Run full test suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add app/src/commands/update.rs
git commit -m "feat(update): replace per-category flags with --only/--skip"
```

---

### Task 3: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin HEAD
```

- [ ] **Step 2: Create PR**

```bash
gh pr create --repo brujack/etch-cli \
  --title "feat(update): replace per-category flags with --only/--skip" \
  --body "$(cat <<'EOF'
## Summary
- Drops 10 individual bool flags (--brew, --rust, etc.) from \`etch update\`
- Adds \`--only <categories>\` and \`--skip <categories>\` (comma-separated)
- Default (no flags) continues to run all categories

## Test Plan
- [ ] \`cargo test -p etch-cli\` passes
- [ ] \`etch update --only brew,rust\` runs only brew and rust steps
- [ ] \`etch update --skip pip\` runs all except pip
- [ ] \`etch update --only foo\` errors with "unknown category 'foo'"
- [ ] \`etch update --only brew --skip pip\` errors (conflicting args)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli --watch
```

Fix any failures before proceeding.

- [ ] **Step 4: After merge — update plan index** _(do this on main, not in worktree)_

In `docs/superpowers/README.md`, update the `etch-update-only-skip` row:

- Set status to `Done`
- Add plan file link

Add `> **Status: DONE**` banner at the top of this plan file.
