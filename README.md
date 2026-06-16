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
| `git.clone` / `git.pull` / `git.config`                                                                                     | Git repository and config management. `git.clone` skips if directory exists; set `update_existing: true` to pull instead of skip when directory already exists.                                                                                                                                                                                                                                             |
| `package.install` / `package.repository` / `package.autoremove`                                                             | Install packages and remove unused dependencies (Homebrew, apt, snap). Optional `version:` field pins a package to an exact version — skips if already correct, errors if wrong version installed, installs at declared version if absent. `version:` requires `name:` (not `list:`); incompatible with `cask: true`. Homebrew: `<name>@<version>`; apt: `<pkg>=<version>`; snap: channel name.             |
| `package.upgrade`                                                                                                           | Upgrade installed packages via apt or snap. Runs `apt list --upgradable` / `snap refresh --list` at plan time and generates steps only when upgrades exist — no-ops if nothing to upgrade. `provider` required (`apt`/`apt-get`/`aptitude` or `snap`/`snapcraft`); optional `name` (single package) or `list` (multiple, mutually exclusive). `provider: homebrew` fails with a redirect to `brew.upgrade`. |
| `package.remove`                                                                                                            | Remove installed packages (apt/snap/homebrew). `name` (single) or `list` (multiple — mutually exclusive); `provider` (`apt`, `snap`, `homebrew`); `purge` (bool, apt only — also removes config files via `apt-get purge`); `cask` (bool, homebrew only — required when removing a cask-installed app). Idempotent — skips packages not installed.                                                          |
| `brew.bundle` / `brew.upgrade` / `brew.cleanup`                                                                             | Homebrew bundle, upgrades, and cache cleanup                                                                                                                                                                                                                                                                                                                                                                |
| `npm.install`                                                                                                               | Install npm packages globally (idempotent)                                                                                                                                                                                                                                                                                                                                                                  |
| `claude.install`                                                                                                            | Install Claude Code plugins (idempotent). Accepts `name:` (single plugin) or `list:` (multiple). Queries `claude plugins list` at plan time and skips any plugin already installed, matching by base name so `superpowers@claude-plugins-official` and `superpowers` are treated as the same plugin.                                                                                                        |
| `claude.upgrade`                                                                                                            | Upgrade all currently installed Claude Code plugins. Discovers installed plugins at plan time via `claude plugins list`; generates one streaming upgrade step per plugin. No-op if no plugins are installed.                                                                                                                                                                                                |
| `claude.marketplace`                                                                                                        | Add a Claude Code plugin marketplace. `name` (marketplace handle), `source` (GitHub `owner/repo` or full git URL), `scope` (optional: `user`/`project`/`local`, default `user`), `sparse` (optional list of paths for monorepo sparse checkout). Idempotent — skips if marketplace already registered.                                                                                                      |
| `claude.marketplace.remove`                                                                                                 | Remove a registered Claude Code plugin marketplace. `name` (marketplace handle), `scope` (optional — removes from all scopes if omitted). Idempotent — skips if marketplace not present.                                                                                                                                                                                                                    |
| `claude.plugin.update`                                                                                                      | Update already-installed Claude Code plugins. Accepts `name:` (single plugin) or `list:` (multiple). Always runs `claude plugins update` — no idempotency pre-check. Use alongside `claude.install` (install-if-missing) to keep plugins current.                                                                                                                                                           |
| `mas.install` / `mas.upgrade`                                                                                               | Mac App Store apps (macOS). `mas.install` accepts a single app (`name:` + `id:`) or a list of apps (`list:` of `{name, id}` entries) — mutually exclusive. `mas.upgrade` upgrades all installed App Store apps.                                                                                                                                                                                             |
| `macos.default`                                                                                                             | Write macOS defaults                                                                                                                                                                                                                                                                                                                                                                                        |
| `macos.rosetta`                                                                                                             | Ensure Rosetta 2 is installed on Apple Silicon (macOS only). No fields. Idempotent — skips if already installed. Use `where: 'os.name == "macos"'` to gate on macOS.                                                                                                                                                                                                                                        |
| `macos.service`                                                                                                             | Load/unload LaunchDaemons and LaunchAgents                                                                                                                                                                                                                                                                                                                                                                  |
| `macos.softwareupdate`                                                                                                      | Install all available macOS software updates via `softwareupdate --install --all`. Privileged, self-idempotent. macOS only.                                                                                                                                                                                                                                                                                 |
| `systemd.service`                                                                                                           | Enable/disable/start/stop systemd units                                                                                                                                                                                                                                                                                                                                                                     |
| `terraform.tfenv`                                                                                                           | Install tfenv (Terraform version manager) via git clone to `~/.tfenv`, optionally install and activate a specific Terraform version. Idempotent. Add `~/.tfenv/bin` to PATH separately.                                                                                                                                                                                                                     |
| `binary.github` / `binary.url`                                                                                              | Install binaries from releases or URLs. When `version:` is a pinned tag, `etch status` detects install mismatches (via sidecar `{dir}/.{name}.version`) and available updates (via cached GitHub API, 1h TTL at `~/.cache/etch/github-versions/`).                                                                                                                                                          |
| `group.add` / `user.add` / `user.group` / `user.default_shell`                                                              | Manage Unix groups and users. `user.default_shell` sets the login shell via `chsh`; idempotent (reads current shell at plan time). `username:` targets another user (requires privilege escalation).                                                                                                                                                                                                        |
| `plugin`                                                                                                                    | Load and run community or local etch plugins                                                                                                                                                                                                                                                                                                                                                                |
| `ruby.install`                                                                                                              | Install Ruby versions via ruby-install; optional `version_manager` field (`"rbenv"` \| `"chruby"`) runs post-install steps                                                                                                                                                                                                                                                                                  |
| `ruby.chruby`                                                                                                               | Install chruby via Homebrew; optionally set default ruby in ~/.ruby-version                                                                                                                                                                                                                                                                                                                                 |
| `gem.install`                                                                                                               | Install Ruby gems (idempotent)                                                                                                                                                                                                                                                                                                                                                                              |
| `pip.install`                                                                                                               | Install Python packages (idempotent)                                                                                                                                                                                                                                                                                                                                                                        |
| `pyenv.install`                                                                                                             | Install Python versions via pyenv; optional `configure_opts` field sets `PYTHON_CONFIGURE_OPTS` before install                                                                                                                                                                                                                                                                                              |
| `pyenv.virtualenv`                                                                                                          | Create a pyenv virtualenv (idempotent). `recreate: true` deletes and recreates the venv when the installed Python version differs from `python_version:` — use for Python patch version bumps.                                                                                                                                                                                                              |
| `zsh.oh-my-zsh`                                                                                                             | Install oh-my-zsh and optionally clone community plugins into `~/.oh-my-zsh/custom/plugins/`                                                                                                                                                                                                                                                                                                                |

## etch update

`etch update` runs an ordered sequence of tool update steps. With no flags it runs all applicable steps; use `--only` or `--skip` to filter.

```shell
etch update                    # run all steps
etch update --only brew,rust   # Homebrew and Rust only
etch update --skip pip,gems    # everything except pip and gems
etch update --only foobar      # error: unknown category 'foobar'
```

### Flags

| Flag                  | Description                                    |
| --------------------- | ---------------------------------------------- |
| `--only <categories>` | Run only the listed comma-separated categories |
| `--skip <categories>` | Run all categories except the listed ones      |

`--only` and `--skip` are mutually exclusive. An unknown category name is a hard error that lists all valid names.

### Categories

| Category    | What it updates                                          | Platform    |
| ----------- | -------------------------------------------------------- | ----------- |
| `brew`      | `brew upgrade` + `brew cleanup`                          | macOS/Linux |
| `system`    | `softwareupdate -ia`                                     | macOS only  |
| `mas`       | Mac App Store apps via `mas upgrade`                     | macOS only  |
| `claude`    | Claude plugins + npm globals (from config)               | any         |
| `packages`  | `apt-get upgrade` + `snap refresh`                       | Linux only  |
| `pip`       | `pip install --upgrade` outdated packages                | any         |
| `rust`      | `rustup update` + `cargo-nextest`                        | any         |
| `git-tools` | `git pull` on ai-config, dotfiles, oh-my-zsh, tpm, tfenv | any         |
| `gems`      | `gem update`                                             | any         |
| `cheatsh`   | Re-downloads `~/bin/cht.sh` via curl                     | any         |

Steps that require a tool not present on the machine are automatically skipped. Platform-specific steps (softwareupdate, mas, apt, snap) are silently skipped on the wrong OS.

> **`etch update --only packages` vs `package.upgrade` in manifests:** `etch update --only packages` is an imperative one-shot upgrade of all apt/snap packages on the machine. `package.upgrade` in a manifest is the declarative equivalent — it checks for upgradeable packages at plan time and generates steps only when needed, so it is safe to include in an `etch apply` run and will no-op if everything is already up to date.

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

## etch doctor

`etch doctor` validates system health — symlink integrity, tools in PATH, credential directory permissions, and binary version drift. It complements `etch status` (manifest drift) by covering system-level invariants that manifests don't check.

```shell
etch doctor              # run all checks, exit 1 if any fail
etch doctor --json       # machine-readable JSON output
etch doctor --missing-only  # suppress passing checks, show failures only
```

### Checks

| Check           | What it validates                                | Source                                                         |
| --------------- | ------------------------------------------------ | -------------------------------------------------------------- |
| Symlinks        | `file.link` targets exist and resolve            | Manifest-derived                                               |
| Tools           | Tools exist in PATH                              | Manifest-derived + `doctor.tools:` config                      |
| Credential dirs | Directories have mode 700                        | `doctor.credential_dirs:` config                               |
| Versions        | Binary `--version` output contains pinned string | `binary.github`/`binary.url` atoms + `doctor.versions:` config |

### Configuration

Add a `doctor:` section to `~/.config/etch/etch.yaml`:

```yaml
doctor:
    tools: # explicit tools beyond manifest-derived
        - kubectl
        - helm
    versions: # explicit version pins (substring match against command output)
        - tool: ripgrep
          command: "rg --version"
          expected: "14.1.0"
    credential_dirs: # directories to verify have mode 700
        - ~/.ssh
        - ~/.tf_creds
        - ~/.tsh
```

Manifest-derived tool checks: `brew.bundle`/`brew.upgrade`/`brew.cleanup` → `brew`, `gem.install` → `gem`, `pip.install` → `pip`, `npm.install` → `npm`, `mas.install`/`mas.upgrade` → `mas`, `pyenv.install`/`pyenv.virtualenv` → `pyenv`, `ruby.install` → `ruby-install`, `claude.install`/`claude.upgrade`/`claude.plugin.update` → `claude`.

## etch history

`etch history` shows what `etch apply` has done — a persistent record of every atom that executed successfully, written to `~/.local/share/etch/state.yaml` after each apply.

```shell
etch history                         # table of all recorded atoms
etch history --manifest <substr>     # filter by manifest name substring
etch history --json                  # NDJSON, one object per atom
```

**Output columns:** `MANIFEST` · `ACTION` · `KEY` (destination path, package name, etc.) · `APPLIED AT` · `CHANGED` (yes/no — whether the atom mutated state in that run).

The state file uses merge semantics: re-running the same action updates the existing row rather than appending — the file always reflects the most-recent outcome per `(manifest, action, key)` triple.

## etch rollback

`etch rollback` lists and restores pre-apply file backups. Before `file.copy` overwrites an existing file, etch stashes the original to `~/.local/share/etch/backups/`. The three most recent stashes per path are kept (configurable via `ETCH_STASH_DIR` env var in tests).

```shell
etch rollback                              # list all stashed paths with timestamps
etch rollback --path ~/.zshrc             # restore latest stash for that path
etch rollback --path ~/.zshrc --dry-run   # diff stash vs current; no write
etch rollback --all --yes                 # restore all paths, skip confirmation
```

Restore preserves the original file permissions. If `~/.ssh/id_rsa` was stashed at mode 0600, it is restored at 0600 regardless of the current umask. Old stash entries (created before v0.13.0) that have no recorded mode are silently restored without a permission change.

Stash is best-effort: stash failures log a warning and never block `etch apply`.

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

Package install, upgrade, and autoremove operations stream output directly to the terminal in real time — apt/brew/snap progress appears as it runs. If a package operation fails, the error message is visible inline.

To reproduce a failure outside etch:

```bash
# package.install
sudo env DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true NEEDRESTART_MODE=a \
  apt install --yes <package-list>

# package.upgrade (upgrade-all)
sudo env DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true NEEDRESTART_MODE=a \
  apt-get update && \
sudo env DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true NEEDRESTART_MODE=a \
  apt-get upgrade -y

# package.upgrade (named package)
sudo env DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true NEEDRESTART_MODE=a \
  apt-get update && \
sudo env DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true NEEDRESTART_MODE=a \
  apt-get install --only-upgrade -y <package>
```

> **Note:** etch propagates all three environment variables to suppress interactive prompts from dpkg post-invoke hooks. `DEBIAN_FRONTEND=noninteractive` disables debconf UI; `DEBCONF_NONINTERACTIVE_SEEN=true` marks questions as seen so debconf applies defaults silently; `NEEDRESTART_MODE=a` auto-restarts services instead of prompting. When reproducing failures manually, include all three.

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
