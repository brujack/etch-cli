# package.install cask Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `cask: bool` field to `package.install` so Homebrew cask installs can be declared as `cask: true` instead of `extra_args: ["--cask"]`.

**Architecture:** Add `cask: bool` to `Package` and `PackageVariant` in `lib/src/actions/package/mod.rs`. Include it in the `From<&Package>` conversion. In `lib/src/actions/package/providers/homebrew.rs`, insert `--cask` into the `install()` args when `package.cask` is true. `query()` needs no changes — it already checks `Caskroom/` as well as `Cellar/`.

**Tech Stack:** Rust, serde, existing `Exec` atom, `anyhow`

---

## Files

| File                                            | Change                                                                      |
| ----------------------------------------------- | --------------------------------------------------------------------------- |
| `lib/src/actions/package/mod.rs`                | Add `cask: bool` to `Package` and `PackageVariant`; update `From<&Package>` |
| `lib/src/actions/package/providers/homebrew.rs` | Insert `--cask` in `install()` when `cask: true`                            |
| `lib/src/actions/package/install.rs`            | Add cask deserialization test                                               |

---

### Task 1: Add `cask` field to Package/PackageVariant and tests

**Files:**

- Modify: `lib/src/actions/package/mod.rs`
- Modify: `lib/src/actions/package/install.rs`

- [ ] **Step 1: Write failing tests in `lib/src/actions/package/mod.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn package_variant_from_package_with_cask() {
        let pkg = Package {
            name: Some(String::from("alfred")),
            cask: true,
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert!(variant.cask);
        assert_eq!(variant.packages(), vec!["alfred"]);
    }

    #[test]
    fn package_variant_from_package_cask_overridden_by_variant() {
        // A variant's cask value overrides the base (same as provider)
        let os = os_info::get();
        let mut variants = std::collections::HashMap::new();
        variants.insert(
            os.os_type(),
            PackageVariant {
                name: Some(String::from("variant-cask")),
                cask: true,
                ..Default::default()
            },
        );
        let pkg = Package {
            name: Some(String::from("base-formula")),
            cask: false,
            variants,
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert!(variant.cask, "variant cask:true should override base cask:false");
        assert_eq!(variant.packages(), vec!["variant-cask"]);
    }
```

- [ ] **Step 2: Write failing cask deserialization test in `lib/src/actions/package/install.rs`**

Find the existing `#[cfg(test)] mod tests { ... }` block and add:

```rust
    #[test]
    fn it_can_be_deserialized_with_cask() {
        use crate::actions::Actions;
        let yaml = r#"
- action: package.install
  name: alfred
  provider: homebrew
  cask: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!("alfred", action.action.name.unwrap());
                assert!(action.action.cask);
            }
            _ => panic!("PackageInstall didn't deserialize correctly"),
        }
    }
```

- [ ] **Step 3: Confirm tests fail**

```bash
cargo test -p etch-lib package_variant_from_package_with_cask package_variant_from_package_cask_overridden_by_variant it_can_be_deserialized_with_cask 2>&1 | tail -5
```

Expected: compile error — `Package` has no field `cask`.

- [ ] **Step 4: Add `cask` to `Package` in `lib/src/actions/package/mod.rs`**

Find the `file: bool` field in `Package` (around line 37) and add `cask` immediately after:

```rust
    #[serde(default)]
    file: bool,

    #[serde(default)]
    cask: bool,
```

No `pub` needed — all fields in `Package` are private (module-level visibility). Tests in `install.rs` are in a child module of `package`, so they can access private fields of the parent module within the same crate.

- [ ] **Step 5: Add `cask` to `PackageVariant` in `lib/src/actions/package/mod.rs`**

Find the `file: bool` field in `PackageVariant` (around line 54) and add `cask` immediately after:

```rust
    #[serde(default)]
    file: bool,

    #[serde(default)]
    cask: bool,
```

- [ ] **Step 6: Update `From<&Package>` to include `cask`**

The `From<&Package>` impl has two branches. Update both:

**No-variant branch** (around line 74–82) — change from:

```rust
        return PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: package.provider.clone(),
            extra_args: package.extra_args.clone(),
            file: package.file,
        };
```

to:

```rust
        return PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: package.provider.clone(),
            extra_args: package.extra_args.clone(),
            file: package.file,
            cask: package.cask,
        };
```

**With-variant branch** (around line 89–95) — change from:

```rust
        let mut package = PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: variant.provider.clone(),
            extra_args: variant.extra_args.clone(),
            file: package.file,
        };
```

to:

```rust
        let mut package = PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: variant.provider.clone(),
            extra_args: variant.extra_args.clone(),
            file: package.file,
            cask: variant.cask,   // variant overrides base (same as provider)
        };
```

- [ ] **Step 7: Run the 3 new tests to confirm they pass**

```bash
cargo test -p etch-lib package_variant_from_package_with_cask package_variant_from_package_cask_overridden_by_variant it_can_be_deserialized_with_cask 2>&1 | tail -10
```

Expected: 3 tests PASS.

- [ ] **Step 8: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/package/mod.rs lib/src/actions/package/install.rs
git commit -m "feat: add cask field to Package and PackageVariant

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Use `cask` in Homebrew `install()` and add provider test

**Files:**

- Modify: `lib/src/actions/package/providers/homebrew.rs`

- [ ] **Step 1: Write a failing test in `lib/src/actions/package/providers/homebrew.rs`**

Add inside the existing `#[cfg(test)] mod test { ... }` block:

```rust
    #[test]
    fn install_includes_cask_flag_when_cask_true() {
        let homebrew = Homebrew {};
        // Guard: skip on Linux CI where brew isn't available
        if !homebrew.available() {
            return;
        }
        let pkg = PackageVariant {
            name: Some(String::from("etch-definitely-not-installed-cask-xyz")),
            cask: true,
            ..Default::default()
        };
        let steps = homebrew.install(&pkg, &Contexts::default()).unwrap();
        // If the package were already installed, steps would be empty.
        // Using an unlikely name so this is safe.
        if steps.is_empty() {
            return;
        }
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--cask"),
            "expected '--cask' in brew install args: {display}"
        );
    }

    #[test]
    fn install_excludes_cask_flag_when_cask_false() {
        let homebrew = Homebrew {};
        if !homebrew.available() {
            return;
        }
        let pkg = PackageVariant {
            name: Some(String::from("etch-definitely-not-installed-formula-xyz")),
            cask: false,
            ..Default::default()
        };
        let steps = homebrew.install(&pkg, &Contexts::default()).unwrap();
        if steps.is_empty() {
            return;
        }
        let display = steps[0].atom.to_string();
        assert!(
            !display.contains("--cask"),
            "did not expect '--cask' in brew install args: {display}"
        );
    }
```

- [ ] **Step 2: Run tests to confirm they fail (on macOS) or skip (on Linux)**

```bash
cargo test -p etch-lib install_includes_cask_flag_when_cask_true install_excludes_cask_flag_when_cask_false 2>&1 | tail -10
```

Expected on macOS: compile error (no `cask` field on `PackageVariant` yet) — wait, Task 1 already added the field. So: 1 test PASS (the false case trivially returns with no `--cask`), 1 test FAIL (the true case doesn't include `--cask` yet since we haven't changed `install()`).

- [ ] **Step 3: Modify `install()` in `lib/src/actions/package/providers/homebrew.rs`**

Change the `install()` method from:

```rust
    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // Does not require privilege escalation

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: [
                    vec![String::from("install")],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
```

to:

```rust
    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // Does not require privilege escalation

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        let mut base = vec![String::from("install")];
        if package.cask {
            base.push(String::from("--cask"));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: [base, package.extra_args.clone(), need_installed].concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
```

- [ ] **Step 4: Run the new tests**

```bash
cargo test -p etch-lib install_includes_cask_flag_when_cask_true install_excludes_cask_flag_when_cask_false 2>&1 | tail -10
```

Expected: both PASS (or both return early if brew isn't available, which is also a pass).

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/package/providers/homebrew.rs
git commit -m "feat: pass --cask to brew install when cask:true

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Docs update (post-merge on main)

Per the worktree-docs-conflict pattern: do docs updates directly on main after the PR merges.

- [ ] **Step 1: After PR merges, mark spec Done in README**

Change the `package-install-cask` row from `Pending` to `Done`.

- [ ] **Step 2: Update CLAUDE.md action catalog**

Update the `package.install` row to note the `cask` field:

```markdown
| `package.install` | Install OS packages | `name` (single) or `list` (multiple); `provider` (`apt`, `snap`, `brew`); `cask` (bool, Homebrew only) |
```

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md CLAUDE.md
git commit -m "docs: mark package-install-cask Done; update action catalog"
git push origin main
```
