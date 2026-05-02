# Platform Pruning — Design Spec

**Date:** 2026-05-02
**Status:** Pending implementation

## Goal

Reduce etch-cli to exactly the platforms it runs on: Ubuntu 24.04, Ubuntu 26.04, and macOS. Delete all code that supports other operating systems rather than leaving stubs or feature-gating it.

## Supported targets after this change

| OS                   | Package managers                 |
| -------------------- | -------------------------------- |
| Ubuntu 24.04 / 26.04 | apt (aptitude), snap (snapcraft) |
| macOS (aarch64)      | homebrew                         |

## Approach

Hard delete — remove files and references entirely. No stubs, no `unimplemented!()`, no Cargo feature gates. The codebase should honestly reflect the two machine types it runs on.

## Section 1: Files to delete

```
lib/src/actions/package/providers/bsdpkg.rs
lib/src/actions/package/providers/dnf.rs
lib/src/actions/package/providers/macports.rs
lib/src/actions/package/providers/paru.rs
lib/src/actions/package/providers/pkgin.rs
lib/src/actions/package/providers/winget.rs
lib/src/actions/package/providers/xbps.rs
lib/src/actions/package/providers/yay.rs
lib/src/actions/package/providers/zypper.rs
lib/src/actions/group/providers/freebsd.rs
lib/src/actions/user/providers/freebsd.rs
```

## Section 2: Provider registry updates

### `lib/src/actions/package/providers/mod.rs`

- Remove all `mod` declarations and `use` imports for the 9 deleted providers
- `PackageProviders` enum shrinks to 3 variants: `Aptitude`, `Homebrew`, `Snapcraft`
- `get_provider()` match shrinks to 3 arms
- `Default` impl retains only:
    - `os_info::Type::Ubuntu` → `Aptitude` (primary Linux target)
    - `os_info::Type::Macos` → `Homebrew`
    - All other `os_info::Type` arms removed, including `Debian`, `Mint`, `Pop`, `OracleLinux` — they are not target machines and removing them keeps the match honest
    - Panic message updated to: `"Unsupported OS. Use provider: apt, snap, or brew explicitly."`

### `lib/src/actions/group/providers/mod.rs`

- Remove `mod freebsd` and `use FreeBSDGroupProvider`
- `GroupProviders` enum drops the `FreeBSD` variant
- `get_provider()` match drops the `FreeBSD` arm
- `Default` non-linux branch: remove `FreeBSD` arm, leaving only `Macos → MacOs, _ => None`

### `lib/src/actions/user/providers/mod.rs`

- Same pattern as group: drop `FreeBSD` variant, arm, and `Default` mapping

## Section 3: Platform guard cleanup

### `lib/src/atoms/file/link.rs`

- Remove the `if cfg!(target_os = "windows")` path-prefix branch in `plan()`; source path becomes unconditional `self.source.to_owned()`
- Delete the `#[cfg(windows)]` `execute()` impl
- Drop the `#[cfg(unix)]` guard on the remaining `execute()` impl (it is now the only impl)

### `lib/src/actions/file/chown.rs`

- Delete the `#[cfg(not(unix))]` plan stub that logs the Windows warning
- Drop the `#[cfg(unix)]` guard on the remaining `plan()` impl

### `lib/src/actions/directory/copy.rs`

- Delete the `#[cfg(target_family = "windows")]` Xcopy impl block
- Drop the `#[cfg(target_family = "unix")]` guard on the remaining impl

### `lib/src/contexts/os.rs`

- Remove the `#[cfg(windows)]` test block
- Remove the `#[cfg(target_os = "freebsd")]` test block

### `lib/src/steps/initializers/command_found.rs`

- Remove the two `#[cfg(target_family = "windows")]` test blocks (`cmd.exe` and `Xcopy`)

### `lib/src/manifests/mod.rs`

- Remove the `#[cfg(windows)]` test module containing the `C:\` path test

## Section 4: Verification

- `make test` must pass (cargo fmt --check + clippy -D warnings + cargo test)
- `cargo tarpaulin --fail-under 25` must pass — coverage will likely increase as dead/untested code is removed
- No CI changes required; the workflow already runs on `ubuntu-latest`
- `jsonschemagen` schema output will change (9 removed package provider variants) — correct behavior, no action needed

## Out of scope

- Adding new actions or features
- Changing the behavior of retained providers
- Updating documentation or examples beyond what breaks from compilation
