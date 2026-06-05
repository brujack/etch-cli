> **Status: DONE** — Merged in PR #87 (2026-06-05)

# Package Streaming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real-time terminal output for package install/upgrade/remove operations by adding `streaming: bool` to the `Exec` atom.

**Architecture:** `Exec` branches on `self.streaming` — `false` preserves existing `.output()` path unchanged; `true` uses `.spawn()` + `Stdio::inherit()` + `child.wait()` so the package manager's output flows directly to the terminal.

**Tech Stack:** Rust, std::process::Command, serial_test crate for test isolation

---

### Task 1: Add `streaming: bool` to `Exec` + unit tests

**Files:**

- Modify: `lib/src/atoms/command/exec.rs`

All tests use `#[serial]` from `serial_test` crate.

- [x] Write failing test `streaming_false_captures_output`
- [x] Write failing test `streaming_true_output_string_empty`
- [x] Write failing test `streaming_true_succeeds`
- [x] Write failing test `streaming_true_fails_on_nonzero`
- [x] Add `pub streaming: bool` field to `Exec` struct
- [x] Branch `execute()` on `self.streaming` — streaming path uses `spawn()+Stdio::inherit()+wait()`
- [x] Set `self.status.code` on streaming failure path
- [x] All tests pass; commit

### Task 2: aptitude.rs — stream mutating atoms

**Files:**

- Modify: `lib/src/actions/package/providers/aptitude.rs`

- [x] Set `streaming: true` on bootstrap apt install, apt-add-repository, apt update, install() apt install, apt_version_step

### Task 3: homebrew.rs — stream mutating atoms

**Files:**

- Modify: `lib/src/actions/package/providers/homebrew.rs`

- [x] Set `streaming: true` on bootstrap bash install, add_repository brew tap, install() brew install, brew_version_step

### Task 4: snapcraft.rs — stream install atoms

**Files:**

- Modify: `lib/src/actions/package/providers/snapcraft.rs`

- [x] Set `streaming: true` on bootstrap snap install, install() snap install, snap_version_step

### Task 5: apt_upgrade.rs — stream upgrade step

**Files:**

- Modify: `lib/src/actions/package/providers/apt_upgrade.rs`

- [x] Set `streaming: true` on apt-get upgrade (leave apt-get update buffered)

### Task 6: snap_upgrade.rs — stream refresh atoms

**Files:**

- Modify: `lib/src/actions/package/providers/snap_upgrade.rs`

- [x] Set `streaming: true` on snap refresh (targeted) and snap refresh (all); leave snap refresh --list and snap list buffered

### Task 7: autoremove.rs — stream autoremove atom

**Files:**

- Modify: `lib/src/actions/package/autoremove.rs`

- [x] Set `streaming: true` on apt autoremove --yes

### Task 8: PR, CI, merge, cleanup

- [x] Open PR #87 against main
- [x] CI passes (Test + coverage ≥81%)
- [x] PR auto-merges
- [x] Worktree removed, branches cleaned up
- [x] Docs updated (this file + index)
