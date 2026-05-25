# binary.url Design Spec

> **Status: DONE** — Implemented in PR #44 (2026-05-25)

## Goal

Add a `binary.url` action alongside `binary.github` that downloads binaries from arbitrary URLs (go.dev, releases.hashicorp.com, etc.) with optional sha256 checksum verification and archive extraction.

## Motivation

`binary.github` is GitHub-only. Many tools distribute binaries via arbitrary HTTPS URLs (Go toolchain, Terraform, Vault, Nomad, Docker Compose, yq, etc.). Users need a way to install these tools declaratively without shell scripts.

## Design

### Action: `binary.url`

Registered as `binary.url` and `bin.url`.

**Fields:**

| Field        | Type             | Required | Description                                                   |
| ------------ | ---------------- | -------- | ------------------------------------------------------------- |
| `name`       | String           | yes      | Installed binary filename                                     |
| `url`        | String           | yes      | Download URL; Tera-rendered at plan time                      |
| `directory`  | String           | yes      | Install directory (e.g. `/usr/local/bin`)                     |
| `version`    | Option\<String\> | no       | Injected as `{{ version }}` in URL template                   |
| `file`       | Option\<String\> | no       | Path inside archive to extract (required for non-Raw formats) |
| `sha256`     | Option\<String\> | no       | Expected sha256 hex digest                                    |
| `privileged` | Option\<bool\>   | no       | Reserved for future use (matches binary.github behavior)      |

**Idempotency:** if `{directory}/{name}` already exists, `plan()` returns an empty step list (skip).

**Format detection** from URL suffix (strips `?` and `#` fragments):

- `.tar.gz` / `.tgz` → `TarGz`
- `.tar.xz` → `TarXz`
- `.zip` → `Zip`
- anything else → `Raw`

**Step sequence:**

1. `Download` → `{directory}/{name}.etch-tmp`
2. `BinaryVerify` (only if `sha256` is set) — sha256 check on temp file
3. `BinaryExtract` — Raw: `fs::rename`; archives: extract named `file` to dest
4. `Chmod(dest, 0o755)`

### New Atoms

**`BinaryVerify`** (`lib/src/atoms/binary/verify.rs`):

- Uses `sha256::try_digest()` (existing dep)
- Error includes both expected and actual hashes

**`BinaryExtract`** (`lib/src/atoms/binary/extract.rs`):

- `ArchiveFormat` enum: `Raw`, `TarGz`, `TarXz`, `Zip`
- Raw: `fs::rename` (same-filesystem, atomic)
- TarGz/TarXz: iterates entries, extracts the named `file`
- Zip: `ZipArchive::by_name()`, streams to dest

### New Dependencies

```toml
zip = "2"
xz2 = "0.1"
```

(Existing: `sha256 = "1.6"`, `tar = "0.4"`, `flate2 = "1.x"`)

### Example Manifest

```yaml
- action: binary.url
  name: go
  url: "https://go.dev/dl/go{{ version }}.darwin-arm64.tar.gz"
  directory: /usr/local/go/bin
  version: "1.22.3"
  file: go/bin/go
  sha256: "abc123..."

- action: binary.url
  name: terraform
  url: "https://releases.hashicorp.com/terraform/{{ version }}/terraform_{{ version }}_linux_amd64.zip"
  directory: /usr/local/bin
  version: "1.8.5"
  file: terraform
  sha256: "def456..."
```

## Decisions

- **`temp_path = {directory}/{name}.etch-tmp`**: predictable, same filesystem as dest, enables atomic rename for Raw format. On retry, `Download` overwrites unconditionally.
- **`file` required for archives**: validated at `plan()` time so the error is immediate, not deferred to execution.
- **`privileged` field accepted but ignored**: consistent with `binary.github` behavior; reserved for future elevation support.
- **No cleanup on verify failure**: `.etch-tmp` stays on disk after a sha256 mismatch. On retry, `plan()` regenerates all steps (dest still absent) and `Download` overwrites the temp file.
