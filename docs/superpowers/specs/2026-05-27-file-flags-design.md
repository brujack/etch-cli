# file.flags Action Design

## Overview

Add a `file.flags` action for setting and clearing BSD file flags (macOS only). Covers the four flags needed for dotfiles management: `hidden`, `nohidden`, `uchg`, `nouchg`.

## Motivation

`file.chmod` sets POSIX permission bits only. BSD file flags (`chflags`) are a separate macOS mechanism: `UF_HIDDEN` controls Finder visibility, `UF_IMMUTABLE` prevents modification or deletion. No existing etch action covers these.

## Manifest Syntax

```yaml
# Unhide ~/Library (macOS hides it by default)
- action: file.flags
  path: "{{ user.home_dir }}/Library"
  flags: [nohidden]
  where: 'os.name == "macos"'

# Protect SSH private key from accidental modification
- action: file.flags
  path: "{{ user.home_dir }}/.ssh/id_ed25519"
  flags: [uchg]
  where: 'os.name == "macos"'

# Clear immutable before modifying, then re-set
- action: file.flags
  path: "{{ user.home_dir }}/.ssh/id_ed25519"
  flags: [nouchg]
  where: 'os.name == "macos"'

# Combine: clear hidden AND set immutable in one call
- action: file.flags
  path: "{{ user.home_dir }}/important-dir"
  flags: [nohidden, uchg]
  where: 'os.name == "macos"'
```

**Note:** `path:` does not expand `~`. Use `{{ user.home_dir }}/...` for home-relative paths (consistent with `file.chmod` behavior).

## Flag Semantics

`flags:` is an explicit delta — each entry sets or clears one bit; bits not mentioned are left untouched.

| Flag       | Bit constant   | Value  | Effect               |
| ---------- | -------------- | ------ | -------------------- |
| `hidden`   | `UF_HIDDEN`    | 0x8000 | set hidden bit       |
| `nohidden` | `UF_HIDDEN`    | 0x8000 | clear hidden bit     |
| `uchg`     | `UF_IMMUTABLE` | 0x0002 | set user immutable   |
| `nouchg`   | `UF_IMMUTABLE` | 0x0002 | clear user immutable |

Unknown flag names fail at `plan()` time with a clear error before any state is changed.

## Architecture

### New files

**`lib/src/atoms/file/chflags.rs`** (`#[cfg(target_os = "macos")]`)

```rust
pub struct Chflags {
    pub path: PathBuf,
    pub flags: Vec<String>,
}
```

`execute()` logic:

1. Read current `st_flags` via `libc::stat()`
2. Compute desired flags: start from current, apply each entry (set or clear bit)
3. If `current == desired`: return `Ok(false)` — no change
4. Call `libc::chflags(path, desired)` — return `Ok(true)`

**`lib/src/actions/file/flags.rs`**

```rust
pub struct FileFlags {
    pub path: String,
    pub flags: Vec<String>,
    pub config: FileActionConfig,   // privileged: bool
}
```

`plan()` logic — uses `#[cfg]` to avoid referencing `Chflags` on non-macOS:

```rust
fn plan(&self, ...) -> anyhow::Result<Vec<Step>> {
    #[cfg(not(target_os = "macos"))]
    return Err(anyhow!("file.flags is only supported on macOS"));

    #[cfg(target_os = "macos")]
    {
        // validate all flags — error on unknown
        // if privileged: false → return Step { atom: Box::new(Chflags { path, flags }) }
        // if privileged: true → read st_flags via libc::stat(), compute desired,
        //   if different: return Step { atom: Box::new(Exec { command: "chflags",
        //                              arguments: [flags_str, path], privileged: true }) }
        //   if same: return Ok(vec![])
    }
}
```

### Modified files

**`lib/Cargo.toml`** — add `libc = "0.2"`

**`lib/src/atoms/file/mod.rs`** — add `#[cfg(target_os = "macos")] pub mod chflags;`

**`lib/src/actions/file/mod.rs`** — add `pub mod flags;`

**`lib/src/actions/mod.rs`** — register `Actions::FileFlags`, add to all match arms

## Error Handling

| Condition                      | Behavior                                                  |
| ------------------------------ | --------------------------------------------------------- |
| Unknown flag name              | `plan()` fails: `"unknown flag: <name>"`                  |
| Path not found                 | `libc::stat()` → ENOENT → error with path                 |
| Non-macOS                      | `plan()` fails: `"file.flags is only supported on macOS"` |
| EPERM (e.g. schg without root) | `libc::chflags()` error propagated with path + errno      |
| Already at desired state       | `Ok(false)` — no change, handlers not triggered           |

## Testing

**Unit tests (`flags.rs`):**

- Deserialization: `path`, `flags`, `privileged` fields parse correctly
- Unknown flag name errors at plan time
- Non-macOS path (mocked via `#[cfg(not(target_os = "macos"))]`) returns error

**Atom tests (`chflags.rs`, `#[cfg(target_os = "macos")]`):**

- Current flags == desired → `Ok(false)`
- Current flags differ → `Ok(true)` + flags applied
- Unknown flag in compute step → error

**Integration test (`app/tests/integration.rs`, `#[cfg(target_os = "macos")]`):**

- Apply `file.flags` with `flags: [hidden]` on a tempfile
- Verify via `ls -lO` output contains `hidden`
- Apply again → `Ok(false)` (idempotent)
- Apply `file.flags` with `flags: [nohidden]` → verify flag cleared

## Platform Notes

- macOS only. The `Chflags` atom is `#[cfg(target_os = "macos")]`.
- `where: 'os.name == "macos"'` is the intended gate in manifests.
- If applied on Linux without a `where:` guard, `plan()` returns a clear error.
- BSD flags are a separate namespace from POSIX permission bits — `file.chmod` and `file.flags` are independent and composable.
