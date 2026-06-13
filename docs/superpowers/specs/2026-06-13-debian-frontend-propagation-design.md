# DEBIAN_FRONTEND Propagation Design

## Overview

Extend `Aptitude::env()` with two additional environment variables so that all
apt invocations are fully non-interactive: `DEBCONF_NONINTERACTIVE_SEEN=true`
and `NEEDRESTART_MODE=a`. These propagate to dpkg hook scripts via the existing
`env KEY=VAL` sudo injection in `exec.rs`.

## Motivation

`etch apply` runs as a non-root user. Privileged apt invocations are wrapped as
`sudo env DEBIAN_FRONTEND=noninteractive apt install --yes ...`. Two classes of
dpkg hooks block unattended operation:

1. **needrestart** — prompts "which services need restarting?" after kernel or
   library updates. `NEEDRESTART_MODE=a` suppresses the prompt and restarts all
   affected services automatically.

2. **debconf dialogs** (e.g. `ubuntu-restricted-extras` → `ttf-mscorefonts-installer`)
   — `DEBCONF_NONINTERACTIVE_SEEN=true` combined with the existing
   `DEBIAN_FRONTEND=noninteractive` tells debconf all questions have been seen
   and to apply defaults silently. For packages requiring explicit EULA
   acceptance, the user must pre-seed via `command.run` + `debconf-set-selections`
   before the package install.

## Change

**File:** `lib/src/actions/package/providers/aptitude.rs`

```rust
fn env(&self) -> Vec<(String, String)> {
    vec![
        (String::from("DEBIAN_FRONTEND"), String::from("noninteractive")),
        (String::from("DEBCONF_NONINTERACTIVE_SEEN"), String::from("true")),
        (String::from("NEEDRESTART_MODE"), String::from("a")),
    ]
}
```

No other files change. The existing `elevate_if_required()` logic in `exec.rs`
already injects all entries from `environment` as `env KEY=VAL` arguments
before the apt command, so the new vars reach dpkg hooks automatically.

## What This Fixes

| Package                    | Root cause                          | Fix                                              |
| -------------------------- | ----------------------------------- | ------------------------------------------------ |
| `needrestart`              | Interactive "restart services" menu | `NEEDRESTART_MODE=a` auto-restart                |
| `ubuntu-restricted-extras` | debconf EULA dialog hangs           | `DEBCONF_NONINTERACTIVE_SEEN=true` uses defaults |

**Limitation:** `ttf-mscorefonts-installer` requires EULA acceptance. The
default debconf answer is "not accepted." `DEBCONF_NONINTERACTIVE_SEEN=true`
suppresses the prompt but uses the unset default — the install may succeed (apt
accepts the EULA on behalf of the user in noninteractive mode) or fail silently.
If explicit acceptance is required on a fresh system, pre-seed with:

```yaml
- action: command.run
  command: bash
  args:
      - -c
      - >-
          echo "ttf-mscorefonts-installer msttcorefonts/accepted-mscorefonts-eula select true"
          | debconf-set-selections
  privileged: true
```

## Testing

**Unit test** in `lib/src/actions/package/providers/aptitude.rs`:

```rust
#[test]
fn env_contains_required_noninteractive_vars() {
    let apt = Aptitude {};
    let env = apt.env();
    let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"DEBIAN_FRONTEND"));
    assert!(keys.contains(&"DEBCONF_NONINTERACTIVE_SEEN"));
    assert!(keys.contains(&"NEEDRESTART_MODE"));
    let map: std::collections::HashMap<&str, &str> =
        env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(map["DEBIAN_FRONTEND"], "noninteractive");
    assert_eq!(map["DEBCONF_NONINTERACTIVE_SEEN"], "true");
    assert_eq!(map["NEEDRESTART_MODE"], "a");
}
```

**Existing tests** (`apt_version_step` tests) already assert that `env` flows
into step `environment` fields — no changes needed there.

## Scope

- 1 file modified: `lib/src/actions/package/providers/aptitude.rs`
- 1 test added (same file)
- No new API surface, no manifest changes, no new dependencies
