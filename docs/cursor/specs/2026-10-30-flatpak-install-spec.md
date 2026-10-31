# `flatpak.install` action

**Status:** design approved in conversation 2026-10-30; spec under review.

## Problem

etch-config cannot manage Steam. Its backlog defers `steam-installer` until etch-cli has a
`flatpak.install` action (etch-config `docs/superpowers/README.md`, "Deferred packages"
rows). Today dotfiles installs Steam imperatively in `lib/linux_ubuntu.sh`:

```bash
sudo flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
sudo flatpak install flathub com.valvesoftware.Steam -y
```

On `claude` (2026-10-30): flatpak 1.16.6, system remote `flathub`, and
`com.valvesoftware.Steam` installed at system scope from it. Both etch-config machine
profiles set `has_flatpak: true`.

## Action name

`flatpak.install` (new `flatpak` namespace; struct `FlatpakInstall`).

## YAML schema

```yaml
- action: flatpak.install
  name: com.valvesoftware.Steam # string; exactly one of `name` or `list`
  # list:                          # list of strings; exactly one of `name` or `list`
  #   - com.valvesoftware.Steam
  #   - org.videolan.VLC
  remote: flathub # string, default "flathub"
  remote_url: https://dl.flathub.org/repo/flathub.flatpakrepo # optional string
  scope: system # "system" (default) or "user"
  where: os.name == "linux"
```

`remote_url` defaults to `https://dl.flathub.org/repo/flathub.flatpakrepo` only when
`remote` is `flathub`. For any other remote it has no default.

## Behavior

`plan()` inspects the host and emits only the steps still needed:

1. **Inputs.** If both `name` and `list` are empty, or both are set, `plan()` returns an
   error naming `name` and `list`.
2. **flatpak present.** If `flatpak` is not on `PATH`, `plan()` returns an error saying
   flatpak is not installed. The action does not install flatpak.
3. **Remote.** `flatpak remotes --<scope> --columns=name` lists the scope's remotes. If
   `remote` is absent:
    - with a `remote_url` (given, or the flathub default), emit one step:
      `flatpak remote-add --if-not-exists --<scope> <remote> <remote_url>`;
    - without one, return an error naming the remote and asking for `remote_url`.
4. **Apps.** For each app id, `flatpak info --<scope> <id>` exits 0 when installed in that
   scope and 1 otherwise (measured on `claude`). Each missing app gets one step:
   `flatpak install --<scope> --noninteractive -y <remote> <id>`, in the order given.
   Duplicate ids in `list` produce one step.
5. **Idempotency.** When the remote exists and every app is installed, `plan()` returns no
   steps, so a second apply does nothing.
6. **Privilege.** Every system-scope step sets `privileged: true` on its `Exec`, so it runs
   through etch's configured privilege provider (sudo by default), as the package
   providers do. User-scope steps never set it.
7. **Streaming.** Install steps stream output (`streaming: true`), since Steam's first
   install downloads runtimes for minutes.

Errors from the inspection commands themselves (a non-zero exit from `flatpak remotes`, or
a `flatpak info` exit other than 0 or 1) are returned as plan errors that name the command,
not read as "missing".

## Step initializer

None. State is checked in `plan()` (as `claude.install` does), so no `SkipIf` is used.

## Platform constraints

Linux only. Manifests guard it with `where: os.name == "linux"` or the profile's
`has_flatpak` variable. The action itself does not check the OS; on a host without flatpak
it fails at plan time per Behavior 2.

## Test cases

A fake `flatpak` script on `PATH` controls the exit codes and output of `remotes` and
`info`. Tests that change `PATH` are `#[serial]`, and `plan()` is called on the concrete
struct (static dispatch, for tarpaulin).

- Deserializes `name`, `list`, `remote`, `remote_url`, `scope`; defaults are `flathub` and
  `system`.
- `scope: user` deserializes; an unknown scope value is a deserialize error.
- Empty `name` and `list` → error; both set → error.
- `flatpak` absent from `PATH` → error mentioning flatpak.
- Remote present, app installed → zero steps.
- Remote present, app missing → one install step with `--system`, `--noninteractive`, `-y`,
  the remote and the id, `privileged=true`.
- `scope: user`, app missing → `--user`, not privileged.
- Remote missing, `flathub` → remote-add step with the flathub URL, then the install step.
- Remote missing, custom remote with `remote_url` → remote-add with that URL.
- Remote missing, custom remote without `remote_url` → error naming the remote.
- `list` with one installed and one missing app → one install step, for the missing one.
- `list` with a duplicate id → one step.
- `flatpak info` exiting 2 → error naming the command (not treated as missing).
- `plan()` twice gives the same steps.
- Registration lists in `actions/mod.rs` tests include `flatpak.install`.

## Action catalog entry

`flatpak.install` — Install Flatpak apps from a remote (default Flathub), adding the remote
if it is missing. System scope (default, privileged) or user scope.

## Requirements

- **R1.** `flatpak.install` deserializes into `FlatpakInstall` with fields `name`, `list`, `remote` (default `flathub`), `remote_url` (optional) and `scope` (`system` default, or `user`).
- **R2.** `plan()` returns an error when neither or both of `name` and `list` are set.
- **R3.** `plan()` returns an error when `flatpak` is not on `PATH`.
- **R4.** `plan()` emits one `flatpak install --<scope> --noninteractive -y <remote> <id>` step per app that `flatpak info --<scope> <id>` reports missing (exit 1), and none for installed apps (exit 0).
- **R5.** `plan()` emits a `flatpak remote-add --if-not-exists --<scope> <remote> <url>` step before any install step when the remote is absent from `flatpak remotes --<scope> --columns=name`, using `https://dl.flathub.org/repo/flathub.flatpakrepo` as the URL for `flathub` when `remote_url` is unset.
- **R6.** `plan()` returns an error naming the remote when the remote is absent, it is not `flathub`, and `remote_url` is unset.
- **R7.** System-scope steps set `privileged: true`; user-scope steps do not.
- **R8.** A `flatpak info` exit other than 0 or 1, or a non-zero `flatpak remotes` exit, is a plan error naming the command.
- **R9.** The action is registered in `lib/src/actions/mod.rs`, listed in `README.md`'s action table, has `examples/flatpak/` YAML covering `name`, `list`, custom remote with `remote_url`, and `scope: user`, and is in the regenerated `jsonschemagen/schema.json`.
- **V1.** On `claude`, `etch apply --dry-run` of a manifest installing `com.valvesoftware.Steam` from `flathub` at system scope plans zero steps (it is already installed).
- **N1.** The action does not install, update or remove flatpak itself, and adds no `flatpak.update` or `flatpak.uninstall`.
- **N2.** No change to `PackageProviders` or the `package.*` actions.
- **N3.** No new crate dependency.
