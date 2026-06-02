# Spec: etch status integration tests

## Problem

`app/src/commands/status.rs` is 0% covered (78 lines). `run_status()` is never exercised — no integration tests spawn `etch status`. This is the biggest single coverage gap on Linux CI.

## Design

### Approach

New file `app/tests/status.rs`. Spawns the `etch` binary via `assert_cmd::Command::cargo_bin("etch")` with temp dirs constructed using the existing `utils` helpers. Follows the same pattern as `app/tests/basic_usage.rs`.

The checkable atom used throughout is `file.link` — it has a real `status()` implementation that returns `Ok`, `Missing`, or `Drifted`. `command.run` is used for the `Unchecked` case.

### Test inventory

| Test                                     | Manifest                                     | Filesystem state             | Expected exit | Key stdout assertion                                   |
| ---------------------------------------- | -------------------------------------------- | ---------------------------- | ------------- | ------------------------------------------------------ |
| `status_exits_zero_when_all_atoms_ok`    | `file.link` source→target                    | symlink in place             | 0             | contains "ok"                                          |
| `status_exits_nonzero_when_atom_missing` | `file.link` source→target                    | no symlink                   | non-zero      | contains "missing"                                     |
| `status_exits_nonzero_when_atom_drifted` | `file.link` source→target                    | symlink points to wrong file | non-zero      | contains "drifted"                                     |
| `status_unchecked_atom_exits_zero`       | `command.run echo hello`                     | —                            | 0             | contains "unchecked" or "ok"                           |
| `status_json_flag_produces_valid_json`   | `file.link` + symlink in place               | symlink ok                   | 0             | output is valid JSON with `manifests` + `summary` keys |
| `status_json_missing_has_nonzero_exit`   | `file.link`                                  | no symlink                   | non-zero      | JSON `summary.missing == 1`                            |
| `status_where_false_skips_manifest`      | manifest with `where: 'false'` + `file.link` | no symlink                   | 0             | manifest name absent from stdout                       |

### CLI invocation pattern

```
etch --no-color -d <tmpdir> status -m <manifest_name>
```

Mirrors the existing `apply` tests: `cd(path).run("--no-color -d ./directory status -m <name>")`.

### File structure per test (temp dir layout)

```
<tmpdir>/
  directory/
    <manifest>/
      main.yaml          ← the manifest
      files/
        source.txt       ← source file for file.link tests
```

The symlink target lives outside `files/` at `<tmpdir>/target_link` (or inside the manifest dir), created via `std::os::unix::fs::symlink`.

### Manifest YAML — file.link (ok case)

```yaml
actions:
    - action: file.link
      source: source.txt
      target: TARGET_PATH # replaced at test time with tmpdir.path().join("target_link").display()
```

`TARGET_PATH` is substituted in each test by formatting the manifest YAML string with the actual `TempDir` path before writing the file. This keeps paths unique per test run and ensures cleanup.

### Manifest YAML — command.run (unchecked case)

```yaml
actions:
    - action: command.run
      command: echo
      args:
          - hello
```

### JSON output structure

```json
{
    "manifests": [
        {
            "name": "mymanifest",
            "atoms": [{ "label": "...", "status": "ok" }]
        }
    ],
    "summary": {
        "ok": 1,
        "unchecked": 0,
        "drifted": 0,
        "missing": 0
    }
}
```

Parse with `serde_json::from_str::<serde_json::Value>` in tests.

### Error handling

No new error handling needed — tests assert on exit code and stdout. `run_status()` error paths (action plan failure, atom status check failure) emit `warn!` and continue; those paths remain uncovered but are lower-priority than the happy/missing/drifted paths.

### Coverage impact

Exercising `run_status()` across ok/missing/drifted/unchecked/json/where paths covers the full body of the function. Expected: `app/src/commands/status.rs` goes from 0% to ~85%+ (remaining gap: coloured output detail lines that are hard to assert without stripping ANSI when `--no-color` is not used — all tests use `--no-color` so the colour branches are dead; those 3–4 lines are acceptable tarpaulin exclusions).
