# Tilde Expansion in manifest_paths — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

`manifest_paths` entries in `etch.yaml` are passed verbatim to `PathBuf::from(url).canonicalize()`, which does not expand `~`. Users must write absolute paths (e.g. `/Users/bruce/git-repos/...`) instead of `~/git-repos/...`, breaking cross-user and cross-platform setups. The dotfiles `etch.yaml` currently has a hardcoded absolute path as a workaround.

## Scope

**Modify:** `lib/src/manifests/providers/local.rs` — expand `~` before canonicalize  
**Modify:** `lib/Cargo.toml` — add `shellexpand = "3"` to `[dependencies]`  
**Modify:** dotfiles `~/git-repos/personal/dotfiles/etch.yaml` — update to use `~` path

## Implementation

`shellexpand::tilde(url)` is called before `PathBuf::from`. It expands `~` to the current user's home directory and leaves all other input unchanged.

```rust
fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError> {
    let expanded = shellexpand::tilde(url);
    PathBuf::from(expanded.as_ref())
        .canonicalize()
        .map_err(|_| ManifestProviderError::NoResolution)
}
```

`shellexpand::tilde` (not `shellexpand::full`) is used deliberately — it expands `~` only, not `$VAR` env vars. Env var expansion in `manifest_paths` is out of scope.

## Dependency

```toml
[dependencies]
shellexpand = "3"
```

`shellexpand` is pure Rust, no unsafe, actively maintained, MIT/Apache-2 licensed.

## Testing

Two new tests in `lib/src/manifests/providers/local.rs`:

| Test                                  | What it verifies                                                                                         |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `test_resolve_tilde_expands_home`     | `"~"` alone resolves to the current user's home dir (which exists)                                       |
| `test_resolve_tilde_nonexistent_path` | `"~/etch-test-nonexistent-path-xyz"` returns `Err(NoResolution)` — tilde expanded but path doesn't exist |

Existing tests (`test_resolve_absolute_url`, `test_resolve_relative_url`) remain unchanged.

## Follow-up

After this lands, update `~/git-repos/personal/dotfiles/etch.yaml` to replace the hardcoded absolute path with `~/git-repos/personal/dotfiles/manifests`.
