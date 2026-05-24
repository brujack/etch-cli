# Criterion Benchmarks — etch-cli Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Criterion benchmarks to `etch-lib` (manifest deserialization + file path resolution) and publish historical trend charts to GitHub Pages via `benchmark-action/github-action-benchmark`, running monthly.

**Architecture:** Three benchmarks in `lib/benches/etch_lib.rs` measure YAML deserialization (`serde_yaml_ng`), TOML deserialization (`toml`), and file path resolution (`FileAction::resolve`). A single `benchmarks.yml` CI workflow runs `cargo bench -p etch-lib`, pipes `--output-format bencher` output to the action, which auto-commits to `gh-pages`. Chart.js renders time-series charts at `https://brujack.github.io/etch-cli/dev/bench/`.

**Tech Stack:** Criterion 0.5, `benchmark-action/github-action-benchmark@v1`, `dtolnay/rust-toolchain@stable`, `actions/checkout@v6`

---

### Task 1: Initialize gh-pages branch

**Files:**

- Creates: `gh-pages` orphan branch (remote only)

The `benchmark-action/github-action-benchmark` action requires a `gh-pages` branch to exist before it can push benchmark data.

- [ ] **Step 1: Create the orphan branch**

Run from the etch-cli repo root:

```bash
cd ~/git-repos/personal/etch-cli
git checkout --orphan gh-pages
git rm -rf .
echo "# Benchmark Results" > README.md
git add README.md
git commit -m "chore: init gh-pages branch for benchmark results"
git push origin gh-pages --repo brujack/etch-cli
git checkout main
```

- [ ] **Step 2: Enable GitHub Pages via API**

```bash
gh api repos/brujack/etch-cli/pages \
  --method POST \
  --field source='{"branch":"gh-pages","path":"/"}' \
  --header "Accept: application/vnd.github.v3+json" 2>/dev/null || true
```

Expected: either success JSON or "already enabled". If it returns an error about Pages being already configured, that's fine — the branch exists and the action will populate it.

- [ ] **Step 3: Verify**

```bash
git branch -a | grep gh-pages
```

Expected: `remotes/origin/gh-pages` appears.

---

### Task 2: etch-lib — Criterion bench

**Files:**

- Modify: `lib/Cargo.toml`
- Create: `lib/benches/etch_lib.rs`

- [ ] **Step 1: Add Criterion to lib/Cargo.toml dev-dependencies and add bench target**

In `lib/Cargo.toml`, add `criterion` to `[dev-dependencies]` and add a `[[bench]]` section:

```toml
[dev-dependencies]
tempfile = "3.26"
pretty_assertions = "1.4"
proptest = "1"
serial_test = "3"
tracing-test = "0.2"
tracing-subscriber = "0.3"
criterion = "0.5"

[[bench]]
name = "etch_lib"
harness = false
```

Note: only `criterion = "0.5"` and the `[[bench]]` section are new additions; the existing dev-dependencies stay as-is.

- [ ] **Step 2: Create `lib/benches/etch_lib.rs`**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use etch_lib::actions::file::{link::FileLink, FileAction};
use etch_lib::manifests::Manifest;
use std::path::PathBuf;

const MANIFEST_YAML: &str = r#"
name: dotfiles
labels:
  - development
  - macos
actions:
  - file.link:
      source: .zshrc
      target: ~/.zshrc
  - file.link:
      source: .gitconfig
      target: ~/.gitconfig
  - file.link:
      source: .vimrc
      target: ~/.vimrc
"#;

const MANIFEST_TOML: &str = r#"
name = "dotfiles"
labels = ["development", "macos"]

[[actions]]
[actions."file.link"]
source = ".zshrc"
target = "~/.zshrc"

[[actions]]
[actions."file.link"]
source = ".gitconfig"
target = "~/.gitconfig"

[[actions]]
[actions."file.link"]
source = ".vimrc"
target = "~/.vimrc"
"#;

fn bench_manifest_yaml(c: &mut Criterion) {
    c.bench_function("manifest_yaml", |b| {
        b.iter(|| serde_yaml_ng::from_str::<Manifest>(black_box(MANIFEST_YAML)).unwrap())
    });
}

fn bench_manifest_toml(c: &mut Criterion) {
    c.bench_function("manifest_toml", |b| {
        b.iter(|| toml::from_str::<Manifest>(black_box(MANIFEST_TOML)).unwrap())
    });
}

fn bench_file_link_resolve(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let files_dir = dir.path().join("files");
    std::fs::create_dir_all(&files_dir).unwrap();
    std::fs::write(files_dir.join(".zshrc"), b"").unwrap();
    std::fs::write(files_dir.join(".gitconfig"), b"").unwrap();
    std::fs::write(files_dir.join(".vimrc"), b"").unwrap();

    let manifest = Manifest {
        root_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let link = FileLink::default();

    let mut group = c.benchmark_group("file_link_resolve");
    group.bench_function("single_dotfile", |b| {
        b.iter(|| link.resolve(black_box(&manifest), black_box(".zshrc")).unwrap())
    });
    group.bench_function("nested_path", |b| {
        std::fs::create_dir_all(files_dir.join("config/git")).unwrap();
        std::fs::write(files_dir.join("config/git/config"), b"").unwrap();
        b.iter(|| {
            link.resolve(black_box(&manifest), black_box("config/git/config"))
                .unwrap()
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_manifest_yaml,
    bench_manifest_toml,
    bench_file_link_resolve
);
criterion_main!(benches);
```

Note: `serde_yaml_ng` and `toml` are already in `[dependencies]` in `lib/Cargo.toml`, so they are available to `benches/` without adding them to `[dev-dependencies]`. `tempfile` is already in `[dev-dependencies]`.

- [ ] **Step 3: Run the bench locally to verify it compiles and runs**

```bash
cd ~/git-repos/personal/etch-cli
cargo bench -p etch-lib -- --output-format bencher 2>/dev/null
```

Expected: lines like `test manifest_yaml ... bench: 1234 ns/iter (+/- 56)` — exact numbers don't matter, just that it runs to completion without panics.

If the TOML manifest parse fails (toml syntax is strict), adjust `MANIFEST_TOML` until `toml::from_str::<Manifest>` parses it without error. The YAML format is the primary one; TOML is a secondary format etch supports but rarely uses.

- [ ] **Step 4: Commit**

```bash
cd ~/git-repos/personal/etch-cli
git add lib/Cargo.toml lib/benches/etch_lib.rs
git commit -m "feat(bench): add Criterion benchmarks for manifest deserialization and file path resolution"
```

---

### Task 3: CI workflow — benchmarks.yml

**Files:**

- Create: `.github/workflows/benchmarks.yml`

- [ ] **Step 1: Create the workflow file**

```yaml
name: Benchmarks

on:
    workflow_dispatch:
    schedule:
        - cron: "0 2 1 * *" # 1st of month, 2am UTC

env:
    FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true

jobs:
    benchmark:
        runs-on: ubuntu-latest
        permissions:
            contents: write
        steps:
            - uses: actions/checkout@v6

            - uses: dtolnay/rust-toolchain@stable

            - name: Run benchmarks
              run: |
                  cargo bench -p etch-lib \
                    -- --output-format bencher 2>/dev/null | tee output.txt

            - uses: benchmark-action/github-action-benchmark@v1
              with:
                  tool: cargo
                  output-file-path: output.txt
                  github-token: ${{ secrets.GITHUB_TOKEN }}
                  auto-push: true
                  gh-pages-branch: gh-pages
                  benchmark-data-dir-path: dev/bench
                  comment-on-alert: false
```

- [ ] **Step 2: Commit**

```bash
cd ~/git-repos/personal/etch-cli
git add .github/workflows/benchmarks.yml
git commit -m "ci: add monthly Criterion benchmark workflow"
```

---

### Task 4: Makefile bench target

**Files:**

- Modify: `Makefile`

- [ ] **Step 1: Add bench target to Makefile**

Add after the existing `mutants` target:

```makefile
bench:
	cargo bench -p etch-lib
```

The `.PHONY` line at the top already lists all targets — add `bench` to it:

```makefile
.PHONY: all test lint build build-linux install-hooks mutants changelog fuzz fuzz-manifest fuzz-path bench
```

- [ ] **Step 2: Verify it runs**

```bash
cd ~/git-repos/personal/etch-cli
make bench 2>&1 | tail -15
```

Expected: benchmark output with timing numbers, no panics or compilation errors.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore(make): add bench target for etch-lib Criterion benchmarks"
```

---

### Task 5: PR, smoke test, post-merge docs

**Files:**

- Worktree branch: push and open PR
- Post-merge (on main): update `docs/superpowers/README.md` and `docs/superpowers/plans/2026-05-23-criterion-benchmarks.md`

- [ ] **Step 1: Open PR**

```bash
git -C ~/git-repos/personal/etch-cli push origin <feature-branch> --repo brujack/etch-cli
gh pr create \
  --repo brujack/etch-cli \
  --title "feat(bench): Criterion benchmarks for etch-lib" \
  --body "$(cat <<'EOF'
## Summary
- Adds three Criterion benchmarks to etch-lib: YAML manifest parse, TOML manifest parse, file path resolution
- Adds monthly `benchmarks.yml` CI workflow that publishes results to gh-pages
- Adds `make bench` target

## Test plan
- [ ] `cargo bench -p etch-lib` runs locally without panics
- [ ] `make bench` runs without error
- [ ] CI workflow passes on `workflow_dispatch` trigger after merge
- [ ] GitHub Pages at `https://brujack.github.io/etch-cli/dev/bench/` shows charts
EOF
)"
```

- [ ] **Step 2: Monitor CI**

```bash
gh pr checks <number> --watch --repo brujack/etch-cli
```

Wait for all checks to pass.

- [ ] **Step 3: Post-merge docs update — do this on main after the PR merges, NOT inside the worktree**

Add `> **Status: DONE**` banner at the top of this plan file (below the header line, before the Goal line):

```markdown
> **Status: DONE**
```

Update `docs/superpowers/README.md` in the etch-cli repo — change the criterion-benchmarks row status from `In Progress` to `Done`.

Commit directly to main:

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-23-criterion-benchmarks.md
git commit -m "docs: mark criterion-benchmarks plan Done"
git push --repo brujack/etch-cli
```

Also update `docs/superpowers/README.md` in ai-config to mark the criterion-benchmarks spec row Done.
