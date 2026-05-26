# macos.defaults: array-add and delete operations

## Context

`macos.default` currently supports only scalar `defaults write` — one operation, four required fields (`domain`, `key`, `kind`, `value`). Two gaps block dotfiles migration:

1. `-array-add` — append a value to an array key (e.g. `menuExtras` for menu bar extras)
2. `defaults delete` — remove a key entirely

The dotfiles `scripts/.osx.sh` uses `-array-add` for `menuExtras` (Volume.menu, Bluetooth.menu). No `defaults delete` examples exist in dotfiles today, but the operation is needed for completeness and for future manifests that clean up stale keys.

## Decision

Extend the existing `MacOSDefault` struct with an `operation` enum field. No new action types, no struct splits — one action, one `operation` field with three variants.

## Data Model

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacOSDefaultOperation {
    #[default]
    Write,
    ArrayAdd,
    Delete,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    pub domain: String,
    pub key: String,
    #[serde(default)]
    pub operation: MacOSDefaultOperation,
    pub kind: Option<String>,   // required for Write and ArrayAdd; ignored for Delete
    pub value: Option<String>,  // required for Write and ArrayAdd; ignored for Delete
}
```

`kind` and `value` become `Option<String>`. Existing manifests omitting `operation` deserialize to `Write` via `#[default]` — fully backward-compatible. Existing manifests using string `kind`/`value` fields continue to deserialize correctly (serde maps `"bool"` → `Some("bool")`).

## Manifest syntax

```yaml
# Existing write (unchanged)
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"

# array-add (idempotent)
- action: macos.default
  operation: array-add
  domain: com.apple.systemuiserver
  key: menuExtras
  kind: string
  value: "/System/Library/CoreServices/Menu Extras/Volume.menu"

# delete (idempotent)
- action: macos.default
  operation: delete
  domain: com.apple.dock
  key: stale-key
```

## plan() behavior

### Write (default)

Validates `kind` and `value` are `Some`, else `anyhow::bail!`. Emits:

```
Exec { command: "defaults", arguments: ["write", domain, key, "-{kind}", value] }
```

Same as current behavior.

### Delete

`kind` and `value` are ignored. Emits a shell wrapper for idempotency (`defaults delete` exits 1 when the key is absent):

```
Exec {
    command: "sh",
    arguments: ["-c", "defaults delete '<domain>' '<key>' 2>/dev/null || true"],
}
```

Single quotes around domain and key values; any embedded single quotes in domain/key are escaped as `'\''` before interpolation.

### ArrayAdd

Validates `kind` and `value` are `Some`, else `anyhow::bail!`. Emits a read-check-write shell one-liner for idempotency:

```
Exec {
    command: "sh",
    arguments: [
        "-c",
        "defaults read '<domain>' '<key>' 2>/dev/null | grep -qF '<value>' \
         || defaults write '<domain>' '<key>' -array-add -<kind> '<value>'"
    ],
}
```

`grep -qF` matches the literal value string (no regex interpretation). If `defaults read` fails (key absent) or the value is not found, `array-add` runs. Single-quote escaping applied to domain, key, and value.

## Idempotency

| Operation   | Mechanism                                                  |
| ----------- | ---------------------------------------------------------- | --- | ------------------------------------------------- |
| `write`     | `defaults write` naturally overwrites — already idempotent |
| `delete`    | `                                                          |     | true` shell suffix absorbs exit 1 when key absent |
| `array-add` | Read current array, grep for exact value, skip if present  |

## Tests

All unit tests in `lib/src/actions/macos/default.rs`. No real `defaults` binary invoked.

| Test                                    | What it verifies                                          |
| --------------------------------------- | --------------------------------------------------------- | --- | -------------- |
| `it_can_be_deserialized` (existing)     | Write deserialization unchanged                           |
| `plan_returns_one_step` (existing)      | Write step count unchanged                                |
| `plan_with_integer_kind` (existing)     | Write integer kind unchanged                              |
| `plan_with_string_kind` (existing)      | Write string kind unchanged                               |
| `operation_defaults_to_write`           | Manifest without `operation` field → `Write`              |
| `write_missing_kind_returns_error`      | `plan()` bails when `kind` absent for Write               |
| `write_missing_value_returns_error`     | `plan()` bails when `value` absent for Write              |
| `it_can_deserialize_array_add`          | YAML with `operation: array-add` round-trips              |
| `array_add_emits_correct_shell_command` | Correct `sh -c` one-liner in Exec arguments               |
| `array_add_missing_kind_returns_error`  | `plan()` bails when `kind` absent for ArrayAdd            |
| `array_add_missing_value_returns_error` | `plan()` bails when `value` absent for ArrayAdd           |
| `it_can_deserialize_delete`             | YAML with `operation: delete` round-trips                 |
| `delete_emits_correct_shell_command`    | `sh -c "defaults delete ...                               |     | true"` in Exec |
| `delete_ignores_kind_and_value`         | `kind`/`value` present in YAML but not in emitted command |

## Out of scope

- Array `write` (replace entire array) — not needed by dotfiles; can be added later as `operation: array-write`
- Dictionary operations — deferred (per original comment in source)
- `defaults export` / `defaults import` — not needed
