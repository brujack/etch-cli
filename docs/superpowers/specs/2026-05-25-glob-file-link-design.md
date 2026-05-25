# Spec: Wildcard/Glob Support for file.link

**Date:** 2026-05-25
**Status:** Draft

## Motivation

`file.link` requires enumerating each source file explicitly. Dotfiles manifests that manage groups of config files (e.g. all files under `.claude/`, all Cursor rule files) must list every file individually. A glob field eliminates that overhead and keeps manifests stable as files are added or removed from a directory.

## Design

### New field: `glob`

Add `glob: Option<String>` to the `FileLink` struct. The field is mutually exclusive with `source`/`from` — if both are set, `plan()` returns `Err`.

```yaml
# Link all files under files/claude/ into ~/.claude/, preserving structure
- action: file.link
  glob: "claude/*"
  target: ~/.claude/

# Recursive — files in subdirectories preserve relative path
- action: file.link
  glob: "claude/**/*"
  target: ~/.claude/
```

### Glob semantics

- Pattern is relative to `manifest.root_dir/files/` — the same base as `source`
- `*` matches files in one directory level (not recursive)
- `**/*` matches recursively into subdirectories
- Standard Unix glob conventions (`?`, `[abc]`, `[a-z]` also supported via the `glob` crate)

### Expansion and path resolution

Expansion happens at the start of `plan()`, before any symlink steps are built:

1. Resolve glob root: `manifest.root_dir/files/`
2. Expand `glob::glob(glob_root.join(pattern))` → list of matched paths
3. No matches → `Err("glob pattern '...' matched no files in '...'")` — fail loudly
4. For each matched path:
    - `relative = matched.strip_prefix(glob_root)`
    - `link_source = glob_root / relative`
    - `link_target = PathBuf::from(target) / relative`
    - Feed into existing `plan_no_walk(link_source, link_target)` → `DirCreate + Link` steps

Each matched file produces 2 steps (same as a single `source` file). No new atoms required.

### Privileged mode

When `config.privileged` is set, each matched file feeds into `plan_privileged()` instead of `plan_no_walk()`. Behaviour is identical to multiple individual privileged `file.link` actions.

### Interaction with `walk_dir`

`walk_dir` is ignored when `glob` is set. Glob already enumerates all matching files explicitly; directory walking is redundant and mixing them is silently safe.

### New dependency

Add `glob = "0.3"` to `lib/Cargo.toml`. The existing `ignore` crate handles gitignore-style directory walking; the `glob` crate is purpose-built for expanding a pattern into a `Vec<PathBuf>`.

## YAML schema

```yaml
- action: file.link
  glob: <pattern> # glob pattern relative to files/ dir; mutually exclusive with source/from
  target: <path> # destination directory; relative paths from matched files are appended
  privileged: false # optional; same semantics as single-file privileged mode
```

## Error cases

| Condition                                 | Behaviour                                                    |
| ----------------------------------------- | ------------------------------------------------------------ |
| `glob` and `source`/`from` both set       | `plan()` returns `Err`                                       |
| Pattern matches no files                  | `plan()` returns `Err` with pattern and base path in message |
| Matched path cannot be stripped of prefix | `plan()` returns `Err` (should not occur in practice)        |

## Testing

All tests in `lib/src/actions/file/link.rs`:

| Test                                   | Verifies                                                        |
| -------------------------------------- | --------------------------------------------------------------- |
| `glob_matches_top_level_files`         | `files/claude/*` with 3 files → 6 steps (2 per file)            |
| `glob_double_star_preserves_structure` | `claude/**/*` with a subdir → target paths include subdirectory |
| `glob_no_match_returns_err`            | Pattern matching nothing → `Err`                                |
| `glob_and_source_both_set_returns_err` | Mutual exclusion enforced at plan time                          |
| `glob_deserialization`                 | YAML with `glob:` field deserializes to correct struct          |

Privileged glob behaviour is covered by the existing privileged tests — no separate test needed.

## Out of scope

- Glob support for `file.copy` (separate feature if needed)
- Multiple patterns (`globs: []` list) — YAGNI; single pattern covers all known use cases
- `allow_empty: true` option — use a `where:` condition instead
