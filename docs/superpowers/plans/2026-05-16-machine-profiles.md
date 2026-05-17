# Machine Profiles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Document the machine profiles convention using the existing `variables.*` system — zero code changes, pure documentation and examples.

**Architecture:** Add a "Machine Profiles" section to `CLAUDE.md` after the Homebrew section. Create `examples/machine-profiles/` with two example `etch.yaml` files and a manifests example. Remove the item from the backlog.

**Tech Stack:** Markdown, YAML

---

## Files

| File                                               | Change                                                             |
| -------------------------------------------------- | ------------------------------------------------------------------ |
| `CLAUDE.md`                                        | Add "Machine Profiles" section after "Homebrew macOS Workflow"     |
| `examples/machine-profiles/mac-workstation.yaml`   | **Create** — example etch.yaml for a Mac workstation               |
| `examples/machine-profiles/linux-workstation.yaml` | **Create** — example etch.yaml for a Linux workstation             |
| `examples/machine-profiles/kubernetes.yaml`        | **Create** — example manifest using `where: 'variables.has_k8s'`   |
| `docs/superpowers/README.md`                       | Move "Machine profiles" from backlog to plans table (status: Done) |

---

### Task 1: Add "Machine Profiles" section to `CLAUDE.md`

**Files:**

- Modify: `CLAUDE.md` (after line 128, before `## Config File`)

- [ ] **Step 1: Add the section**

In `CLAUDE.md`, find `## Config File` (currently at line 129) and insert the following section immediately before it:

````markdown
## Machine Profiles

etch-cli does not have built-in profile concepts — use the `variables:` section of `etch.yaml` to define a machine's profile and capabilities. Manifests use `where:` conditions to apply actions selectively.

**Convention:** define `profile` (a human-readable name) and one `has_<capability>: true` boolean per capability your machine supports.

```yaml
# Mac Studio — ~/.config/etch/etch.yaml
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
````

```yaml
# Linux workstation — ~/.config/etch/etch.yaml
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

**Manifest usage:**

```yaml
# Entire manifest skips on machines without k8s capability
where: "variables.has_k8s"

actions:
    - action: package.install
      list: [kubectl, helm, k9s]
      provider: homebrew
```

```yaml
# Per-action capability guard
actions:
    - action: package.install
      name: gh
      provider: homebrew
      where: "variables.has_devtools"
```

**Capability naming convention:**

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

See `examples/machine-profiles/` for complete example files.

````

- [ ] **Step 2: Verify it looks right**

```bash
grep -A 5 "## Machine Profiles" /Users/bruce/git-repos/personal/etch-cli/CLAUDE.md | head -10
````

Expected: the section heading and first paragraph appear.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add Machine Profiles section to CLAUDE.md

Convention: variables.profile + variables.has_* boolean flags.
Zero code changes — uses existing variables.* context system.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Create example files in `examples/machine-profiles/`

**Files:**

- Create: `examples/machine-profiles/mac-workstation.yaml`
- Create: `examples/machine-profiles/linux-workstation.yaml`
- Create: `examples/machine-profiles/kubernetes.yaml`

- [ ] **Step 1: Create `examples/machine-profiles/mac-workstation.yaml`**

```yaml
# Example etch.yaml for a Mac workstation
# Copy to ~/.config/etch/etch.yaml and adjust to match your machine's capabilities.

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

- [ ] **Step 2: Create `examples/machine-profiles/linux-workstation.yaml`**

```yaml
# Example etch.yaml for a Linux workstation
# Copy to ~/.config/etch/etch.yaml and adjust to match your machine's capabilities.

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

- [ ] **Step 3: Create `examples/machine-profiles/kubernetes.yaml`**

```yaml
# Example manifest that only applies on machines with the k8s capability.
# Place in your manifests directory (e.g. manifests/kubernetes/main.yaml).
# The top-level where: condition skips the entire manifest on non-k8s machines.

where: "variables.has_k8s"

actions:
    - action: package.install
      list:
          - kubectl
          - helm
          - k9s
      provider: homebrew

    - action: package.install
      name: argocd
      provider: homebrew
      where: "variables.has_devtools"
```

- [ ] **Step 4: Commit**

```bash
git add examples/machine-profiles/
git commit -m "docs: add machine-profiles examples

Two example etch.yaml files (mac-workstation, linux-workstation)
and a kubernetes manifest using where: 'variables.has_k8s'.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Update docs/superpowers/README.md — move from backlog to plans table

**Note: Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plans table**

Change the `machine-profiles` row from `Pending` to `Done`:

```
| 2026-05-16 | [machine-profiles](plans/2026-05-16-machine-profiles.md) | [machine-profiles](specs/2026-05-16-machine-profiles-design.md) | Done |
```

Also add the plan file reference (update the `—` to link to this plan file).

- [ ] **Step 2: Remove from backlog**

Remove the "Machine profiles / capability groups" row from the Backlog table.

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-16-machine-profiles.md
git commit -m "docs: mark machine-profiles Done; remove from backlog"
git push origin main
```
