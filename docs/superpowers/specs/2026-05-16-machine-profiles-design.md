# Machine Profiles — Design Spec

**Date:** 2026-05-16
**Status:** Approved

## Context

The backlog entry "Machine profiles / capability groups" identified that etch-cli lacks a way to apply manifests selectively based on machine role. The dotfiles repo models this as `hostname → profile → [HAS_K8S, HAS_DEVTOOLS, ...]`. etch-cli already has `variables.*` context and rhai `where:` conditions — no code changes are needed.

This spec defines the convention for expressing machine profiles using the existing system.

## Approach: Pure Convention (Zero Code Changes)

Each machine has its own `~/.config/etch/etch.yaml`. The user manually sets a `profile` name and one boolean capability flag per capability in the `variables:` section. Manifests reference these via `where:` conditions.

This approach:

- Works today with zero code changes
- Gives each machine full control over its own capabilities
- Follows the dotfiles `[HAS_K8S]` naming pattern
- Uses the existing `variables.*` rhai namespace

## Convention

### etch.yaml per machine

```yaml
# Mac Studio (~/.config/etch/etch.yaml)
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests

variables:
    profile: "mac_workstation"
    has_gui: true
    has_devtools: true
    has_k8s: true
    has_docker: true
    has_rust: true
    has_printing: true
```

```yaml
# Linux workstation (~/.config/etch/etch.yaml)
manifest_paths:
    - ~/git-repos/personal/dotfiles/manifests

variables:
    profile: "linux_workstation"
    has_gui: true
    has_devtools: true
    has_k8s: true
    has_docker: true
    has_rust: true
    has_snap: true
```

### Capability naming convention

All capability flags follow the `has_<capability>` pattern with boolean values. Defined capability names:

| Variable       | Meaning                                         |
| -------------- | ----------------------------------------------- |
| `has_gui`      | Machine runs a graphical desktop                |
| `has_devtools` | Install developer tools (gh, jq, etc.)          |
| `has_k8s`      | Install Kubernetes tooling (kubectl, helm, k9s) |
| `has_docker`   | Install Docker and container tools              |
| `has_rust`     | Install Rust toolchain                          |
| `has_printing` | Install printer drivers                         |
| `has_snap`     | Use snap package manager (Linux only)           |

New capabilities can be added freely — the convention is the only constraint.

### Manifest usage

```yaml
# manifests/kubernetes/main.yaml
where: "variables.has_k8s"

actions:
    - action: package.install
      list: [kubectl, helm, k9s]
      provider: homebrew
      where: 'os.name == "macos"'
```

```yaml
# manifests/devtools/main.yaml
actions:
    - action: package.install
      name: gh
      provider: homebrew
      where: "variables.has_devtools"

    - action: package.install
      name: jq
      provider: homebrew
      where: "variables.has_devtools"
```

```yaml
# Profile-level guard (entire manifest skips on machines without the capability)
where: 'variables.profile == "linux_workstation"'
```

## What Changes

**1. `CLAUDE.md`** — new "Machine Profiles" section documenting the convention with both example `etch.yaml` files and a manifest snippet.

**2. `examples/machine-profiles/`** — new directory with:

- `mac-workstation.yaml` — example etch.yaml for a Mac workstation
- `linux-workstation.yaml` — example etch.yaml for a Linux workstation
- `kubernetes.yaml` — example manifest using `where: 'variables.has_k8s'`

**3. `docs/superpowers/README.md`** — remove "Machine profiles" from the backlog (solved by convention, no implementation required).

## What Is NOT in Scope

- Automatic hostname→profile resolution (manual is preferred)
- A `profiles:` block in etch.yaml (zero code changes)
- Validation that capability flags are spelled correctly
- An etch command to show the current machine's profile
