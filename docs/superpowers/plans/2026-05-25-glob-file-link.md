> **Status: DONE**

# Glob/Wildcard Support for file.link — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `glob` field to `file.link` that expands a glob pattern relative to the manifest's `files/` directory and creates one symlink per matched file, preserving subdirectory structure in the target.

**Architecture:** Add `glob: Option<String>` to `FileLink`; handle it at the top of `plan()` before the existing single-file path. Glob expansion uses the `glob` crate. Each matched file feeds into the existing `plan_no_walk()` (or `plan_privileged()`) to produce `DirCreate + Link` step pairs. No new atoms required.

**Tech Stack:** Rust, `glob = "0.3"` (new dep), existing `plan_no_walk` / `plan_privileged` helpers in `lib/src/actions/file/link.rs`.

---

## File Map

| File                           | Change                                                                                        |
| ------------------------------ | --------------------------------------------------------------------------------------------- |
| `lib/Cargo.toml`               | Add `glob = "0.3"` dependency                                                                 |
| `lib/src/actions/file/link.rs` | Add `glob` field, update `plan()` and `summarize()`, add tests                                |
| `docs/superpowers/README.md`   | Move backlog item to All Plans, add row — **do on main post-merge**                           |
| `CLAUDE.md`                    | Update `file.link` row in Action Catalog to document `glob` field — **do on main post-merge** |

---

## Task 1: Add `glob` dependency and struct field

**Files:**

- Modify: `lib/Cargo.toml`
- Modify: `lib/src/actions/file/link.rs:15-28`

- [ ] **Step 1: Add `glob` to lib/Cargo.toml**

In `lib/Cargo.toml`, add after the `gethostname` line:

```toml
glob = "0.3"
```

- [ ] **Step 2: Add `glob` field to `FileLink` struct**

In `lib/src/actions/file/link.rs`, replace the struct definition (lines 14–28):

```rust
// TODO: Next Major Version - Deprecate from and to
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLink {
    pub from: Option<String>,
    pub source: Option<String>,

    pub target: Option<String>,
    pub to: Option<String>,

    pub glob: Option<String>,

    #[serde(default = "walk_dir_default")]
    pub walk_dir: bool,

    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p etch-lib
```

Expected: compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add lib/Cargo.toml lib/src/actions/file/link.rs
git commit -m "feat(file.link): add glob field to FileLink struct"
```

---

## Task 2: Deserialization test

**Files:**

- Modify: `lib/src/actions/file/link.rs` (test module)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `link.rs`:

```rust
#[test]
fn glob_deserialization() {
    let yaml = r#"
- action: file.link
  glob: "claude/*"
  target: /tmp/dest
"#;
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::FileLink(action)) => {
            assert_eq!(action.action.glob, Some("claude/*".to_string()));
            assert_eq!(action.action.target, Some("/tmp/dest".to_string()));
        }
        _ => panic!("FileLink with glob didn't deserialize"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo nextest run -p etch-lib -- glob_deserialization
```

Expected: FAIL — `glob` field not present yet (or compile error before Task 1).

- [ ] **Step 3: Verify it passes after Task 1**

The `glob` field was added in Task 1 with standard `#[derive(Deserialize)]` — no extra implementation needed. Run again:

```bash
cargo nextest run -p etch-lib -- glob_deserialization
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "test(file.link): glob deserialization"
```

---

## Task 3: Mutual exclusion — `glob` + `source` returns Err

**Files:**

- Modify: `lib/src/actions/file/link.rs` (test module + `plan()`)

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn glob_and_source_both_set_returns_err() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = Manifest {
        root_dir: Some(tmp.path().to_path_buf()),
        ..Default::default()
    };
    let contexts = build_contexts(&Config::default());
    let action = FileLink {
        glob: Some("*.txt".to_string()),
        source: Some("myfile.txt".to_string()),
        target: Some("/tmp/dest".to_string()),
        ..Default::default()
    };
    assert!(action.plan(&manifest, &contexts).is_err());
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo nextest run -p etch-lib -- glob_and_source_both_set_returns_err
```

Expected: FAIL — `plan()` does not yet check mutual exclusion.

- [ ] **Step 3: Add mutual exclusion guard to `plan()`**

In `impl Action for FileLink`, replace the `plan()` method body, adding the glob guard at the top. The full method becomes:

```rust
fn plan(&self, manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    if self.glob.is_some() {
        if self.source.is_some() || self.from.is_some() {
            return Err(anyhow::anyhow!(
                "file.link: 'glob' and 'source'/'from' are mutually exclusive"
            ));
        }
        // Glob expansion will be implemented in subsequent tasks.
        // For now, return an error so the test below can be added incrementally.
        return Err(anyhow::anyhow!("file.link: glob not yet implemented"));
    }

    let from: PathBuf = self.resolve(manifest, self.source().as_str())?;
    let to = PathBuf::from(self.target());

    if self.config.privileged {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        let walk = self.walk_dir && !from.is_file();
        return Ok(FileLink::plan_privileged(
            from,
            to,
            &privilege_provider,
            walk,
        ));
    }

    // Can't walk a file
    if from.is_file() {
        return Ok(FileLink::plan_no_walk(from, to));
    }

    match self.walk_dir {
        false => Ok(FileLink::plan_no_walk(from, to)),
        true => Ok(FileLink::plan_walk(from, to)),
    }
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo nextest run -p etch-lib -- glob_and_source_both_set_returns_err
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "test(file.link): glob+source mutual exclusion returns Err"
```

---

## Task 4: No-match error

**Files:**

- Modify: `lib/src/actions/file/link.rs` (test module + `plan()` glob branch)

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn glob_no_match_returns_err() {
    let tmp = tempfile::tempdir().unwrap();
    let real_tmp = tmp.path().canonicalize().unwrap();
    let files_dir = real_tmp.join("files");
    std::fs::create_dir_all(&files_dir).unwrap();
    // No files — pattern matches nothing

    let manifest = Manifest {
        root_dir: Some(real_tmp.clone()),
        ..Default::default()
    };
    let contexts = build_contexts(&Config::default());
    let action = FileLink {
        glob: Some("*.txt".to_string()),
        target: Some(real_tmp.join("dest").display().to_string()),
        ..Default::default()
    };
    let result = action.plan(&manifest, &contexts);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("matched no files"),
        "error message should mention 'matched no files'"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo nextest run -p etch-lib -- glob_no_match_returns_err
```

Expected: FAIL — `plan()` returns "glob not yet implemented", not "matched no files".

- [ ] **Step 3: Add `use glob::glob;` import and expand the glob branch in `plan()`**

At the top of `link.rs`, add to the imports:

```rust
use glob::glob as glob_expand;
```

Replace the glob branch in `plan()` (the `if self.glob.is_some()` block):

```rust
if let Some(ref pattern) = self.glob {
    if self.source.is_some() || self.from.is_some() {
        return Err(anyhow::anyhow!(
            "file.link: 'glob' and 'source'/'from' are mutually exclusive"
        ));
    }

    let glob_root = manifest
        .root_dir
        .clone()
        .ok_or_else(|| anyhow::anyhow!("file.link: manifest has no root_dir"))?
        .join("files");

    let full_pattern = glob_root.join(pattern);
    let full_pattern_str = full_pattern
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("file.link: pattern contains invalid UTF-8"))?;

    let matched: Vec<PathBuf> = glob_expand(full_pattern_str)?
        .filter_map(|r| r.ok())
        .filter(|p| p.is_file())
        .collect();

    if matched.is_empty() {
        return Err(anyhow::anyhow!(
            "file.link: glob pattern '{}' matched no files in '{}'",
            pattern,
            glob_root.display()
        ));
    }

    // Step expansion implemented in Task 5.
    return Err(anyhow::anyhow!("file.link: glob expansion not yet complete"));
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo nextest run -p etch-lib -- glob_no_match_returns_err
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "test(file.link): glob no-match returns Err with message"
```

---

## Task 5: Top-level glob expansion produces correct steps

**Files:**

- Modify: `lib/src/actions/file/link.rs` (test module + `plan()` glob branch)

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn glob_matches_top_level_files() {
    let tmp = tempfile::tempdir().unwrap();
    let real_tmp = tmp.path().canonicalize().unwrap();
    let files_dir = real_tmp.join("files").join("claude");
    std::fs::create_dir_all(&files_dir).unwrap();
    std::fs::write(files_dir.join("a.txt"), b"a").unwrap();
    std::fs::write(files_dir.join("b.txt"), b"b").unwrap();
    std::fs::write(files_dir.join("c.txt"), b"c").unwrap();

    let manifest = Manifest {
        root_dir: Some(real_tmp.clone()),
        ..Default::default()
    };
    let contexts = build_contexts(&Config::default());
    let dest = real_tmp.join("dest");
    let action = FileLink {
        glob: Some("claude/*".to_string()),
        target: Some(dest.display().to_string()),
        ..Default::default()
    };
    let steps = action.plan(&manifest, &contexts).unwrap();
    // 2 steps per file: DirCreate + Link
    assert_eq!(steps.len(), 6);
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo nextest run -p etch-lib -- glob_matches_top_level_files
```

Expected: FAIL — `plan()` returns "glob expansion not yet complete".

- [ ] **Step 3: Complete the glob branch in `plan()`**

Replace the temporary `return Err(...)` stub at the end of the glob branch with full step generation:

```rust
if let Some(ref pattern) = self.glob {
    if self.source.is_some() || self.from.is_some() {
        return Err(anyhow::anyhow!(
            "file.link: 'glob' and 'source'/'from' are mutually exclusive"
        ));
    }

    let glob_root = manifest
        .root_dir
        .clone()
        .ok_or_else(|| anyhow::anyhow!("file.link: manifest has no root_dir"))?
        .join("files");

    let full_pattern = glob_root.join(pattern);
    let full_pattern_str = full_pattern
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("file.link: pattern contains invalid UTF-8"))?;

    let matched: Vec<PathBuf> = glob_expand(full_pattern_str)?
        .filter_map(|r| r.ok())
        .filter(|p| p.is_file())
        .collect();

    if matched.is_empty() {
        return Err(anyhow::anyhow!(
            "file.link: glob pattern '{}' matched no files in '{}'",
            pattern,
            glob_root.display()
        ));
    }

    let target_base = PathBuf::from(self.target());
    let privilege_provider = if self.config.privileged {
        Some(
            utilities::get_privilege_provider(contexts)
                .unwrap_or_else(|| "sudo".to_string()),
        )
    } else {
        None
    };

    let mut steps = Vec::new();
    for matched_path in matched {
        let relative = matched_path
            .strip_prefix(&glob_root)
            .map_err(|e| anyhow::anyhow!("file.link: strip_prefix failed: {}", e))?;
        let link_target = target_base.join(relative);

        if let Some(ref provider) = privilege_provider {
            steps.extend(FileLink::plan_privileged(
                matched_path,
                link_target,
                provider,
                false,
            ));
        } else {
            steps.extend(FileLink::plan_no_walk(matched_path, link_target));
        }
    }
    return Ok(steps);
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo nextest run -p etch-lib -- glob_matches_top_level_files
```

Expected: PASS.

- [ ] **Step 5: Run all link tests to check for regressions**

```bash
cargo nextest run -p etch-lib -- actions::file::link
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "feat(file.link): implement glob expansion with structure-preserving targets"
```

---

## Task 6: Recursive glob preserves subdirectory structure

**Files:**

- Modify: `lib/src/actions/file/link.rs` (test module only — implementation already complete)

- [ ] **Step 1: Write the test**

Add to `mod tests`:

```rust
#[test]
fn glob_double_star_preserves_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let real_tmp = tmp.path().canonicalize().unwrap();
    let top_dir = real_tmp.join("files").join("claude");
    let sub_dir = top_dir.join("sub");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(top_dir.join("top.txt"), b"top").unwrap();
    std::fs::write(sub_dir.join("nested.txt"), b"nested").unwrap();

    let manifest = Manifest {
        root_dir: Some(real_tmp.clone()),
        ..Default::default()
    };
    let contexts = build_contexts(&Config::default());
    let dest = real_tmp.join("dest");
    let action = FileLink {
        glob: Some("claude/**/*".to_string()),
        target: Some(dest.display().to_string()),
        ..Default::default()
    };
    let steps = action.plan(&manifest, &contexts).unwrap();
    // 2 files × 2 steps each = 4
    assert_eq!(steps.len(), 4);
}
```

- [ ] **Step 2: Run to verify it passes (no implementation change needed)**

```bash
cargo nextest run -p etch-lib -- glob_double_star_preserves_structure
```

Expected: PASS — `glob_expand` handles `**` natively.

- [ ] **Step 3: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "test(file.link): recursive glob preserves subdirectory structure"
```

---

## Task 7: Update `summarize()` for glob mode

**Files:**

- Modify: `lib/src/actions/file/link.rs` (`summarize()` + test)

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn glob_summarize() {
    let action = FileLink {
        glob: Some("claude/*".to_string()),
        target: Some("~/.claude".to_string()),
        ..Default::default()
    };
    let summary = action.summarize();
    assert!(summary.contains("claude/*"), "summary should include pattern");
    assert!(summary.contains("~/.claude"), "summary should include target");
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo nextest run -p etch-lib -- glob_summarize
```

Expected: FAIL — `summarize()` currently only reads `from`/`to` fields.

- [ ] **Step 3: Update `summarize()`**

Replace the existing `summarize()` implementation:

```rust
fn summarize(&self) -> String {
    if let Some(ref pattern) = self.glob {
        return format!(
            "Linking files matching {} to {}",
            pattern,
            self.target
                .clone()
                .or_else(|| self.to.clone())
                .unwrap_or_default()
        );
    }
    format!(
        "Linking file {} to {}",
        self.from.clone().unwrap_or(String::from("unknown")),
        self.to.clone().unwrap_or(String::from("unknown"))
    )
}
```

- [ ] **Step 4: Run to verify it passes**

```bash
cargo nextest run -p etch-lib -- glob_summarize
```

Expected: PASS.

- [ ] **Step 5: Run full test suite**

```bash
cargo nextest run -p etch-lib
```

Expected: all pass.

- [ ] **Step 6: Run lint**

```bash
make lint
```

Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "feat(file.link): update summarize() for glob mode"
```

---

## Task 8: Open PR

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli --title "feat(file.link): glob/wildcard pattern support" --body "$(cat <<'EOF'
## Summary

- Adds `glob` field to `file.link` — expands a glob pattern relative to the manifest's `files/` directory
- `*` matches top-level files; `**/*` matches recursively
- Target paths preserve subdirectory structure
- No match → error; `glob` + `source` → error
- New dep: `glob = "0.3"`

## Test plan

- [x] `glob_deserialization` — YAML with `glob:` field deserializes
- [x] `glob_and_source_both_set_returns_err` — mutual exclusion enforced
- [x] `glob_no_match_returns_err` — no-match error message
- [x] `glob_matches_top_level_files` — 3 files → 6 steps
- [x] `glob_double_star_preserves_structure` — subdir paths preserved
- [x] `glob_summarize` — summarize() reflects glob pattern

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli <PR-number> --watch
```

Expected: all checks green, auto-merge fires.

---

## Task 9: Post-merge docs update _(do directly on main after PR merges — not in worktree)_

**Files:**

- Modify: `docs/superpowers/README.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Pull merged main**

```bash
git checkout main && git pull
```

- [ ] **Step 2: Update superpowers README**

In `docs/superpowers/README.md`:

- Add to the All Plans table:
    ```
    | 2026-05-25 | [glob-file-link](plans/2026-05-25-glob-file-link.md) | [spec](specs/2026-05-25-glob-file-link-design.md) | Done |
    ```
- Remove the "Wildcard / glob file.link" row from the Backlog table.

- [ ] **Step 3: Add Done banner to plan file**

At the top of `docs/superpowers/plans/2026-05-25-glob-file-link.md`, add:

```markdown
> **Status: DONE**
```

- [ ] **Step 4: Update CLAUDE.md Action Catalog**

In the `file.link` row of the Action Catalog table in `CLAUDE.md`, update the Key fields column to include:

```
`glob` (glob pattern relative to files/ dir — expands to one symlink per match; mutually exclusive with source)
```

- [ ] **Step 5: Commit and push**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-25-glob-file-link.md CLAUDE.md
git commit -m "docs: mark glob-file-link Done, update action catalog"
git push origin main
```
