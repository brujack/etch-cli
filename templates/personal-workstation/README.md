# personal-workstation

A baseline manifest set for a personal macOS or Linux workstation. Sets up:

- Shell: oh-my-zsh, tmux plugin manager, zsh config symlinks
- Editor: vim config symlink
- Git: gitconfig symlink (platform-aware)
- SSH: directory with correct permissions, config symlink
- Credential directories: `.ssh`, `.tf_creds`, `.tsh` (700)

## Manifests

| Manifest | Depends on | What it does |
| -------- | ---------- | ------------ |
| `tools.yaml` | — | Bootstrap tools (oh-my-zsh, tpm) |
| `core.yaml` | tools | Dirs, shell config, editor, SSH symlinks |
| `gitconfig.yaml` | core | Git config symlink (macOS/Linux) |

## Required variable

Set `dotfiles_dir` in your `etch.yaml`:

```yaml
manifest_paths:
  - ~/my-manifests

variables:
  dotfiles_dir: "~/git-repos/personal/dotfiles"
```

All symlink sources are expressed as `{{ variables.dotfiles_dir }}/...` so the
template works regardless of where your dotfiles live.

## Customization

- Add symlinks in `core.yaml` for any additional dotfiles you manage.
- Add a `packages.yaml` for `brew.bundle` or `package.install` actions.
- Add a `repos.yaml` for `git.pull` on personal repos.
- Gate actions on machine capabilities using `where:` and `variables.*` — see
  `examples/machine-profiles/` for the pattern.
