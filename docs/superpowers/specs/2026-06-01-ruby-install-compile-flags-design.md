# Spec: ruby.install compile_flags field

## Problem

`ruby.install` calls `ruby-install` to compile Ruby from source but provides no way to pass flags to the underlying `./configure` step. On macOS with Homebrew, compiling Ruby often requires flags such as `--with-openssl-dir=/opt/homebrew/opt/openssl@3` to link against the Homebrew-managed OpenSSL. Without this field, users must fall back to `command.run` workarounds.

## Design

### Struct changes (`lib/src/actions/ruby/install.rs`)

Add a `compile_flags` field to `RubyInstall`:

```rust
pub struct RubyInstall {
    pub version: String,
    pub implementation: Option<String>,
    pub rubies_dir: Option<String>,
    pub version_manager: Option<VersionManager>,
    #[serde(default)]
    pub compile_flags: Vec<String>,  // new
}
```

`#[serde(default)]` makes the field optional in YAML (defaults to empty vec when absent).

### `plan()` changes

When `compile_flags` is non-empty, append a `--` separator followed by each flag to the `ruby-install` arguments:

```
ruby dir exists? → return []
otherwise:
  args = [impl, version]
  if rubies_dir set: args += ["--rubies-dir", dir]
  if compile_flags non-empty: args += ["--", flag1, flag2, ...]
  step 1: ruby-install <args...>
  if version_manager == Rbenv:
    step 2: rbenv global <version>
    step 3: rbenv rehash
```

Flags are forwarded verbatim to `ruby-install`, which passes them after `--` to `./configure`.

### Manifest usage

```yaml
# macOS: link against Homebrew OpenSSL
- action: ruby.install
  version: "3.3.0"
  compile_flags:
      - "--with-openssl-dir=/opt/homebrew/opt/openssl@3"

# Multiple flags
- action: ruby.install
  version: "3.3.0"
  compile_flags:
      - "--with-openssl-dir=/opt/homebrew/opt/openssl@3"
      - "--with-readline-dir=/opt/homebrew/opt/readline"
```

### Error handling

Flags are forwarded verbatim. Invalid flags cause `ruby-install` to fail with its own error — no validation in etch.

### Testing

| Test                                                  | Assertion                                                   |
| ----------------------------------------------------- | ----------------------------------------------------------- |
| `it_can_be_deserialized_with_compile_flags`           | YAML `compile_flags` deserializes to a `Vec<String>`        |
| `compile_flags_defaults_to_empty`                     | Absent field deserializes to `vec![]`                       |
| `plan_includes_compile_flags_after_separator`         | Non-empty flags → `--` then flags appear in step args       |
| `plan_includes_compile_flags_with_default_rubies_dir` | `rubies_dir` absent; flags still appended correctly         |
| `plan_includes_multiple_compile_flags`                | Two flags both forwarded                                    |
| `plan_omits_separator_when_compile_flags_empty`       | Empty flags → no `--` in args                               |
| `plan_with_rbenv_and_compile_flags_emits_three_steps` | `version_manager: rbenv` + flags → 3 steps, flags in step 1 |

### Documentation updates

- `CLAUDE.md` Action Catalog: add `compile_flags` to `ruby.install` row
- `README.md` Action Catalog: same
- `examples/ruby/`: add example showing `compile_flags`
