# etch-cli

[![CI](https://img.shields.io/github/actions/workflow/status/brujack/etch-cli/ci.yml?event=pull_request&style=for-the-badge)](https://github.com/brujack/etch-cli/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/brujack/etch-cli?style=for-the-badge)](https://github.com/brujack/etch-cli/blob/main/LICENSE)
![coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/brujack/etch-cli/coverage-data/coverage.json)

Declarative configuration management for personal workstations. Define your packages, dotfiles, git repos, and macOS defaults in YAML manifests and apply them with a single command.

> **Note:** etch-cli is a personal fork of [comtrya](https://github.com/comtrya/comtrya) (archived April 2026, MIT license). The upstream project is maintained by [@rawkode](https://github.com/rawkode) and contributors; all credit for the original design and implementation goes to them.

---

## Installing

```shell
cargo install etch-cli
```

Or build from source:

```shell
git clone https://github.com/brujack/etch-cli.git
cd etch-cli
cargo build --release
# binary at target/release/etch
```

## Usage

```shell
# Apply all manifests in the current directory
etch apply

# Apply a subset of manifests
etch apply -m one,two,three

# Apply manifests from a specific directory
etch -d ./manifests apply

# Dry run — show what would change without applying
etch apply --dry-run
```

## Manifest format

Manifests are YAML files describing actions to perform:

```yaml
actions:
    - action: command.run
      command: echo
      args:
          - hello from etch

    - action: package.install
      name: htop

    - action: file.link
      from: ~/.dotfiles/.zshrc
      to: ~/.zshrc
```

See `CLAUDE.md` for the full action catalog with all fields documented.

## Action catalog

| Action                                   | Description                                |
| ---------------------------------------- | ------------------------------------------ |
| `command.run`                            | Run shell commands                         |
| `directory.create` / `directory.copy`    | Manage directories                         |
| `file.copy` / `file.link` / `file.chmod` | Manage files and permissions               |
| `git.clone` / `git.pull` / `git.config`  | Git repository and config management       |
| `package.install` / `package.repository` | Install packages (Homebrew, apt, …)        |
| `brew.bundle` / `brew.upgrade`           | Homebrew bundle and upgrades               |
| `mas.install` / `mas.upgrade`            | Mac App Store apps (macOS)                 |
| `macos.defaults`                         | Write macOS defaults                       |
| `macos.service`                          | Load/unload LaunchDaemons and LaunchAgents |
| `binary.github` / `binary.url`           | Install binaries from releases or URLs     |
| `group` / `user`                         | Manage Unix groups and users               |

## Development

```shell
make test     # lint + test
make lint     # cargo clippy -D warnings
make build    # cargo build --release
make semver   # check for API-breaking changes vs origin/main (advisory)
make install-hooks  # install pre-commit and pre-push hooks (run once per checkout)
```

`make test` also runs 5 `insta` snapshot tests (`app/tests/snapshots.rs`) that lock the exact stdout format of `etch -h`, `etch apply --help`, `etch version`, and `etch apply --dry-run`. Any accidental format change fails the test. To update snapshots intentionally: `INSTA_UPDATE=new cargo test --test snapshots`, then `cargo insta accept`, then commit the updated `.snap` files.

Prerequisites:

- `brew install git-cliff` — CHANGELOG generation (`make changelog`)

## Verifying releases

Release binaries are signed with [cosign](https://docs.sigstore.dev/cosign/overview/) using keyless Sigstore signing. Each release includes:

- `etch` — compiled binary
- `etch.sig` — detached signature
- `etch.pem` — signing certificate
- `etch.sbom.spdx.json` — SPDX bill of materials

To verify a release binary:

```bash
cosign verify-blob etch \
  --signature etch.sig \
  --certificate etch.pem \
  --certificate-identity \
    "https://github.com/brujack/etch-cli/.github/workflows/release-sign.yml@refs/tags/TAG" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
```

Replace `TAG` with the release tag (e.g. `v1.2.0`).
