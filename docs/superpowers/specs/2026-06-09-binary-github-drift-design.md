# binary.github Version Drift Detection — Design Spec

## Overview

Add version drift detection to `binary.github` so `etch status` can report:

1. **Install mismatch (B):** the binary on disk was installed from a different version than what `version:` specifies
2. **Update available (A):** the pinned `version:` is behind the latest GitHub release

Only activates when `version:` is explicitly set (not `"latest"`, not `None`).

## Motivation

`binary.github` is idempotent by file existence — once installed, `etch apply` skips re-download even if the manifest's `version:` changes. `etch status` currently returns `Unchecked` for all binary atoms. `setup_env.sh` works around this by manually fetching the GitHub API and comparing against hardcoded constants. This spec makes `etch status` the canonical drift detector for pinned GitHub binaries.

## Design

### New: `BinaryGitHubStatus` atom

Defined in `lib/src/actions/binary/github.rs` alongside the action (colocation; similar to the private `GitHubAsset` struct already there).

```rust
pub(crate) struct BinaryGitHubStatus {
    pub name: String,
    pub directory: String,
    pub owner: String,
    pub repo: String,
    pub version: String,  // the pinned version string from the manifest
}
```

Implements `Atom`:

- **`execute()`** — no-op; returns `Ok(Outcome::default())`. This atom is status-only; `etch apply` does nothing with it.
- **`status()`** — sequential chain (stops at first non-Ok result):
    1. Read sidecar `{directory}/.{name}.version`. If absent → `AtomStatus::Unchecked` (installed before this feature).
    2. Parse sidecar version. Compare to `self.version` (both normalized: strip leading `v`). If different → `AtomStatus::Drifted { expected: self.version.clone(), actual: sidecar_version }`.
    3. Fetch latest GitHub release tag via cache (see Cache section). Compare to `self.version` (both normalized). If different → `AtomStatus::Drifted { expected: format!("{} (latest)", latest), actual: format!("{} (pinned)", self.version) }`.
    4. All match → `AtomStatus::Ok`.
- **`Display`** — `"binary.github version check: {owner}/{repo}@{version}"`

**Tag normalization:** `normalize_version(s: &str) -> String` strips a leading `v` for comparison only. The raw strings are preserved in `Drifted` messages.

### Cache

- **Path:** `shellexpand::tilde("~/.cache/etch/github-versions/{owner}-{repo}.json")`
- **Content (JSON):**
    ```json
    { "tag": "v1.7.0", "fetched_at": 1718000000 }
    ```
    `fetched_at` is a Unix timestamp (seconds). Use `std::time::SystemTime`.
- **TTL:** 3600 seconds (1 hour). If cache file exists and `now - fetched_at < 3600`, use cached tag. Otherwise fetch from GitHub API and overwrite.
- **On API failure:** log a warning and return `AtomStatus::Unchecked` — do not fail `etch status`.
- **Cache directory creation:** `fs::create_dir_all` before writing, so it works on a fresh machine.

### Sidecar file

- **Path:** `{directory}/.{name}.version`
- **Content:** the version string verbatim (e.g., `v1.5.0\n`)
- **Written by:** new `SetContents` step added to `BinaryGitHub::plan()` when `version` is Some and binary is absent.
- **Not written** when `version` is `None` or `"latest"`.

### `BinaryGitHub::plan()` changes

Current behavior (unchanged when `version` is `None` or `"latest"`):

```
if binary exists: return []
else: return [Download, Chmod]
```

New behavior when `version` is `Some(v)` and `v != "latest"`:

```
base = [BinaryGitHubStatus { owner, repo, name, directory, version }]
if binary absent:
    owner/repo = parse(self.repository)
    base += [Download, Chmod, SetContents(.{name}.version, version_bytes)]
return base
```

The `BinaryGitHubStatus` step is **always present** when `version` is a pinned tag. This ensures `etch status` always has something to report, even when the binary is already installed.

**Parsing `owner`/`repo`:** extract once at the top of `plan()` using the existing `split_once('/')` pattern and store in locals. Pass into `BinaryGitHubStatus`.

### Scope constraint

This feature is **`binary.github`-only**. `binary.url` has no reliable way to check upstream for a newer version. Its behavior is unchanged.

## Tests

All tests in `lib/src/actions/binary/github.rs`:

### `BinaryGitHubStatus::status()` tests (all use `tempfile::tempdir()`)

| Test                                               | Setup                                                             | Expected                                                             |
| -------------------------------------------------- | ----------------------------------------------------------------- | -------------------------------------------------------------------- |
| `status_unchecked_when_no_sidecar`                 | No sidecar file                                                   | `Unchecked`                                                          |
| `status_drifted_when_sidecar_mismatches_pinned`    | Sidecar = `"v1.4.0"`, pinned = `"v1.5.0"`                         | `Drifted { expected: "v1.5.0", actual: "v1.4.0" }`                   |
| `status_ok_when_sidecar_matches_and_cache_matches` | Sidecar = `"v1.5.0"`, pinned = `"v1.5.0"`, cache tag = `"v1.5.0"` | `Ok`                                                                 |
| `status_drifted_when_pinned_behind_latest`         | Sidecar = `"v1.5.0"`, pinned = `"v1.5.0"`, cache tag = `"v1.7.0"` | `Drifted { expected: "v1.7.0 (latest)", actual: "v1.5.0 (pinned)" }` |
| `normalize_version_strips_leading_v`               | `normalize_version("v1.5.0")`                                     | `"1.5.0"`                                                            |
| `normalize_version_no_v_unchanged`                 | `normalize_version("1.5.0")`                                      | `"1.5.0"`                                                            |

**Note:** "API failure → Unchecked" is specified in prose but not unit-tested — it requires mocking network calls. All `status()` unit tests pre-write a cache file to avoid hitting the real GitHub API.

**Testing without hitting GitHub API:** pre-write a cache file in the temp directory with a fresh `fetched_at` timestamp. The cache path is injected via `BinaryGitHubStatus { cache_dir: Option<PathBuf>, .. }` (defaults to `None` → `~/.cache/etch/github-versions/`). Tests pass a tempdir path.

### `BinaryGitHub::plan()` behavior tests

| Test                                                          | Condition                                 | Expected steps                         |
| ------------------------------------------------------------- | ----------------------------------------- | -------------------------------------- |
| `plan_with_pinned_version_and_binary_absent_emits_four_steps` | `version: Some("v1.5.0")`, binary absent  | 4: status + download + chmod + sidecar |
| `plan_with_pinned_version_and_binary_present_emits_one_step`  | `version: Some("v1.5.0")`, binary present | 1: status only                         |
| `plan_with_no_version_and_binary_absent_emits_two_steps`      | `version: None`, binary absent            | 2: download + chmod (unchanged)        |
| `plan_with_no_version_and_binary_present_emits_zero_steps`    | `version: None`, binary present           | 0 (unchanged)                          |
| `plan_with_latest_version_and_binary_absent_emits_two_steps`  | `version: Some("latest")`, binary absent  | 2: download + chmod (no status atom)   |

These tests use `tempfile::tempdir()` to create the binary file for "present" cases.

## Files changed

| File                               | Change                                                                                                                  |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `lib/src/actions/binary/github.rs` | Add `BinaryGitHubStatus` struct + `Atom` impl; update `BinaryGitHub::plan()`; add `normalize_version` helper; add tests |
| `docs/superpowers/README.md`       | Add row, mark Done post-merge                                                                                           |
| `README.md`                        | Update `binary.github` entry to mention version drift detection                                                         |
