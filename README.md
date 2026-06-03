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

# Show all actions, including those with nothing to do
etch apply --verbose

# Show flags for every subcommand in one shot
etch help-all
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

### Handlers

Handlers run once at the end of the manifest when a notifying action made a change:

```yaml
actions:
    - action: macos.default
      domain: com.apple.dock
      key: autohide
      kind: bool
      value: "true"
      notify: [restart-dock]

handlers:
    - name: restart-dock
      action: command.run
      command: killall
      args: [Dock]
```

### Templates

`file.copy` supports Tera (Jinja2-compatible) template rendering with `template: true`. The source file is rendered before being written to the destination.

```yaml
actions:
    - action: file.copy
      from: nginx.conf.j2 # file in the `files/` subdirectory
      to: /etc/nginx/nginx.conf
      template: true
      chmod: "0644"
      privileged: true
```

Available context namespaces inside templates:

| Namespace      | Examples                                                                                     |
| -------------- | -------------------------------------------------------------------------------------------- |
| `user.*`       | `{{ user.username }}`, `{{ user.home_dir }}`                                                 |
| `os.*`         | `{{ os.name }}` (`macos`/`linux`), `{{ os.arch }}` (`aarch64`/`x86_64`), `{{ os.hostname }}` |
| `variables.*`  | `{{ variables.my_var }}` — values from `etch.yaml` `variables:`                              |
| `env.*`        | `{{ env.HOME }}`                                                                             |
| `manifest_dir` | absolute path to the manifest directory                                                      |

**Common mistake:** using `{{ my_var }}` (bare) instead of `{{ variables.my_var }}`. Bare names render empty without error.

Tera supports `{% if %}`, `{% for %}`, filters (`| default(value="x")`), and the custom `read_file_contents(path=...)` function. See `examples/file/files/some-file.j2` for a full reference.

See `CLAUDE.md` for the full action catalog with all fields documented. Complete working examples are in [`examples/package/`](examples/package/).

## Action catalog

| Action                                                                                                                      | Description                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `command.run`                                                                                                               | Run shell commands                                                                                                                                                                                                                                                                                                                                                                                          |
| `directory.create` / `directory.copy` / `directory.remove`                                                                  | Manage directories                                                                                                                                                                                                                                                                                                                                                                                          |
| `file.copy` / `file.link` / `file.chmod` / `file.chown` / `file.flags` / `file.download` / `file.remove` / `file.unarchive` | Manage files, permissions, and BSD flags                                                                                                                                                                                                                                                                                                                                                                    |
| `git.clone` / `git.pull` / `git.config`                                                                                     | Git repository and config management                                                                                                                                                                                                                                                                                                                                                                        |
| `package.install` / `package.repository` / `package.autoremove`                                                             | Install packages and remove unused dependencies (Homebrew, apt, snap). Optional `version:` field pins a package to an exact version — skips if already correct, errors if wrong version installed, installs at declared version if absent. `version:` requires `name:` (not `list:`); incompatible with `cask: true`. Homebrew: `<name>@<version>`; apt: `<pkg>=<version>`; snap: channel name.             |
| `package.upgrade`                                                                                                           | Upgrade installed packages via apt or snap. Runs `apt list --upgradable` / `snap refresh --list` at plan time and generates steps only when upgrades exist — no-ops if nothing to upgrade. `provider` required (`apt`/`apt-get`/`aptitude` or `snap`/`snapcraft`); optional `name` (single package) or `list` (multiple, mutually exclusive). `provider: homebrew` fails with a redirect to `brew.upgrade`. |
| `brew.bundle` / `brew.upgrade` / `brew.cleanup`                                                                             | Homebrew bundle, upgrades, and cache cleanup                                                                                                                                                                                                                                                                                                                                                                |
| `npm.install`                                                                                                               | Install npm packages globally (idempotent)                                                                                                                                                                                                                                                                                                                                                                  |
| `mas.install` / `mas.upgrade`                                                                                               | Mac App Store apps (macOS). `mas.install` accepts a single app (`name:` + `id:`) or a list of apps (`list:` of `{name, id}` entries) — mutually exclusive. `mas.upgrade` upgrades all installed App Store apps.                                                                                                                                                                                             |
| `macos.defaults`                                                                                                            | Write macOS defaults                                                                                                                                                                                                                                                                                                                                                                                        |
| `macos.rosetta`                                                                                                             | Ensure Rosetta 2 is installed on Apple Silicon (macOS only). No fields. Idempotent — skips if already installed. Use `where: 'os.name == "macos"'` to gate on macOS.                                                                                                                                                                                                                                        |
| `macos.service`                                                                                                             | Load/unload LaunchDaemons and LaunchAgents                                                                                                                                                                                                                                                                                                                                                                  |
| `systemd.service`                                                                                                           | Enable/disable/start/stop systemd units                                                                                                                                                                                                                                                                                                                                                                     |
| `binary.github` / `binary.url`                                                                                              | Install binaries from releases or URLs                                                                                                                                                                                                                                                                                                                                                                      |
| `group.add` / `user.add` / `user.group`                                                                                     | Manage Unix groups and users                                                                                                                                                                                                                                                                                                                                                                                |
| `plugin`                                                                                                                    | Load and run community or local etch plugins                                                                                                                                                                                                                                                                                                                                                                |
| `ruby.install`                                                                                                              | Install Ruby versions via ruby-install; optional `version_manager` field (`"rbenv"` \| `"chruby"`) runs post-install steps                                                                                                                                                                                                                                                                                  |
| `gem.install`                                                                                                               | Install Ruby gems (idempotent)                                                                                                                                                                                                                                                                                                                                                                              |
| `pip.install`                                                                                                               | Install Python packages (idempotent)                                                                                                                                                                                                                                                                                                                                                                        |
| `pyenv.install`                                                                                                             | Install Python versions via pyenv; optional `configure_opts` field sets `PYTHON_CONFIGURE_OPTS` before install                                                                                                                                                                                                                                                                                              |
| `pyenv.virtualenv`                                                                                                          | Create a pyenv virtualenv (idempotent)                                                                                                                                                                                                                                                                                                                                                                      |

## etch update

`etch update` runs an ordered sequence of tool update steps. With no flags it runs all applicable steps; any flag limits the run to only that step.

```shell
etch update            # run all steps
etch update --brew     # Homebrew only
etch update --rust     # Rust toolchain only
```

### Flags

| Flag          | What it updates                                          | Platform    |
| ------------- | -------------------------------------------------------- | ----------- |
| `--brew`      | `brew upgrade` + `brew cleanup`                          | macOS/Linux |
| `--system`    | `softwareupdate -ia`                                     | macOS only  |
| `--mas`       | Mac App Store apps via `mas upgrade`                     | macOS only  |
| `--claude`    | Claude plugins + npm globals (from config)               | any         |
| `--packages`  | `apt-get upgrade` + `snap refresh`                       | Linux only  |
| `--pip`       | `pip install --upgrade` outdated packages                | any         |
| `--rust`      | `rustup update` + `cargo-nextest`                        | any         |
| `--git-tools` | `git pull` on ai-config, dotfiles, oh-my-zsh, tpm, tfenv | any         |
| `--gems`      | `gem update`                                             | any         |
| `--cheatsh`   | Re-downloads `~/bin/cht.sh` via curl                     | any         |

Steps that require a tool not present on the machine are automatically skipped. Platform-specific steps (softwareupdate, mas, apt, snap) are silently skipped on the wrong OS.

> **`etch update --packages` vs `package.upgrade` in manifests:** `etch update --packages` is an imperative one-shot upgrade of all apt/snap packages on the machine. `package.upgrade` in a manifest is the declarative equivalent — it checks for upgradeable packages at plan time and generates steps only when needed, so it is safe to include in an `etch apply` run and will no-op if everything is already up to date.

### Summary output

After all steps run, etch prints a fixed-order summary table and appends the same to a log file:

```
=== Update Summary — 2026-05-31 09:00:00 ===

[OK]   brew             3 formulae (git 2.45.0, ripgrep 14.1.0, fzf 0.53.0)
[SKIP] softwareupdate   not applicable
[SKIP] mas              not applicable
[OK]   claude           2 plugin(s) updated
[SKIP] npm              npm not installed
[SKIP] apt              not applicable
[SKIP] snap             not applicable
[OK]   pip              no changes
[OK]   rust             no changes
[OK]   ai-config        1 commit(s)
[OK]   dotfiles         no changes
[OK]   oh-my-zsh        no changes
[SKIP] tpm              directory not found
[SKIP] tfenv            directory not found
[SKIP] gems             gem not installed
[SKIP] cheat.sh         ~/bin/cht.sh not found

15 sections: 6 OK, 0 failed, 0 warnings, 9 skipped
Log appended: /Users/you/.etch-update.log
```

### Configuration

Add an `update:` section to `~/.config/etch/etch.yaml` to configure git repos and Claude plugins:

```yaml
update:
    log_path: ~/.etch-update.log # default; omit to use this path
    git_tools:
        ai_config: "enabled" # any non-null string enables; pulls sibling of dotfiles_dir
        dotfiles: "enabled" # any non-null string enables; pulls dotfiles_dir path
        oh_my_zsh: true # pulls ~/.oh-my-zsh
        tpm: true # pulls ~/.tmux/plugins/tpm
        tfenv: true # pulls ~/.tfenv
    claude:
        plugins:
            - superpowers
        npm_globals:
            - typescript
            - "@anthropic-ai/claude-code"

variables:
    dotfiles_dir: ~/git-repos/personal/dotfiles # required for ai_config + dotfiles git_tools
    has_snap: "true" # enable snap updates (Linux)
    has_rust: "true" # enable rustup updates
    has_devtools: "true" # enable pip updates
```

`git_tools.ai_config` and `git_tools.dotfiles` accept any non-null string (the value is unused; only presence is checked). Both require `variables.dotfiles_dir` — ai-config is assumed at `<dotfiles_dir>/../ai-config`.

## etch plugin

Manage etch plugins — community or local collections of custom actions.

```shell
etch plugin add username/repo          # install latest
etch plugin add username/repo:v1.2.0   # pin to a tag
etch plugin list                       # list installed plugins and sync status
etch plugin remove name                # remove a plugin by name
etch plugin update [name]              # update one or all plugins
```

Plugins are stored in the platform data directory (`~/Library/Application Support/etch/plugins` on macOS, `~/.local/share/etch/plugins` on Linux). Each plugin is a cloned git repository. Plugin names in `remove` and `update` are the bare repo name (the part after `/`).

## Debugging

### Verbose output

Two separate verbosity mechanisms exist:

```bash
# Apply-level: show all actions including those with nothing to do
etch apply --verbose        # or: etch apply -v
etch apply --dry-run        # dry-run implies --verbose automatically

# Global debug logging: show command exit codes, captured stdout/stderr
etch -v apply               # DEBUG level
etch -vv apply              # TRACE level
```

`RUST_LOG` is not used. The global flag (`-v`) must come **before** the subcommand — `etch -v apply`, not `etch apply -v` (that activates the apply-level verbose flag).

### Linux: logs go to journald

On Linux with systemd, etch sends all log levels (including DEBUG) to journald in addition to stdout. When an action fails with no visible error, the captured subprocess output is in journald:

```bash
# Stream live while applying
journalctl -f &
etch apply

# Read after failure
journalctl -n 100 | grep -A5 "etch\|exit code\|stdout\|stderr"
```

### Diagnosing package.install and package.upgrade failures

When `package.install` or `package.upgrade` fails silently, etch has captured apt's stdout/stderr but only emits them at DEBUG level. To see the actual apt error:

```bash
etch -v apply 2>&1 | tee /tmp/etch-debug.log
grep -A5 "exit code\|stderr\|stdout" /tmp/etch-debug.log
```

Or run the apt command directly to reproduce outside etch:

```bash
# package.install
sudo DEBIAN_FRONTEND=noninteractive apt install --yes <package-list>

# package.upgrade (upgrade-all)
sudo DEBIAN_FRONTEND=noninteractive apt-get update && sudo apt-get upgrade -y

# package.upgrade (named package)
sudo DEBIAN_FRONTEND=noninteractive apt-get update && sudo apt-get install --only-upgrade -y <package>
```

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
- `etch.sha256` — SHA256 checksum
- `etch.bundle` — cosign bundle (signature + certificate)
- `etch.sbom.spdx.json` — SPDX bill of materials

Verify the checksum:

```bash
sha256sum -c etch.sha256
```

Verify the cosign signature:

```bash
cosign verify-blob etch \
  --bundle etch.bundle \
  --certificate-identity \
    "https://github.com/brujack/etch-cli/.github/workflows/release-sign.yml@refs/tags/TAG" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
```

Replace `TAG` with the release tag (e.g. `v0.10.4`).
