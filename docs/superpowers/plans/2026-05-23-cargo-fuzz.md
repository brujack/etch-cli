# cargo-fuzz Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two cargo-fuzz targets to etch-cli — one for manifest deserialization and one for file path resolution — both runnable via `make fuzz` / `make fuzz-manifest` / `make fuzz-path`.

**Architecture:** A standalone `fuzz/` crate (not in the workspace members) contains two `libfuzzer-sys` fuzz targets. The manifest target feeds arbitrary bytes as YAML/TOML to the `Manifest` serde deserializer. The path-resolve target feeds arbitrary strings to `FileLink::resolve()` with a stub `Manifest`. Corpus inputs are committed to `fuzz/corpus/`; crash artifacts are gitignored.

**Tech Stack:** Rust nightly (required by libFuzzer), `libfuzzer-sys 0.4`, `etch-lib`, `serde_yaml_ng`, `toml`, `cargo fuzz`

---

### Task 1: Create fuzz crate

**Files:**

- Create: `fuzz/Cargo.toml`
- Modify: `Cargo.toml` (root workspace — add `exclude`)
- Modify: `.gitignore`

- [ ] **Step 1: Add fuzz to workspace exclude**

Open `Cargo.toml` (root). Add an `exclude` key so `cargo build` from the workspace root never accidentally picks up the fuzz crate:

```toml
[workspace]
members = ["app", "jsonschemagen", "lib"]
resolver = "2"
exclude = ["fuzz"]
```

- [ ] **Step 2: Create `fuzz/Cargo.toml`**

```toml
[package]
name = "etch-lib-fuzz"
version = "0.0.1"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
serde_yaml_ng = "0.10"
toml = "1.0"

[dependencies.etch-lib]
path = "../lib"

[[bin]]
name = "fuzz_manifest"
path = "fuzz_targets/fuzz_manifest.rs"
test = false
doc = false

[[bin]]
name = "fuzz_path_resolve"
path = "fuzz_targets/fuzz_path_resolve.rs"
test = false
doc = false
```

- [ ] **Step 3: Update `.gitignore`**

Append to `.gitignore`:

```gitignore
# cargo-fuzz artifacts (crashes, slow inputs) — not committed
fuzz/artifacts/
# cargo-fuzz build output
fuzz/target/
```

- [ ] **Step 4: Verify workspace builds**

Run from repo root:

```bash
export PATH="$PATH:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
cargo build 2>&1 | tail -5
```

Expected: build succeeds, no mention of `etch-lib-fuzz`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml fuzz/Cargo.toml .gitignore
git commit -m "chore(fuzz): add fuzz crate scaffold"
```

---

### Task 2: fuzz_manifest target

**Files:**

- Create: `fuzz/fuzz_targets/fuzz_manifest.rs`

- [ ] **Step 1: Create `fuzz/fuzz_targets/fuzz_manifest.rs`**

```rust
#![no_main]

use etch_lib::manifests::Manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_yaml_ng::from_str::<Manifest>(s);
        let _ = toml::from_str::<Manifest>(s);
    }
});
```

- [ ] **Step 2: Verify it compiles**

```bash
export PATH="$PATH:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
cargo +nightly fuzz build fuzz_manifest 2>&1 | tail -10
```

Expected: `Finished` with no errors. If nightly is not installed:

```bash
rustup toolchain install nightly
cargo +nightly fuzz build fuzz_manifest 2>&1 | tail -10
```

- [ ] **Step 3: Commit**

```bash
git add fuzz/fuzz_targets/fuzz_manifest.rs
git commit -m "feat(fuzz): add fuzz_manifest target for YAML/TOML deserialization"
```

---

### Task 3: fuzz_path_resolve target

**Files:**

- Modify: `lib/src/actions/mod.rs` (add public re-exports)
- Create: `fuzz/fuzz_targets/fuzz_path_resolve.rs`

`mod file` in `lib/src/actions/mod.rs` is private, so `FileLink` and `FileAction` are not accessible from the fuzz crate. Add minimal public re-exports.

- [ ] **Step 1: Re-export `FileLink` and `FileAction` from `lib/src/actions/mod.rs`**

Open `lib/src/actions/mod.rs`. Find the `mod file;` line and add two `pub use` lines immediately after it:

```rust
mod file;
pub use file::link::FileLink;
pub use file::FileAction;
```

Verify the existing `use file::...` lines are left untouched — only the two new `pub use` lines are added.

- [ ] **Step 2: Confirm the re-exports compile**

```bash
export PATH="$PATH:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
cargo build -p etch-lib 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Create `fuzz/fuzz_targets/fuzz_path_resolve.rs`**

```rust
#![no_main]

use etch_lib::actions::{FileAction, FileLink};
use etch_lib::manifests::Manifest;
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fn stub_manifest() -> Manifest {
    Manifest {
        root_dir: Some(PathBuf::from(std::env::temp_dir())),
        ..Default::default()
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let manifest = stub_manifest();
        let action = FileLink::default();
        let _ = action.resolve(&manifest, s);
    }
});
```

- [ ] **Step 4: Verify it compiles**

```bash
export PATH="$PATH:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
cargo +nightly fuzz build fuzz_path_resolve 2>&1 | tail -10
```

Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/mod.rs fuzz/fuzz_targets/fuzz_path_resolve.rs
git commit -m "feat(fuzz): add fuzz_path_resolve target for FileAction path normalization"
```

---

### Task 4: Seed corpus

**Files:**

- Create: `fuzz/corpus/fuzz_manifest/` (directory + seed files)
- Create: `fuzz/corpus/fuzz_path_resolve/` (directory + seed files)

- [ ] **Step 1: Create manifest seed inputs**

Create `fuzz/corpus/fuzz_manifest/seed_file_link.yaml`:

```yaml
actions:
    - action: file.link
      source: dotfiles/.zshrc
      target: ~/.zshrc
```

Create `fuzz/corpus/fuzz_manifest/seed_command.yaml`:

```yaml
name: example
labels:
    - dev
actions:
    - action: command.run
      command: echo
      args:
          - hello
```

Create `fuzz/corpus/fuzz_manifest/seed_empty.yaml`:

```yaml
actions: []
```

Create `fuzz/corpus/fuzz_manifest/seed_where.yaml`:

```yaml
where: os.name == "macos"
actions:
    - action: file.link
      source: mac/.zshrc
      target: ~/.zshrc
```

Create `fuzz/corpus/fuzz_manifest/seed_toml.toml`:

```toml
name = "seed"
labels = ["dev"]

[[actions]]
action = "command.run"
command = "echo"
args = ["hello"]
```

- [ ] **Step 2: Create path-resolve seed inputs**

Create `fuzz/corpus/fuzz_path_resolve/seed_relative` (no extension — raw bytes):

```
dotfiles/.zshrc
```

Create `fuzz/corpus/fuzz_path_resolve/seed_traversal`:

```
../../etc/passwd
```

Create `fuzz/corpus/fuzz_path_resolve/seed_empty`:

```

```

Create `fuzz/corpus/fuzz_path_resolve/seed_unicode`:

```
café/.zshrc
```

Create `fuzz/corpus/fuzz_path_resolve/seed_absolute`:

```
/tmp/test
```

- [ ] **Step 3: Verify seeds are valid UTF-8 and non-empty**

```bash
find fuzz/corpus -type f | sort | xargs wc -c
```

Expected: all files show non-zero byte counts (except `seed_empty` which should be 1 byte — a newline).

- [ ] **Step 4: Commit**

```bash
git add fuzz/corpus/
git commit -m "chore(fuzz): add seed corpus for manifest and path-resolve targets"
```

---

### Task 5: Makefile targets

**Files:**

- Modify: `Makefile`

- [ ] **Step 1: Add fuzz targets to `Makefile`**

Add after the existing `mutants:` target:

```makefile
FUZZ_TIMEOUT ?= 60

fuzz-manifest:
	cargo +nightly fuzz run fuzz_manifest fuzz/corpus/fuzz_manifest -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz-path:
	cargo +nightly fuzz run fuzz_path_resolve fuzz/corpus/fuzz_path_resolve -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz: fuzz-manifest fuzz-path
```

Also update the `.PHONY` line to include the new targets:

```makefile
.PHONY: all test lint build build-linux install-hooks mutants changelog fuzz fuzz-manifest fuzz-path
```

- [ ] **Step 2: Verify Makefile syntax**

```bash
make --dry-run fuzz-manifest FUZZ_TIMEOUT=5 2>&1 | head -5
```

Expected: prints the `cargo +nightly fuzz run ...` command without executing it.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore(fuzz): add fuzz, fuzz-manifest, fuzz-path Makefile targets"
```

---

### Task 6: Smoke test and PR

**Files:** none new

- [ ] **Step 1: Run both fuzz targets for 10 seconds each**

```bash
export PATH="$PATH:$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
make fuzz FUZZ_TIMEOUT=10
```

Expected:

- Each target starts, runs for ~10 seconds, then exits with `Done 10s` or similar libFuzzer output
- No `SUMMARY: AddressSanitizer` or crash lines
- Corpus entries may be written to `fuzz/corpus/*/`

If new corpus entries were written, commit them:

```bash
git add fuzz/corpus/
git status  # confirm only corpus files changed
git commit -m "chore(fuzz): add corpus entries from initial smoke run"
```

- [ ] **Step 2: Update superpowers README** _(do this directly on main after the PR merges — not inside the worktree)_

In `docs/superpowers/README.md`, change the `cargo-fuzz` row status from `Pending` to `Done`:

```markdown
| 2026-05-23 | [cargo-fuzz](plans/2026-05-23-cargo-fuzz.md) | [spec](specs/2026-05-23-cargo-fuzz-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

- [ ] **Step 3: Push and open PR**

```bash
git push origin <branch>
gh pr create --title "feat(fuzz): add cargo-fuzz targets for manifest parsing and path resolution" \
  --body "Adds fuzz_manifest and fuzz_path_resolve targets. Run via \`make fuzz\` (default 60s per target). Corpus seeded from examples/. Artifacts gitignored."
```

- [ ] **Step 4: Monitor CI and merge**

```bash
gh pr checks <number> --watch
```

Expected: all checks pass. After merge, run post-merge cleanup:

```bash
git worktree remove /path/to/worktree
git branch -D <branch>
git push origin --delete <branch>
git fetch --prune && git pull
```
