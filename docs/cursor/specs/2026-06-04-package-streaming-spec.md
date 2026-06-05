# Spec: package.install / upgrade / autoremove Progress Streaming

**Date:** 2026-06-04
**Status:** Done — implemented in PR #87

## Problem

`Exec::execute()` uses `.output()` — stdout and stderr are buffered until the command exits. Long package installs (apt, brew, snap) produce no terminal output during execution, making the tool appear hung.

## Solution

Add a `streaming: bool` field to `Exec`. When `true`, `execute()` uses `spawn()` + `Stdio::inherit()` so the package manager's output flows directly to the terminal in real time.

## Scope

Applies to `package.install`, `package.upgrade`, and `package.autoremove` — all three have the same hang problem. Check commands used for idempotency are excluded.

---

## `Exec` changes (`lib/src/atoms/command/exec.rs`)

Add field to struct:

```rust
pub streaming: bool,   // default false — zero behavior change for existing atoms
```

In `execute()`, branch on `self.streaming`:

**`streaming: false` (unchanged):**
`.output()` → capture stdout/stderr into `self.status` → `debug!` log on success.

**`streaming: true`:**
`.spawn()` with `Stdio::inherit()` for stdout and stderr → `child.wait()` → check exit status.
`output_string()` returns `""`. `error_message()` returns `""`.
No callers of install/upgrade/autoremove atoms read captured output.

Privilege elevation is unchanged: `elevate()` (`sudo --validate` with inherited IO) still runs before the actual command when the privilege provider is detected.

---

## Provider changes

Set `streaming: true` on install/upgrade/remove atoms only. Check commands stay `false`.

| File                                | Atoms to stream                         |
| ----------------------------------- | --------------------------------------- |
| `package/providers/aptitude.rs`     | apt install, apt remove, apt autoremove |
| `package/providers/homebrew.rs`     | brew install, brew uninstall            |
| `package/providers/snapcraft.rs`    | snap install                            |
| `package/providers/apt_upgrade.rs`  | apt upgrade                             |
| `package/providers/snap_upgrade.rs` | snap refresh                            |
| `package/autoremove.rs`             | apt autoremove                          |

Check commands that remain `streaming: false`: `apt list --installed`, `dpkg-query`, `brew list`, `snap list`.

---

## Error handling

Streaming path error shape matches non-streaming:

- Exit 0 → `Ok(())`
- Non-zero exit → `Err(anyhow!("Command failed with exit code: {}", code))`
- Spawn failure → `Err(anyhow!(err))`

`self.status.code` is set on failure so callers can inspect it.

---

## Testing

### `exec.rs` unit tests

| Test                                 | What it verifies                                                      |
| ------------------------------------ | --------------------------------------------------------------------- |
| `streaming_false_captures_output`    | `echo hello`, `streaming: false` → `output_string()` contains "hello" |
| `streaming_true_output_string_empty` | `echo hello`, `streaming: true` → `output_string() == ""`             |
| `streaming_true_succeeds`            | `true` command, `streaming: true` → `Ok(())`                          |
| `streaming_true_fails_on_nonzero`    | `false` command, `streaming: true` → `Err` + status.code == 1         |

All streaming tests annotated `#[serial]`.

### Provider plan tests

No field-level assertions needed in provider tests — adding `as_any()` to the `Atom` trait would require updating all 24 implementors. The `Exec` unit tests above cover streaming behavior completely.

---

## What does NOT change

- Non-package atoms (`command.run`, `file.*`, `git.*`, etc.) — unaffected
- `output_string()` contract for non-streaming atoms — unchanged
- Idempotency check commands — unchanged
- Test coverage gate (81% Linux CI) — no structural change to coverable paths
