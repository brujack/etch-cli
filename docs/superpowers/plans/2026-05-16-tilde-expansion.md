# Tilde Expansion in manifest_paths Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `~` expand to the user's home directory in `manifest_paths` entries in `etch.yaml`.

**Architecture:** Add `shellexpand = "3"` to `lib/Cargo.toml`, then call `shellexpand::tilde(url)` before `PathBuf::from` in `LocalManifestProvider::resolve`. One dependency, one function call, two new tests.

**Tech Stack:** Rust, `shellexpand` crate (v3), existing `ManifestProvider` trait

---

## Files

| File                                      | Change                                        |
| ----------------------------------------- | --------------------------------------------- |
| `lib/Cargo.toml`                          | Add `shellexpand = "3"` to `[dependencies]`   |
| `lib/src/manifests/providers/local.rs`    | Call `shellexpand::tilde` before canonicalize |
| `~/git-repos/personal/dotfiles/etch.yaml` | Replace absolute path with `~/...`            |

---

### Task 1: Add shellexpand, fix local.rs, add tests

**Files:**

- Modify: `lib/Cargo.toml`
- Modify: `lib/src/manifests/providers/local.rs`

- [ ] **Step 1: Write two failing tests**

Add inside the existing `#[cfg(test)] mod test { ... }` block in `lib/src/manifests/providers/local.rs`, after the last existing test:

```rust
    #[test]
    fn test_resolve_tilde_expands_home() {
        let provider = LocalManifestProvider;
        let home = std::env::var("HOME").unwrap();
        // "~" alone should resolve to the home directory (which exists)
        assert_eq!(
            std::path::PathBuf::from(&home).canonicalize().unwrap(),
            provider.resolve("~").unwrap()
        );
    }

    #[test]
    fn test_resolve_tilde_nonexistent_path() {
        let provider = LocalManifestProvider;
        // Tilde expands but path doesn't exist → NoResolution
        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            provider.resolve("~/etch-test-nonexistent-path-xyz-abc")
        );
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p etch-lib test_resolve_tilde 2>&1 | tail -10
```

Expected: compile error or FAIL — `shellexpand` not in scope yet.

- [ ] **Step 3: Add shellexpand to `lib/Cargo.toml`**

Find the `[dependencies]` section and add:

```toml
shellexpand = "3"
```

- [ ] **Step 4: Implement tilde expansion in `lib/src/manifests/providers/local.rs`**

Change `resolve` from:

```rust
    fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError> {
        PathBuf::from(url)
            .canonicalize()
            .map_err(|_| ManifestProviderError::NoResolution)
    }
```

to:

```rust
    fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError> {
        let expanded = shellexpand::tilde(url);
        PathBuf::from(expanded.as_ref())
            .canonicalize()
            .map_err(|_| ManifestProviderError::NoResolution)
    }
```

- [ ] **Step 5: Run the new tests to confirm they pass**

```bash
cargo test -p etch-lib test_resolve_tilde 2>&1 | tail -10
```

Expected: both tests PASS.

- [ ] **Step 6: Run the full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add lib/Cargo.toml lib/src/manifests/providers/local.rs
git commit -m "feat: expand ~ in manifest_paths via shellexpand

manifest_paths entries in etch.yaml can now use ~/... paths.
Uses shellexpand::tilde (tilde-only, not full env var expansion).

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Update dotfiles etch.yaml + docs

**Files:**

- Modify: `~/git-repos/personal/dotfiles/etch.yaml`
- Modify: `docs/superpowers/README.md` (post-merge on main, not in worktree)

- [ ] **Step 1: Update dotfiles etch.yaml to use tilde path**

In `~/git-repos/personal/dotfiles/etch.yaml`, replace:

```yaml
# etch resolves manifest_paths via PathBuf::canonicalize() with no tilde
# expansion, so an absolute path is required here. On Linux, update this
# path to /home/<user>/git-repos/personal/dotfiles/manifests, or wait for
# the shellexpand fix tracked in the etch-cli backlog.
manifest_paths:
    - /Users/bruce/git-repos/personal/dotfiles/manifests
```

with:

```yaml
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests
```

- [ ] **Step 2: Verify the updated etch.yaml works**

```bash
etch --config ~/git-repos/personal/dotfiles/etch.yaml apply --dry-run 2>&1 | head -5
```

Expected: etch finds the manifests without error (dry-run output, not "No manifest paths found").

- [ ] **Step 3: Commit etch.yaml to dotfiles**

```bash
cd ~/git-repos/personal/dotfiles
git add etch.yaml
git commit -m "feat(etch): use tilde path in etch.yaml — shellexpand fix merged

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```

- [ ] **Step 4: Update docs/superpowers/README.md on main (post-merge)**

After the etch-cli PR merges and main is pulled, change the `tilde-expansion` row from `Pending` to `Done` in `docs/superpowers/README.md`.

```bash
# After PR merges and git pull:
git add docs/superpowers/README.md
git commit -m "docs: mark tilde-expansion Done"
git push origin main
```
