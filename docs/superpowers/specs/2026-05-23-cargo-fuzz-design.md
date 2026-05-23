# cargo-fuzz for etch-cli

## Context

etch-cli processes two categories of user-supplied input that are worth fuzzing:

1. **Manifest files** — YAML/TOML files deserialized via serde and rendered through Tera templating. Malformed input could cause panics in the parser, type coercion, or template engine.
2. **File paths** — arbitrary strings passed to `FileAction::resolve()`, which joins the path onto `manifest.root_dir/files/` and normalizes the result. Unexpected input could cause panics in the path normalization logic.

Property tests and unit tests cover known edge cases. Fuzzing covers unknown inputs that property tests never generate — malformed UTF-8 sequences, deeply nested structures, path traversal strings, and parser edge cases.

## Decision

Add two cargo-fuzz targets to `etch-cli-lib`, both triggered manually via Makefile. No CI scheduling — runs are too slow for every PR and the value is in periodic discovery sessions.

## Architecture

### Fuzz crate

```
fuzz/
  Cargo.toml               # fuzz crate, workspace member, depends on etch-cli-lib
  fuzz_targets/
    fuzz_manifest.rs       # manifest deserialization target
    fuzz_path_resolve.rs   # file path resolution target
  corpus/
    fuzz_manifest/         # seed inputs + discovered corpus (committed)
    fuzz_path_resolve/     # seed inputs + discovered corpus (committed)
```

The `fuzz/` directory is a standard cargo-fuzz layout. It is a workspace member so `cargo fuzz` can find it, but excluded from normal `cargo build` and `cargo test` runs.

### fuzz_manifest target

Feeds arbitrary bytes as both YAML and TOML to `serde_yaml::from_str::<Manifest>` and `toml::from_str::<Manifest>`. Any panic is a bug. `Result::Err` (parse failure) is expected and ignored — the goal is to find panics, not parse errors.

```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_yaml::from_str::<Manifest>(s);
        let _ = toml::from_str::<Manifest>(s);
    }
});
```

### fuzz_path_resolve target

Builds a minimal `Manifest` with `root_dir` set to a temporary directory, then calls `FileLink::resolve(manifest, input_str)` (using the `file.link` action as a representative `FileAction` implementor). Any panic is a bug. `Result::Err` is expected and ignored.

```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let manifest = stub_manifest();
        let action = FileLink::default();
        let _ = action.resolve(&manifest, s);
    }
});
```

`stub_manifest()` creates a `Manifest` with `root_dir` pointing to a stable temp path — no filesystem writes, no side effects.

### Corpus seeding

`fuzz/corpus/fuzz_manifest/` is seeded with 3–5 real manifest examples from `examples/`. `fuzz/corpus/fuzz_path_resolve/` is seeded with a handful of path strings (relative, absolute, `..` traversal, empty, Unicode). Committed so future runs start from meaningful inputs rather than random bytes.

### Makefile targets

```makefile
FUZZ_TIMEOUT ?= 60

fuzz-manifest:
    cargo fuzz run fuzz_manifest fuzz/corpus/fuzz_manifest -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz-path:
    cargo fuzz run fuzz_path_resolve fuzz/corpus/fuzz_path_resolve -- -max_total_time=$(FUZZ_TIMEOUT)

fuzz: fuzz-manifest fuzz-path
```

Default timeout is 60 seconds per target. Override: `make fuzz FUZZ_TIMEOUT=300`.

## Constraints

- cargo-fuzz requires nightly Rust (`cargo +nightly fuzz run ...`). Makefile targets use `cargo +nightly`.
- libFuzzer runs on Linux and macOS (both supported).
- `fuzz/` is excluded from `cargo test`, `cargo clippy`, and coverage runs — it is not a production crate.
- Artifacts (crashes, slow inputs) written to `fuzz/artifacts/` — gitignored, not committed.

## Success Criteria

- `make fuzz` runs both targets for the default timeout without itself crashing
- Discovered corpus entries are written to `fuzz/corpus/*/` and can be committed to preserve interesting inputs
- Any crash found produces a reproducible artifact in `fuzz/artifacts/`
