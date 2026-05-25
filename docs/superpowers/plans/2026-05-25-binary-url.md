# binary.url Implementation Plan

> **Status: DONE** — Implemented in PR #44 (2026-05-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `binary.url` action that downloads and installs binaries from arbitrary URLs with optional sha256 verification and archive extraction (tar.gz, tar.xz, zip, raw).

**Architecture:** New `BinaryUrl` action in `lib/src/actions/binary/url.rs`. Two new atoms: `BinaryVerify` (sha256 check) and `BinaryExtract` (format-aware install). Step sequence: Download → optional BinaryVerify → BinaryExtract → Chmod(0o755). Idempotency: skip if `{directory}/{name}` already exists.

**Tech Stack:** Rust; existing `sha256 = "1.6"`, `tar = "0.4"`, `flate2`; new `zip = "2"`, `xz2 = "0.1"`.

---

### Task 1: Cargo deps + binary atom module scaffold ✅

Add `zip = "2"` and `xz2 = "0.1"` to `lib/Cargo.toml`. Create `lib/src/atoms/binary/mod.rs` with `mod extract; mod verify; pub use ...`. Add `pub mod binary;` to `lib/src/atoms/mod.rs`.

### Task 2: BinaryVerify atom ✅

`lib/src/atoms/binary/verify.rs` — sha256 check using `sha256::try_digest()`. Error includes both expected and actual hashes.

### Task 3: BinaryExtract — format detection + Raw ✅

`lib/src/atoms/binary/extract.rs` — `ArchiveFormat::detect()` from URL suffix; Raw arm uses `fs::rename`.

### Task 4: BinaryExtract — TarGz ✅

TarGz arm: `GzDecoder` + `tar::Archive`, iterate entries, extract named `file` to dest.

### Task 5: BinaryExtract — TarXz ✅

TarXz arm: same pattern with `XzDecoder`.

### Task 6: BinaryExtract — Zip + missing-file errors ✅

Zip arm: `ZipArchive::by_name()`. All archive arms return error if `file` is `None`.

### Task 7: BinaryUrl struct + URL rendering + summarize ✅

`lib/src/actions/binary/url.rs` — struct fields, `render_url()` via `Tera::default()`, `summarize()`.

### Task 8: BinaryUrl plan() — idempotency + raw binary ✅

`plan()` returns empty vec if dest exists. Step sequence for Raw: Download → BinaryVerify (optional) → BinaryExtract → Chmod(0o755).

### Task 9: BinaryUrl plan() — archive + error cases ✅

`plan()` validates `file` is set for non-Raw archives at plan time.

### Task 10: Register BinaryUrl in actions mod.rs ✅

Added `BinaryUrl` to `Actions` enum, `inner_ref()`, `Deref`, and label. Registered as `binary.url` / `bin.url`.

### Task 11: Open PR, monitor CI, merge ✅

PR #44 merged. All CI checks passed (484 tests). Semver Check advisory failure (non-blocking).

### Task 12: Post-merge docs on main ✅

Update `docs/superpowers/README.md`, add DONE banner, update `CLAUDE.md` action catalog. Do directly on main after PR merges — not inside the worktree.
