# pyenv.virtualenv `recreate:` field — Design Spec

**Date:** 2026-06-09
**Status:** Pending implementation

## Problem

`pyenv.virtualenv` is idempotent: it skips creation if the named virtualenv already
exists. When `python_version` bumps (e.g. `3.14.5` → `3.14.6`), `etch apply` emits
no steps — the existing virtualenv silently keeps the old interpreter. The user must
manually run `pyenv uninstall {name}` before `etch apply` picks up the new version.

**Affected manifest:** `etch-config/workstation/ansible.yaml` — `ansible` virtualenv
must track `python_version` exactly for pip packages that compile C extensions.

## Decision

Add a `recreate: bool` field (default `false`) to the existing `pyenv.virtualenv`
action. When `true`, `plan()` detects version mismatch and emits an uninstall step
before the create step.

No new action type. `recreate: false` (default) is byte-for-byte backward compatible.

## Architecture

### Struct change

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyenvVirtualenv {
    pub python_version: Option<String>,
    pub name: Option<String>,
    /// When true, delete and recreate the virtualenv if its Python version
    /// differs from `python_version`. Default false preserves existing behavior.
    #[serde(default)]
    pub recreate: bool,
}
```

### Version detection

pyenv virtualenv creates `~/.pyenv/versions/{name}` as a symlink to
`~/.pyenv/versions/{python_version}/envs/{name}`. Resolving the symlink with
`std::fs::read_link` and inspecting the path components yields the installed
Python version without spawning a subprocess.

```rust
fn installed_python_version(name: &str) -> Option<String> {
    let link = shellexpand::tilde(&format!("~/.pyenv/versions/{name}"));
    let target = std::fs::read_link(link.as_ref()).ok()?;
    // target path: .../versions/3.14.5/envs/ansible
    // return the component immediately before the "envs" component
    target
        .components()
        .zip(target.components().skip(1))
        .find_map(|(a, b)| {
            if b.as_os_str() == "envs" {
                Some(a.as_os_str().to_string_lossy().into_owned())
            } else {
                None
            }
        })
}
```

Returns `None` when the path is not a symlink (venv absent or non-standard layout).

### plan() logic

**When `recreate: false` (default):** unchanged — skip if venv name appears in
`pyenv virtualenvs --bare` output.

**When `recreate: true`:**

| venv exists | version matches `python_version` | Steps emitted                                                           |
| ----------- | -------------------------------- | ----------------------------------------------------------------------- |
| no          | —                                | `[pyenv virtualenv {python_version} {name}]`                            |
| yes         | yes                              | `[]`                                                                    |
| yes         | no (or version undetectable)     | `[pyenv uninstall -f {name}, pyenv virtualenv {python_version} {name}]` |

When version is undetectable (symlink absent, non-standard layout), emit recreate
steps as a fail-safe — same as the existing policy for `virtualenv_exists` failures.

The `pyenv uninstall -f` step uses the `-f` flag to suppress the interactive
confirmation prompt.

### etch status

`status()` remains `Unchecked` (default). Drift detection for virtualenv Python
version is out of scope — the `etch status` framework requires a sidecar or
queryable state; adding it here would bloat the scope. Version drift is surfaced
implicitly by `etch apply --dry-run` showing the recreate steps.

## Testing

All new tests use fake pyenv binaries and `std::os::unix::fs::symlink` for
filesystem setup — no real pyenv required.

**Unit tests for `installed_python_version`:**

- `installed_python_version_returns_none_when_path_not_symlink` — regular dir
- `installed_python_version_returns_version_from_symlink` — symlink at expected path

**New `plan()` tests (recreate: true):**

- `plan_recreate_true_creates_when_no_venv` — no venv → 1 step (create)
- `plan_recreate_true_skips_when_version_matches` — symlink → correct version → 0 steps
- `plan_recreate_true_recreates_when_version_differs` — symlink → wrong version → 2 steps
- `plan_recreate_true_recreates_when_version_undetectable` — no symlink but venv exists in bare list → 2 steps (fail-safe)

**Regression tests (recreate: false / omitted):**

- `plan_recreate_false_skips_existing_venv` — confirms default behavior unchanged
- Existing 11 tests must continue to pass unchanged.

## Files modified

- `lib/src/actions/pyenv/virtualenv.rs` — only file changed
- `examples/pyenv/pyenv-virtualenv.yaml` — add `recreate: true` variant

## Out of scope

- `etch status` drift detection for virtualenv version
- Support for non-default `PYENV_ROOT`
- Updating `etch-config` manifests (done after etch-cli ships)
