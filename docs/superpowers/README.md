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

| Date       | Plan                                                     | Spec                                                              | Status      |
| ---------- | -------------------------------------------------------- | ----------------------------------------------------------------- | ----------- |
| 2026-05-02 | [etch-cli-phase1](plans/2026-05-02-etch-cli-phase1.md)   | —                                                                 | In Progress |
| 2026-05-02 | [platform-pruning](plans/2026-05-02-platform-pruning.md) | [platform-pruning](specs/2026-05-02-platform-pruning-design.md)   | Done        |
| 2026-05-02 | [test-coverage](plans/2026-05-02-test-coverage.md)       | [test-coverage](specs/2026-05-02-test-coverage-design.md)         | Pending     |
| 2026-05-04 | —                                                        | [dead-code-removal](specs/2026-05-04-dead-code-removal-design.md) | Done        |
| 2026-05-13 | [dry-run](plans/2026-05-13-dry-run.md)                   | [dry-run](specs/2026-05-13-dry-run-design.md)                     | Pending     |

---

## Backlog

| Feature                              | Notes                                                                                                                                                                                                                                     |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ntfy notification action             | Matches existing notification infra                                                                                                                                                                                                       |
| macOS defaults write ergonomics      | If etch-cli's current API is rough                                                                                                                                                                                                        |
| Homebrew cask support                | Dotfiles Brewfile has 57 cask entries; `package.install` brew provider has no cask flag — needs `cask: true` on the action or a separate `brew.cask` action                                                                               |
| Homebrew tap management              | 7 custom taps in Brewfile (chef, cloudflare, datawire, go-task, redpanda, snyk, speedtest); formulae from taps (e.g. `datawire/blackbird/telepresence-arm64`) can't install without the tap being added first — needs a `brew.tap` action |
| Mac App Store (mas) support          | 15 apps installed via `mas install` in dotfiles; no etch action exists — needs `mas.install` action wrapping the `mas` CLI                                                                                                                |
| Homebrew Bundle (Brewfile)           | Dotfiles uses a single Brewfile for all formulae/casks/mas/taps; a `brew.bundle` action running `brew bundle --file=<path>` would allow bulk migration of the Brewfile                                                                    |
| Binary install from arbitrary URL    | Many tools (Go, Docker Compose, YQ, Vault, Nomad, Packer, Vagrant, Consul, Terraform) download from non-GitHub URLs (go.dev, releases.hashicorp.com, etc.); `binary` action is GitHub-only — needs URL + sha256 checksum support          |
| File permissions (chmod/chown)       | Every Linux binary install in dotfiles ends with `chmod 755 + chown root:root`; no declarative `file.chmod`/`file.chown` action exists                                                                                                    |
| Machine profiles / capability groups | Dotfiles has hostname→profile→`[HAS_K8S]`/`[HAS_DEVTOOLS]`/etc. capability matrix; etch-cli `where:` is per-action rhai with no named-group abstraction — needs profile concept for applying manifest sets to machine classes             |
| systemd service management           | Linux daemon installs in dotfiles use `systemctl enable --now`; no `service.enable`/`service.start`/`service.disable` action exists                                                                                                       |
| Git config management                | Dotfiles manages per-machine gitconfig variants (mac vs linux); etch-cli has `git.clone` but no `git.config` action for setting `user.name`, `user.email`, credential helpers, etc.                                                       |

---

## Adding a new entry

When a new spec or plan is created, add a row to the All Plans table. Set status to **In Progress** when implementation starts, **Done** when the PR merges. Also add a `> **Status: DONE**` banner at the top of the plan file once complete. Move backlog items to the All Plans table when their spec is written (remove the backlog row).
