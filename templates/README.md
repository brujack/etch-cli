# Templates

Complete machine setup templates — copy one, customize it, and use it as your
manifest directory.

Templates differ from `examples/`: the `examples/` directory demonstrates individual
actions in isolation. Templates show how to wire multiple manifests together into a
working machine configuration.

## Usage

1. Copy a template directory to your own (private) manifest repo:
   ```
   cp -r templates/personal-workstation ~/my-manifests/
   ```
2. Edit the copied manifests to match your paths and preferences.
3. Point etch at the directory in `~/.config/etch/etch.yaml`:
   ```yaml
   manifest_paths:
     - ~/my-manifests
   variables:
     dotfiles_dir: "~/git-repos/personal/dotfiles"
   ```
4. Run `etch apply`.

## Available Templates

| Template | Description |
| -------- | ----------- |
| [personal-workstation](personal-workstation/) | Shell environment, editor config, git, SSH — the baseline for a personal macOS or Linux workstation |
