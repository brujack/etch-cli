> **Status: DONE** — merged via PR #101 (2026-06-09)

# binary.github Version Drift Detection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `etch status` drift detection to `binary.github` so it reports install mismatches and available updates when `version:` is pinned.

**Architecture:** All changes are confined to `lib/src/actions/binary/github.rs`. A new `BinaryGitHubStatus` atom (status-only, no-op execute) is added alongside the existing action. `BinaryGitHub::plan()` is restructured to always emit the status atom when `version:` is a pinned tag (not `None`, not `"latest"`), plus a sidecar `SetContents` step when the binary is absent so future `etch status` calls can detect install mismatches. A GitHub release tag cache at `~/.cache/etch/github-versions/{owner}-{repo}.json` prevents repeated API calls.

**Tech Stack:** Rust, octocrab (GitHub API), serde_json, `std::fs`, `std::time::SystemTime`, `shellexpand`, `crate::atoms::file::SetContents`, existing `Atom` + `AtomStatus` traits

---

### Task 1: Add `normalize_version` + cache helpers (TDD)

**Files:**

- Modify: `lib/src/actions/binary/github.rs`

All new functions are free functions (not methods), defined above the existing `BinaryGitHub` impl.

- [ ] **Step 1: Write failing tests for `normalize_version`**

Add to the `#[cfg(test)]` module at the bottom of `lib/src/actions/binary/github.rs`:

```rust
#[test]
fn normalize_version_strips_leading_v() {
    assert_eq!(normalize_version("v1.5.0"), "1.5.0");
}

#[test]
fn normalize_version_no_change_when_no_v() {
    assert_eq!(normalize_version("1.5.0"), "1.5.0");
}

#[test]
fn normalize_version_only_strips_lowercase_v() {
    assert_eq!(normalize_version("V1.5.0"), "V1.5.0");
}
```

- [ ] **Step 2: Run to confirm compile error (RED)**

```bash
cargo test -p etch-lib 'github::tests::normalize' 2>&1 | head -15
```

Expected: compile error — `normalize_version` not defined.

- [ ] **Step 3: Implement `normalize_version`**

Add before the `BinaryGitHub` struct definition in `lib/src/actions/binary/github.rs`:

```rust
fn normalize_version(v: &str) -> &str {
    v.strip_prefix('v').unwrap_or(v)
}
```

- [ ] **Step 4: Run tests (GREEN)**

```bash
cargo test -p etch-lib 'github::tests::normalize' 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 5: Write failing tests for cache helpers**

Add to the `#[cfg(test)]` module:

```rust
#[test]
fn read_cache_returns_none_when_file_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache.json");
    assert!(read_cache(&path).is_none());
}

#[test]
fn read_cache_returns_none_for_invalid_json() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache.json");
    std::fs::write(&path, b"not json").unwrap();
    assert!(read_cache(&path).is_none());
}

#[test]
fn read_cache_returns_none_when_expired() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache.json");
    // fetched_at = 0 is always expired
    std::fs::write(&path, r#"{"tag":"v1.5.0","fetched_at":0}"#).unwrap();
    assert!(read_cache(&path).is_none());
}

#[test]
fn write_and_read_cache_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cache.json");
    write_cache(&path, "v1.7.0").unwrap();
    let tag = read_cache(&path).unwrap();
    assert_eq!(tag, "v1.7.0");
}

#[test]
fn write_cache_creates_parent_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("a/b/c/cache.json");
    write_cache(&path, "v1.0.0").unwrap();
    assert!(path.exists());
}
```

- [ ] **Step 6: Run to confirm compile error (RED)**

```bash
cargo test -p etch-lib 'github::tests::read_cache\|github::tests::write_cache' 2>&1 | head -15
```

Expected: compile error — `read_cache` / `write_cache` not defined.

- [ ] **Step 7: Implement cache helpers**

Add these imports at the top of `lib/src/actions/binary/github.rs` (after existing imports):

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
```

Then add these structs and functions above `normalize_version`:

```rust
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    tag: String,
    fetched_at: u64,
}

fn read_cache(cache_path: &Path) -> Option<String> {
    let data = fs::read_to_string(cache_path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(entry.fetched_at) < 3600 {
        Some(entry.tag)
    } else {
        None
    }
}

fn write_cache(cache_path: &Path, tag: &str) -> anyhow::Result<()> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let entry = CacheEntry {
        tag: tag.to_string(),
        fetched_at: now,
    };
    fs::write(cache_path, serde_json::to_string(&entry)?)?;
    Ok(())
}
```

- [ ] **Step 8: Run tests (GREEN)**

```bash
cargo test -p etch-lib 'github::tests::' 2>&1 | tail -15
```

Expected: all existing tests + 8 new tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/binary/github.rs
git commit -m "feat(binary): add normalize_version and GitHub release cache helpers"
```

---

### Task 2: Add `BinaryGitHubStatus` atom (TDD)

**Files:**

- Modify: `lib/src/actions/binary/github.rs`

- [ ] **Step 1: Write failing `BinaryGitHubStatus::status()` tests**

Add to the `#[cfg(test)]` module:

```rust
#[test]
fn binary_github_status_unchecked_when_no_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: Some(cache_dir.path().to_path_buf()),
    };
    assert_eq!(atom.status().unwrap(), AtomStatus::Unchecked);
}

#[test]
fn binary_github_status_drifted_when_sidecar_mismatches_pinned() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    // Write sidecar with different version
    std::fs::write(tmp.path().join(".mytool.version"), "v1.4.0").unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: Some(cache_dir.path().to_path_buf()),
    };
    assert_eq!(
        atom.status().unwrap(),
        AtomStatus::Drifted {
            expected: String::from("v1.5.0"),
            actual: String::from("v1.4.0"),
        }
    );
}

#[test]
fn binary_github_status_ok_when_sidecar_matches_and_cache_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    // Write sidecar matching pinned version
    std::fs::write(tmp.path().join(".mytool.version"), "v1.5.0").unwrap();
    // Write fresh cache file with same version
    write_cache(
        &cache_dir.path().join("owner-repo.json"),
        "v1.5.0",
    ).unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: Some(cache_dir.path().to_path_buf()),
    };
    assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
}

#[test]
fn binary_github_status_drifted_when_pinned_behind_latest() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    // Sidecar matches pinned
    std::fs::write(tmp.path().join(".mytool.version"), "v1.5.0").unwrap();
    // Cache shows newer version available
    write_cache(
        &cache_dir.path().join("owner-repo.json"),
        "v1.7.0",
    ).unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: Some(cache_dir.path().to_path_buf()),
    };
    assert_eq!(
        atom.status().unwrap(),
        AtomStatus::Drifted {
            expected: String::from("v1.7.0 (latest)"),
            actual: String::from("v1.5.0 (pinned)"),
        }
    );
}

#[test]
fn binary_github_status_ok_normalizes_v_prefix() {
    // Sidecar has no "v" prefix, pinned version has "v" prefix — should match
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".mytool.version"), "1.5.0").unwrap();
    write_cache(
        &cache_dir.path().join("owner-repo.json"),
        "v1.5.0",
    ).unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: Some(cache_dir.path().to_path_buf()),
    };
    assert_eq!(atom.status().unwrap(), AtomStatus::Ok);
}

#[test]
fn binary_github_status_display() {
    let tmp = tempfile::tempdir().unwrap();
    let atom = BinaryGitHubStatus {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        owner: String::from("owner"),
        repo: String::from("repo"),
        version: String::from("v1.5.0"),
        cache_dir: None,
    };
    assert_eq!(
        format!("{atom}"),
        "binary.github version check: owner/repo@v1.5.0"
    );
}
```

- [ ] **Step 2: Run to confirm compile error (RED)**

```bash
cargo test -p etch-lib 'github::tests::binary_github_status' 2>&1 | head -10
```

Expected: compile error — `BinaryGitHubStatus` not defined.

- [ ] **Step 3: Implement `BinaryGitHubStatus`**

Add this struct + implementations after the `write_cache` function, before the `BinaryGitHub` struct:

```rust
pub(crate) struct BinaryGitHubStatus {
    pub name: String,
    pub directory: String,
    pub owner: String,
    pub repo: String,
    pub version: String,
    /// Override the cache directory. `None` uses `~/.cache/etch/github-versions/`.
    /// Tests inject a tempdir here to avoid hitting the real GitHub API.
    pub cache_dir: Option<std::path::PathBuf>,
}

impl BinaryGitHubStatus {
    fn cache_path(&self) -> std::path::PathBuf {
        match &self.cache_dir {
            Some(dir) => dir.join(format!("{}-{}.json", self.owner, self.repo)),
            None => std::path::PathBuf::from(
                shellexpand::tilde(&format!(
                    "~/.cache/etch/github-versions/{}-{}.json",
                    self.owner, self.repo
                ))
                .into_owned(),
            ),
        }
    }

    fn fetch_latest_tag(&self) -> anyhow::Result<String> {
        let cache_path = self.cache_path();
        if let Some(cached) = read_cache(&cache_path) {
            return Ok(cached);
        }
        let async_runtime = Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create async runtime: {e}"))?;
        let octocrab = async_runtime.block_on(async { octocrab::instance() });
        let repos = octocrab.repos(&self.owner, &self.repo);
        let releases = repos.releases();
        let release = async_runtime
            .block_on(releases.get_latest())
            .map_err(|e| anyhow::anyhow!("Failed to fetch latest release: {e}"))?;
        let tag = release.tag_name;
        let _ = write_cache(&cache_path, &tag);
        Ok(tag)
    }
}

impl crate::atoms::Atom for BinaryGitHubStatus {
    fn plan(&self) -> anyhow::Result<crate::atoms::Outcome> {
        Ok(crate::atoms::Outcome {
            side_effects: vec![],
            should_run: false,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn status(&self) -> anyhow::Result<crate::atoms::AtomStatus> {
        use crate::atoms::AtomStatus;
        let sidecar = std::path::PathBuf::from(format!(
            "{}/.{}.version",
            self.directory, self.name
        ));
        if !sidecar.exists() {
            return Ok(AtomStatus::Unchecked);
        }
        let installed = fs::read_to_string(&sidecar)?.trim().to_string();
        if normalize_version(&installed) != normalize_version(&self.version) {
            return Ok(AtomStatus::Drifted {
                expected: self.version.clone(),
                actual: installed,
            });
        }
        let latest = match self.fetch_latest_tag() {
            Ok(tag) => tag,
            Err(e) => {
                tracing::warn!("binary.github: failed to fetch latest tag: {e}");
                return Ok(AtomStatus::Unchecked);
            }
        };
        if normalize_version(&latest) != normalize_version(&self.version) {
            return Ok(AtomStatus::Drifted {
                expected: format!("{latest} (latest)"),
                actual: format!("{} (pinned)", self.version),
            });
        }
        Ok(AtomStatus::Ok)
    }
}

impl std::fmt::Display for BinaryGitHubStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "binary.github version check: {}/{}@{}",
            self.owner, self.repo, self.version
        )
    }
}
```

- [ ] **Step 4: Run tests (GREEN)**

```bash
cargo test -p etch-lib 'github::tests::binary_github_status' 2>&1
```

Expected: all 6 `binary_github_status_*` tests pass.

- [ ] **Step 5: Run full suite to confirm no regressions**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/binary/github.rs
git commit -m "feat(binary): add BinaryGitHubStatus atom for version drift detection"
```

---

### Task 3: Restructure `BinaryGitHub::plan()` + add tests

**Files:**

- Modify: `lib/src/actions/binary/github.rs`

- [ ] **Step 1: Write failing tests for new plan() behavior**

Add to the `#[cfg(test)]` module:

```rust
#[test]
fn plan_with_pinned_version_and_binary_present_emits_one_step() {
    let tmp = tempfile::tempdir().unwrap();
    // Create the binary file so the action sees it as already installed
    std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

    let action = BinaryGitHub {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        repository: String::from("owner/repo"),
        version: Some(String::from("v1.5.0")),
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert_eq!(1, steps.len(), "expected 1 status-only step");
    assert!(
        steps[0].atom.to_string().contains("version check"),
        "expected status atom, got: {}",
        steps[0].atom
    );
}

#[test]
fn plan_with_no_version_and_binary_present_emits_zero_steps() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

    let action = BinaryGitHub {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        repository: String::from("owner/repo"),
        version: None,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert_eq!(0, steps.len());
}

#[test]
fn plan_with_latest_version_and_binary_present_emits_zero_steps() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

    let action = BinaryGitHub {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        repository: String::from("owner/repo"),
        version: Some(String::from("latest")),
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    assert_eq!(0, steps.len());
}

#[test]
fn plan_with_pinned_version_and_invalid_repo_errors() {
    let tmp = tempfile::tempdir().unwrap();
    // Binary present — but invalid repo still errors when version is pinned
    std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

    let action = BinaryGitHub {
        name: String::from("mytool"),
        directory: tmp.path().display().to_string(),
        repository: String::from("no-slash-here"),
        version: Some(String::from("v1.5.0")),
    };
    assert!(action
        .plan(&Manifest::default(), &Contexts::default())
        .is_err());
}
```

- [ ] **Step 2: Run to confirm new tests fail (RED)**

```bash
cargo test -p etch-lib 'github::tests::plan_with' 2>&1 | tail -15
```

Expected: `plan_with_pinned_version_and_binary_present_emits_one_step` fails (currently returns 0 steps when binary exists); `plan_with_no_version_and_binary_present_emits_zero_steps` passes (existing behavior); others may fail.

- [ ] **Step 3: Restructure `BinaryGitHub::plan()`**

Replace the entire `plan()` method body in `impl Action for BinaryGitHub`. The new structure:

Add this import at the top of the file (with other atoms imports):

```rust
use crate::atoms::file::SetContents;
```

New `plan()` body:

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    let binary_path =
        std::path::PathBuf::from(format!("{}/{}", self.directory, self.name));
    let binary_exists = binary_path.exists();
    let is_pinned = matches!(&self.version, Some(v) if v != "latest");

    // Early exit: no version pinned + binary exists = nothing to do
    if !is_pinned && binary_exists {
        return Ok(vec![]);
    }

    let (owner, repo) = self.repository.split_once('/').ok_or_else(|| {
        anyhow!(
            "Failed to parse repository name: {}",
            self.repository.as_str()
        )
    })?;

    let mut steps: Vec<Step> = Vec::new();

    // Version drift check atom — always present when version is pinned
    if is_pinned {
        steps.push(Step {
            atom: Box::new(BinaryGitHubStatus {
                name: self.name.clone(),
                directory: self.directory.clone(),
                owner: owner.to_string(),
                repo: repo.to_string(),
                version: self.version.as_ref().unwrap().clone(),
                cache_dir: None,
            }),
            initializers: vec![],
            finalizers: vec![],
        });
    }

    // Download steps — only when binary is absent
    if !binary_exists {
        let async_runtime = match Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(anyhow!("Failed to create async runtime: {e}"));
            }
        };

        let octocrab = async_runtime.block_on(async { octocrab::instance() });
        let repos = octocrab.repos(owner, repo);
        let releases = repos.releases();

        let result = match &self.version {
            Some(version) if version == "latest" => {
                async_runtime.block_on(releases.get_latest())
            }
            Some(version) => {
                async_runtime.block_on(releases.get_by_tag(version.as_str()))
            }
            None => async_runtime.block_on(releases.get_latest()),
        };

        let release = match result {
            Ok(release) => release,
            Err(e) => {
                return Err(anyhow!("Failed to find a release: {e}"));
            }
        };

        let asset: Option<GitHubAsset> =
            release.assets.into_iter().fold(None, |acc, asset| {
                let mut score = 0;

                let mut score_terms = vec![
                    std::env::consts::OS.to_lowercase(),
                    std::env::consts::ARCH.to_lowercase(),
                ];

                let os = os_info::get();
                if os.os_type() == os_info::Type::Macos {
                    score_terms.push(String::from("darwin"));
                    score_terms.push(String::from("apple"));
                } else {
                    score_terms.push(os.os_type().to_string());
                };

                if std::env::consts::ARCH == "aarch64" {
                    score_terms.push("arm".to_string());
                    score_terms.push("aarch".to_string());
                } else {
                    score_terms.push("unknown".to_string());
                };

                match os.bitness() {
                    os_info::Bitness::X32 => score_terms.push("32".to_string()),
                    os_info::Bitness::X64 => score_terms.push("64".to_string()),
                    _ => (),
                }

                score_terms.iter().for_each(|term| {
                    if asset.name.to_lowercase().contains(term.as_str()) {
                        score += 1;
                    }
                });

                match acc {
                    Some(ass) => {
                        if score > ass.score {
                            Some(GitHubAsset {
                                url: asset.browser_download_url.into(),
                                score,
                            })
                        } else {
                            Some(ass)
                        }
                    }
                    None => Some(GitHubAsset {
                        url: asset.browser_download_url.into(),
                        score,
                    }),
                }
            });

        let asset = match asset {
            Some(asset) => {
                debug!("Downloading {:?}", asset.url);
                asset
            }
            None => {
                return Err(anyhow!("Failed to find a downloadable asset"));
            }
        };

        let to_path =
            std::path::PathBuf::from(format!("{}/{}", self.directory, self.name));

        steps.push(Step {
            atom: Box::new(Download {
                url: asset.url,
                to: to_path.clone(),
            }),
            initializers: vec![],
            finalizers: vec![],
        });
        steps.push(Step {
            atom: Box::new(Chmod {
                path: to_path,
                mode: 0o755,
            }),
            initializers: vec![],
            finalizers: vec![],
        });

        // Write sidecar version file so future `etch status` can detect drift
        if is_pinned {
            let sidecar = std::path::PathBuf::from(format!(
                "{}/.{}.version",
                self.directory, self.name
            ));
            steps.push(Step {
                atom: Box::new(SetContents {
                    path: sidecar,
                    contents: self.version.as_ref().unwrap().as_bytes().to_vec(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }
    }

    Ok(steps)
}
```

- [ ] **Step 4: Run new tests (GREEN)**

```bash
cargo test -p etch-lib 'github::tests::plan_with' 2>&1
```

Expected: all 4 new `plan_with_*` tests pass.

- [ ] **Step 5: Verify existing tests still pass**

```bash
cargo test -p etch-lib 'github::tests::' 2>&1
```

Expected: all tests pass including:

- `it_can_be_deserialized`
- `plan_returns_empty_when_binary_already_exists`
- `plan_errors_on_invalid_repository_format`
- All normalize/cache/status tests from Tasks 1+2

Note: `plan_returns_empty_when_binary_already_exists` uses `version: None` — still returns 0 steps when binary exists and no version pinned. This is unchanged. ✅

- [ ] **Step 6: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/binary/github.rs
git commit -m "feat(binary): restructure plan() to add version drift status atom and sidecar"
```

---

### Task 4: Update docs

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Update README action catalog entry for `binary.github`**

Find the `binary.github` row in the action catalog table in `README.md`. Update its description to mention version drift detection:

Current entry mentions `version` as a field. Add: `When \`version:\` is set, \`etch status\` detects install mismatches (via sidecar \`~/{dir}/.{name}.version\`) and available updates (via cached GitHub API check).`

The exact text to add depends on how the current entry reads — read the file first, then edit surgically.

- [ ] **Step 2: Run tests**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document binary.github version drift detection in README"
```

---

### Task 5: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli \
  --title "feat(binary): add version drift detection to binary.github" \
  --body "$(cat <<'EOF'
## Summary
- Adds \`BinaryGitHubStatus\` atom to \`binary.github\` — \`etch status\` now reports version drift when \`version:\` is pinned
- Install mismatch (B): sidecar file \`{dir}/.{name}.version\` written on install; \`etch status\` compares sidecar to pinned version
- Update available (A): cached GitHub API check (1-hour TTL at \`~/.cache/etch/github-versions/\`); compares pinned version to latest release
- \`plan()\` always returns the status atom when version is pinned; download/chmod/sidecar only when binary is absent
- No change when \`version:\` is \`None\` or \`\"latest\"\`

## Test plan
- [x] \`normalize_version\` tests pass
- [x] Cache read/write roundtrip tests pass
- [x] \`BinaryGitHubStatus::status()\` tests pass (all use pre-written cache files, no network)
- [x] \`plan()\` restructure tests pass
- [x] Existing tests unchanged
- [x] \`make test\` green locally

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks <number> --repo brujack/etch-cli --watch
```

Expected: `test`, `secret-scan`, `cargo-audit`, `snyk-scan`, `docs-lint`, `docs-build` all green; `semver-check` advisory failure (expected — public struct/method changes).

- [ ] **Step 4: After PR auto-merges, clean up**

```bash
git fetch --prune
git reset --hard origin/main
git branch -D <branch-name>
git push origin --delete <branch-name>
```

---

### Task 6: Post-merge docs update (on main, not in worktree)

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update `docs/superpowers/README.md`**

Add row:

```markdown
| 2026-06-09 | [binary-github-drift](plans/2026-06-09-binary-github-drift-plan.md) | [binary-github-drift](specs/2026-06-09-binary-github-drift-design.md) | Done |
```

Remove the `binary.get version drift` backlog entry.

Add `> **Status: DONE**` banner at the top of this plan file.

Update `CLAUDE.md` and `docs/knowledge/action-catalog.md` to mention version drift detection under `binary.github`.

- [ ] **Step 2: Commit and push**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-06-09-binary-github-drift-plan.md CLAUDE.md docs/knowledge/action-catalog.md
git commit -m "docs(superpowers): mark binary.github drift detection Done"
git push
```
