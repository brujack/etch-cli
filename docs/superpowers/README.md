# Superpowers Specs and Plans

Master status index for all specs and implementation plans in this directory.

## Status Key

| Status      | Meaning                          |
| ----------- | -------------------------------- |
| Done        | Implemented and merged to master |
| In Progress | Currently being implemented      |
| Pending     | Not yet started                  |

---

## All Plans

| Date       | Plan                                                                                     | Spec                                                                                            | Status      |
| ---------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ----------- |
| 2026-05-02 | [etch-cli-phase1](plans/2026-05-02-etch-cli-phase1.md)                                   | —                                                                                               | In Progress |
| 2026-05-02 | [platform-pruning](plans/2026-05-02-platform-pruning.md)                                 | [platform-pruning](specs/2026-05-02-platform-pruning-design.md)                                 | Done        |
| 2026-05-02 | [test-coverage](plans/2026-05-02-test-coverage.md)                                       | [test-coverage](specs/2026-05-02-test-coverage-design.md)                                       | Pending     |
| 2026-05-04 | —                                                                                        | [dead-code-removal](specs/2026-05-04-dead-code-removal-design.md)                               | Done        |
| 2026-05-13 | [dry-run](plans/2026-05-13-dry-run.md)                                                   | [dry-run](specs/2026-05-13-dry-run-design.md)                                                   | Done        |
| 2026-05-14 | [dotfiles-symlinks](plans/2026-05-14-dotfiles-symlinks.md)                               | [dotfiles-symlinks](specs/2026-05-14-dotfiles-symlinks-design.md)                               | Done        |
| 2026-05-15 | [file-chmod](plans/2026-05-15-file-chmod.md)                                             | [file-chmod](specs/2026-05-15-file-chmod-design.md)                                             | Done        |
| 2026-05-15 | [file-action-privileged](plans/2026-05-15-file-action-privileged.md)                     | [file-action-privileged](specs/2026-05-15-file-action-privileged-design.md)                     | Done        |
| 2026-05-16 | [tilde-expansion](plans/2026-05-16-tilde-expansion.md)                                   | [tilde-expansion](specs/2026-05-16-tilde-expansion-design.md)                                   | Done        |
| 2026-05-16 | [brew-bundle](plans/2026-05-16-brew-bundle.md)                                           | [brew-bundle](specs/2026-05-16-brew-bundle-design.md)                                           | Done        |
| 2026-05-16 | [package-install-cask](plans/2026-05-16-package-install-cask.md)                         | [package-install-cask](specs/2026-05-16-package-install-cask-design.md)                         | Done        |
| 2026-05-16 | [mas-install](plans/2026-05-16-mas-install.md)                                           | [mas-install](specs/2026-05-16-mas-install-design.md)                                           | Done        |
| 2026-05-16 | [brew-upgrade-cleanup-mas-upgrade](plans/2026-05-16-brew-upgrade-cleanup-mas-upgrade.md) | [brew-upgrade-cleanup-mas-upgrade](specs/2026-05-16-brew-upgrade-cleanup-mas-upgrade-design.md) | Done        |
| 2026-05-16 | [machine-profiles](plans/2026-05-16-machine-profiles.md)                                 | [machine-profiles](specs/2026-05-16-machine-profiles-design.md)                                 | Pending     |

---

## Backlog

| Feature                              | Notes                                                                                                                                                                                                                            |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ntfy notification action             | Matches existing notification infra                                                                                                                                                                                              |
| macOS defaults write ergonomics      | If etch-cli's current API is rough                                                                                                                                                                                               |
| Binary install from arbitrary URL    | Many tools (Go, Docker Compose, YQ, Vault, Nomad, Packer, Vagrant, Consul, Terraform) download from non-GitHub URLs (go.dev, releases.hashicorp.com, etc.); `binary` action is GitHub-only — needs URL + sha256 checksum support |
| Machine profiles / capability groups | Dotfiles has hostname→profile→`[HAS_K8S]`/`[HAS_DEVTOOLS]`/etc. capability matrix; etch-cli `where:` is per-action rhai with no named-group abstraction — needs profile concept for applying manifest sets to machine classes    |
| systemd service management           | Linux daemon installs in dotfiles use `systemctl enable --now`; no `service.enable`/`service.start`/`service.disable` action exists                                                                                              |
| Git config management                | Dotfiles manages per-machine gitconfig variants (mac vs linux); etch-cli has `git.clone` but no `git.config` action for setting `user.name`, `user.email`, credential helpers, etc.                                              |
| command.run skip-if condition        | No way to skip a `command.run` action when a path/file already exists without embedding a shell guard inline (`[ -d path ] \|\| ...`); surfaced by oh-my-zsh install in Phase 2 symlinks migration                               |
| Wildcard / glob file.link            | `file.link` requires enumerating each source explicitly; no support for `link all files matching .claude/*` pattern; surfaced by ai-config Claude/Cursor symlinks in Phase 2                                                     |

---

## Adding a new entry

When a new spec or plan is created, add a row to the All Plans table. Set status to **In Progress** when implementation starts, **Done** when the PR merges. Also add a `> **Status: DONE**` banner at the top of the plan file once complete. Move backlog items to the All Plans table when their spec is written (remove the backlog row).
