# Smoke Tests

Run these on the Proxmox VM in order. Snapshot before step 3 (packages).

## VM setup

```shell
# Copy the entire smoke-tests directory to the VM
scp -r smoke-tests/ <user>@<vm-ip>:~/etch-smoke/

# Copy the Linux release binary
scp target/release/etch <user>@<vm-ip>:~/bin/etch
ssh <user>@<vm-ip> "chmod +x ~/bin/etch"

# Or build from source on the VM
git clone https://github.com/brujack/etch-cli.git ~/etch-cli
cd ~/etch-cli && cargo build --release && cp target/release/etch ~/bin/etch
```

## Run order

```shell
# 1. Smoke — confirms etch can parse and run a manifest
etch apply -d ~/etch-smoke -m 01-smoke

# 2. Files — directory create, file copy, symlink (no sudo)
etch apply -d ~/etch-smoke -m 02-files
ls ~/etch-test-output/

# 3. Packages — SNAPSHOT FIRST
etch apply -d ~/etch-smoke -m 03-packages
htop --version

# 4. Templates — Tera rendering with user/os context vars
etch apply -d ~/etch-smoke -m 04-templates
cat ~/etch-test-output/greeting.txt

# 5. Git clone — exercises network egress
etch apply -d ~/etch-smoke -m 05-git
ls ~/etch-test-output/comtrya-src/

# 6. Idempotency — must report no-op for every action
etch apply -d ~/etch-smoke -m 99-idempotency
```

Any action in step 6 that re-executes instead of no-op'ing is a bug in that action's check logic.

## Field names (verified from source)

| Action             | Fields                                                      |
| ------------------ | ----------------------------------------------------------- |
| `command.run`      | `command`, `args` (list), `privileged` (bool)               |
| `directory.create` | `path`                                                      |
| `directory.copy`   | `from`, `to`                                                |
| `file.copy`        | `from` (or `source`), `to` (or `target`), `template` (bool) |
| `file.link`        | `source`, `target` (`from`/`to` deprecated)                 |
| `git.clone`        | `repo_url`, `directory`                                     |
| `package.install`  | `name` (single) or `list` (multiple)                        |

Template engine is [Tera](https://keats.github.io/tera/). Available context variables: `user.username`, `user.home_dir`, `user.name`, `os.hostname`, `os.name`, `os.family`, `os.distribution`, `manifest_dir`.
