# etch doctor — Design Spec

**Date:** 2026-06-06
**Status:** Approved

## Summary

Add an `etch doctor` subcommand that validates system health: symlink integrity, tool existence in PATH, credential directory permissions, and version drift. Complements `etch status` (manifest drift) by covering system-level invariants.

## Invocation

```
etch doctor [--json] [--missing-only]
```

Exit code `0` if all checks pass, `1` if any fail.

## Architecture

Split across lib and app following existing patterns:

```
lib/src/doctor/mod.rs          DoctorCheck trait + CheckResult type
lib/src/doctor/symlinks.rs     SymlinkCheck — file.link targets exist
lib/src/doctor/tools.rs        ToolsCheck — tools in PATH
lib/src/doctor/cred_perms.rs   CredPermsCheck — dirs mode 700
lib/src/doctor/versions.rs     VersionsCheck — binary versions match pins
app/src/commands/doctor.rs     Doctor struct + EtchCommand impl + output
lib/src/config/mod.rs          DoctorConfig added to Config
```

## Check Trait

```rust
pub struct CheckResult {
    pub label: String,
    pub passed: bool,
    pub detail: Option<String>,  // failure reason; None on pass
}

pub trait DoctorCheck {
    fn run(&self, config: &Config, manifests: &Manifests) -> Vec<CheckResult>;
}
```

All four checks implement `DoctorCheck`. The command runs them in order and collects results.

## Config Extension

New optional field in `etch.yaml`:

```yaml
doctor:
    tools: # explicit tools beyond manifest-derived
        - kubectl
        - helm
    versions: # explicit version pins
        - tool: ripgrep
          command: "rg --version"
          expected: "14.1.0"
    credential_dirs: # dirs to verify are mode 700
        - ~/.ssh
        - ~/.tf_creds
        - ~/.tsh
```

`Config` struct gains `doctor: Option<DoctorConfig>`. Missing = no explicit checks (manifest-derived checks still run).

```rust
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema, Clone)]
pub struct DoctorConfig {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub versions: Vec<VersionPin>,
    #[serde(default)]
    pub credential_dirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct VersionPin {
    pub tool: String,
    pub command: String,
    pub expected: String,
}
```

## Four Checks

### SymlinkCheck

Iterates all `file.link` actions across loaded manifests. For each action, calls `plan()` to extract the target path, then checks `std::fs::symlink_metadata(target).is_ok()` to verify the symlink destination exists.

One `CheckResult` per symlink. Label: `source → target`. Failure detail: `"target does not exist"`.

No config fields — entirely manifest-derived.

### ToolsCheck

Two sources merged and deduped before checking:

**Manifest-derived** — maps action type to implied tool:

| Action                                                       | Tool           |
| ------------------------------------------------------------ | -------------- |
| `brew.bundle` / `brew.upgrade` / `brew.cleanup`              | `brew`         |
| `gem.install`                                                | `gem`          |
| `pip.install`                                                | `pip`          |
| `npm.install`                                                | `npm`          |
| `mas.install` / `mas.upgrade`                                | `mas`          |
| `pyenv.install` / `pyenv.virtualenv`                         | `pyenv`        |
| `ruby.install`                                               | `ruby-install` |
| `claude.install` / `claude.upgrade` / `claude.plugin.update` | `claude`       |

**Explicit** — `config.doctor.tools` list.

Each tool checked via `which::which(name).is_ok()`. One `CheckResult` per tool. Failure detail: `"not found in PATH"`.

### CredPermsCheck

Iterates `config.doctor.credential_dirs`. For each path:

1. Shell-expand `~` via `shellexpand::tilde`.
2. If path does not exist → skip (not a failure; machine may not have that credential type).
3. `std::fs::metadata(path)?.permissions().mode() & 0o777`
4. Assert `== 0o700`. Failure detail: `"mode NNN, expected 700"`.

One `CheckResult` per existing dir. Nonexistent dirs produce no result.

### VersionsCheck

Two sources:

**binary.github / binary.url atoms** — scans manifests for `binary.github` and `binary.url` actions where `version:` is set. Runs `<binary-name> --version`, checks output contains the pinned version string. Binary name derived from the action's `name` field.

**Explicit** — `config.doctor.versions` list. Runs `command` field verbatim via `std::process::Command::new("sh").args(["-c", command])`, checks output contains `expected`.

Version match: `output.contains(expected)` — handles `rg 14.1.0 (rev abc1234)` style version strings.

Failure detail: `"got \"<first line of output>\", expected \"<expected>\""`. If binary not found: `"command not found"`.

## Command (app layer)

```rust
#[derive(Parser, Debug, Default)]
/// Check system health
pub struct Doctor {
    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Only show failing checks
    #[arg(long)]
    pub missing_only: bool,
}
```

Registered in `Commands` enum:

```rust
/// Check system health
Doctor(commands::Doctor),
```

### EtchCommand impl

```rust
impl EtchCommand for Doctor {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let results = run_doctor(&self, runtime)?;
        let any_failed = results.iter().any(|r| !r.passed);
        // render output
        if any_failed { std::process::exit(1); }
        Ok(())
    }
}
```

`run_doctor` loads manifests, builds the four checks, runs each, and returns a flat `Vec<CheckResult>`.

### Human output

```
Symlinks
  ✓ ~/.dotfiles/zshrc → ~/.zshrc
  ✗ ~/.dotfiles/gitconfig → ~/.gitconfig  [target does not exist]

Tools
  ✓ brew
  ✗ mas  [not found in PATH]

Credential dirs
  ✓ ~/.ssh (700)
  ✗ ~/.tf_creds  [mode 755, expected 700]

Versions
  ✓ rg 14.1.0
  ✗ etch  [got "0.11.0", expected "0.12.0"]

4 passed, 3 failed
```

`--missing-only` suppresses passing checks and section headers with no failures.

### JSON output

```json
{
    "checks": [
        {
            "label": "~/.dotfiles/zshrc → ~/.zshrc",
            "passed": true,
            "detail": null
        },
        { "label": "mas", "passed": false, "detail": "not found in PATH" }
    ],
    "summary": { "passed": 4, "failed": 3 }
}
```

## Testing

### Unit tests in `lib/src/doctor/`

**SymlinkCheck:**

- Real symlink → existing file → `passed: true`
- Symlink → nonexistent path → `passed: false`, detail contains "does not exist"
- Manifest with no `file.link` actions → empty results

**ToolsCheck:**

- Tool present in injected PATH → `passed: true`
- Tool absent from PATH → `passed: false`
- Manifest-derived tool inferred correctly for each action type
- Explicit and manifest-derived lists merged and deduped

**CredPermsCheck:**

- Dir mode 0o700 → `passed: true`
- Dir mode 0o755 → `passed: false`, detail contains "755"
- Nonexistent dir → no result (skipped, not failed)

**VersionsCheck:**

- Fake binary in PATH outputs expected string → `passed: true`
- Fake binary outputs different version → `passed: false`, detail contains actual output
- Binary not found → `passed: false`, detail contains "not found"

### Integration tests in `app/tests/doctor.rs`

- `etch doctor --help` renders expected flags
- `etch doctor` with empty config and no manifests → exit 0
- `etch doctor` with broken symlink in temp manifest → exit 1
- `etch doctor --json` with failures → valid JSON, `"passed": false`
- `etch doctor --missing-only` suppresses passing checks

### Snapshot test

Add `etch doctor --help` to `app/tests/snapshots.rs`.

## Out of Scope

- Fixing detected issues (doctor reports only — no auto-remediation)
- Parallel check execution
- Watching for changes (`--watch` mode)
- Checks beyond the four defined above
