# package.install Version Pinning Design

## Overview

Add an optional `version:` field to `package.install` that declares the exact version a package must be installed at. When `version:` is set, etch validates the installed version at apply time: if already correct it skips the install (idempotent); if wrong it returns an actionable error; if absent it installs the declared version.

## Motivation

Without version pinning, `package.install` installs whatever the package manager resolves as latest. On a shared dotfiles repo used across machines over months, this causes drift: one machine runs `git 2.43`, another runs `git 2.47`. Manifests that need a specific tool version (Python 3.11 for a particular project, a pinned `kubectl` version) have no way to enforce it today. Version pinning makes the declared state explicit and auditable — it extends the same principle that `binary.url`'s `version:` + `sha256:` fields already apply to downloaded binaries.

## Manifest Syntax

```yaml
# Homebrew formula with versioned tap (python@3.11)
- action: package.install
  name: python
  provider: homebrew
  version: "3.11"

# Homebrew cask — cask versions are not pinnable; version: + cask: true errors at plan time
# (see Error Handling)

# Apt exact version string (from `apt-cache show git`)
- action: package.install
  name: git
  provider: apt
  version: "1:2.43.0-1ubuntu7"

# Snap channel (not a semver string — snap uses channel names like stable/candidate/edge)
- action: package.install
  name: core
  provider: snap
  version: "stable"

# Without version: — existing behavior unchanged
- action: package.install
  name: curl
  provider: apt
```

**Constraints documented at manifest level:**

- `version:` is only valid with `name:`, not with `list:`. Multiple packages cannot share one version string. Providing `version:` + `list:` (or a multi-package `list:` without `name:`) is an error at plan time.
- Homebrew `version:` only works for formulae that ship a versioned tap (`python@3.11`, `node@20`). Formulae that track only the latest release (e.g. `ripgrep`) cannot be pinned by version — etch does not call `brew pin`; it routes through the `<name>@<version>` formula name. Attempting to pin an unversioned formula fails at install time with a brew error, not at etch plan time.
- Snap `version:` is a channel name (`stable`, `edge`, `candidate`, `1.0/stable`), not a semver. Comparing installed channel vs declared channel is a string equality check.

## Architecture

### Modified files

**`lib/src/actions/package/mod.rs`**

Add `version: Option<String>` to both `Package` and `PackageVariant`. `PackageVariant` is what gets passed to providers, so it must carry `version`.

```rust
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Package {
    name: Option<String>,
    #[serde(default)]
    list: Vec<String>,
    #[serde(default)]
    provider: PackageProviders,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    extra_args: Vec<String>,
    #[serde(default)]
    variants: HashMap<os_info::Type, PackageVariant>,
    #[serde(default)]
    file: bool,
    #[serde(default)]
    cask: bool,
    // NEW
    version: Option<String>,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageVariant {
    name: Option<String>,
    #[serde(default)]
    list: Vec<String>,
    #[serde(default)]
    provider: PackageProviders,
    #[serde(default)]
    extra_args: Vec<String>,
    #[serde(default)]
    file: bool,
    #[serde(default)]
    cask: bool,
    // NEW
    version: Option<String>,
}
```

The `From<&Package> for PackageVariant` impl must propagate `version` from base and variant in the same way as `cask` — variant's value takes precedence when the OS variant provides one, otherwise the base value is used.

**`lib/src/actions/package/install.rs`**

Add a plan-time guard before calling `provider.install()`:

```rust
fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
    let variant: PackageVariant = self.into();

    // version: + list: is always an error — version applies to one named package only
    if variant.version.is_some() && !variant.list.is_empty() {
        return Err(anyhow!(
            "package.install: 'version' cannot be used with 'list'; \
             pin one package at a time using 'name'"
        ));
    }

    // version: + cask: true is not supported (Homebrew casks have no versioned-formula route)
    if variant.version.is_some() && variant.cask {
        return Err(anyhow!(
            "package.install: 'version' is not supported with 'cask: true'; \
             Homebrew casks do not have versioned taps"
        ));
    }

    // ... rest of existing plan logic ...
    let box_provider = variant.provider.clone().get_provider();
    let provider = box_provider.deref();
    // ...
    atoms.append(&mut provider.install(&variant, context)?);
    Ok(atoms)
}
```

**`lib/src/actions/package/providers/mod.rs`**

Add `installed_version` to the `PackageProvider` trait — a method each provider implements to query the currently installed version of a single package. Returns `Ok(None)` if not installed, `Ok(Some(version))` if installed.

```rust
pub trait PackageProvider {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step>;
    fn has_repository(&self, package: &PackageRepository) -> bool;
    fn add_repository(
        &self,
        package: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>>;
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>>;
    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;

    // NEW — query installed version of a single named package; None = not installed
    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>>;
}
```

Each provider's `install()` implementation is updated: if `variant.version` is `Some(declared)`, call `self.installed_version(name)` first and branch on the result before building the install step. The version-check logic lives in `install()` rather than in a separate trait method so that providers can express the semantics naturally (e.g. snap channel vs semver).

**`lib/src/actions/package/providers/homebrew.rs`**

`installed_version()`:

```rust
fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
    // brew info --json=v2 <name> exits 0 even if not installed; parse JSON
    let out = Command::new("brew")
        .args(["info", "--json=v2", name])
        .output()?;
    if !out.status.success() {
        return Ok(None); // unknown formula
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    // Check formulae[0].installed[0].version — present only if installed
    let version = json["formulae"][0]["installed"][0]["version"]
        .as_str()
        .map(str::to_owned);
    Ok(version)
}
```

`install()` version-pinning logic:

```rust
fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    if let Some(ref declared) = package.version {
        let name = package.name.as_deref().unwrap_or_default();
        match self.installed_version(name)? {
            Some(ref actual) if actual == declared => return Ok(vec![]), // already correct
            Some(ref actual) => {
                return Err(anyhow::anyhow!(
                    "package {name} is at {actual}, declared version is {declared}; \
                     manually upgrade/downgrade and re-apply"
                ));
            }
            None => {
                // Not installed — install the versioned formula: <name>@<declared>
                let formula = format!("{name}@{declared}");
                return Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("brew"),
                        arguments: vec![String::from("install"), formula],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }]);
            }
        }
    }

    // Existing unversioned install logic (unchanged)
    let need_installed = self.query(package)?;
    if need_installed.is_empty() {
        return Ok(vec![]);
    }
    // ... build brew install step as before ...
}
```

**`lib/src/actions/package/providers/aptitude.rs`**

`installed_version()`:

```rust
fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
    let out = Command::new("dpkg-query")
        .args(["-W", "-f=${Version}", name])
        .output()?;
    if !out.status.success() {
        return Ok(None); // package not installed
    }
    let s = String::from_utf8(out.stdout)?.trim().to_owned();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}
```

`install()` version-pinning logic:

```rust
fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    if let Some(ref declared) = package.version {
        let name = package.name.as_deref().unwrap_or_default();
        match self.installed_version(name)? {
            Some(ref actual) if actual == declared => return Ok(vec![]),
            Some(ref actual) => {
                return Err(anyhow::anyhow!(
                    "package {name} is at {actual}, declared version is {declared}; \
                     manually upgrade/downgrade and re-apply"
                ));
            }
            None => {
                // apt-get install <pkg>=<version>
                let privilege_provider = utilities::get_privilege_provider(contexts)
                    .unwrap_or_else(|| "sudo".to_string());
                return Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("apt-get"),
                        arguments: vec![
                            String::from("install"),
                            String::from("--yes"),
                            format!("{name}={declared}"),
                        ],
                        environment: self.env(),
                        privileged: true,
                        privilege_provider,
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }]);
            }
        }
    }
    // Existing unversioned install logic (unchanged)
    // ...
}
```

**`lib/src/actions/package/providers/snapcraft.rs`**

`installed_version()` queries the tracking channel, not the revision:

```rust
fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>> {
    // `snap list <name>` exits non-zero if not installed
    let out = Command::new("snap").args(["list", name]).output()?;
    if !out.status.success() {
        return Ok(None);
    }
    // Output: Name  Version  Rev  Tracking  Publisher  Notes
    // Parse Tracking column (index 3) from the data row (line 1)
    let line = String::from_utf8(out.stdout)?;
    let data = line.lines().nth(1).unwrap_or("").split_whitespace().collect::<Vec<_>>();
    Ok(data.get(3).map(|s| s.to_string()))
}
```

`install()` version-pinning logic for snap uses `--channel=<declared>`:

```rust
fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    let privilege_provider = utilities::get_privilege_provider(contexts)
        .unwrap_or_else(|| "sudo".to_string());

    if let Some(ref declared) = package.version {
        let name = package.name.as_deref().unwrap_or_default();
        match self.installed_version(name)? {
            Some(ref actual) if actual == declared => return Ok(vec![]),
            Some(ref actual) => {
                return Err(anyhow::anyhow!(
                    "package {name} is tracking channel {actual}, declared version is {declared}; \
                     manually upgrade/downgrade and re-apply"
                ));
            }
            None => {
                return Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: String::from("snap"),
                        arguments: vec![
                            String::from("install"),
                            format!("--channel={declared}"),
                            name.to_string(),
                        ],
                        privileged: true,
                        privilege_provider,
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }]);
            }
        }
    }
    // Existing unversioned install logic (unchanged)
    // ...
}
```

## Error Handling

| Condition                                            | Behavior                                                                                                                            |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `version:` + `list:` both set                        | `plan()` fails: `"'version' cannot be used with 'list'; pin one package at a time using 'name'"`                                    |
| `version:` + `cask: true`                            | `plan()` fails: `"'version' is not supported with 'cask: true'; Homebrew casks do not have versioned taps"`                         |
| Version set, package not installed                   | Install at declared version (brew: `<name>@<version>`; apt: `<pkg>=<version>`; snap: `--channel=<version>`)                         |
| Version set, correct version installed               | `Ok(vec![])` — skip, no steps emitted                                                                                               |
| Version set, wrong version installed                 | `install()` returns `Err`: `"package <name> is at <actual>, declared version is <wanted>; manually upgrade/downgrade and re-apply"` |
| Brew formula has no versioned tap                    | `brew install <name>@<version>` exits non-zero; atom propagates the brew error message                                              |
| `installed_version()` query fails (tool not in PATH) | `anyhow::Error` propagated — provider not available, caught by `provider.available()` earlier                                       |
| Provider is snap, `snap list` header parse fails     | `installed_version()` returns `Ok(None)` — treated as not installed                                                                 |

## Testing

**Unit tests (`mod.rs`):**

- Deserialization: `version: "3.11"` round-trips correctly through `Package` → `PackageVariant`
- Deserialization: omitting `version:` gives `None`
- `plan()` rejects `version:` + `list:` with correct error text
- `plan()` rejects `version:` + `cask: true` with correct error text
- `From<&Package> for PackageVariant` propagates `version` from base when no OS variant exists
- `From<&Package> for PackageVariant` variant's `version` overrides base when OS variant provides one

**Unit tests (`providers/homebrew.rs`):**

- `install()` with `version: Some("3.11")`, `installed_version` stubbed to return `None` → step has `brew install python@3.11`
- `install()` with `version: Some("3.11")`, `installed_version` stubbed to return `Some("3.11")` → empty steps (skip)
- `install()` with `version: Some("3.11")`, `installed_version` stubbed to return `Some("3.12")` → `Err` with mismatch message

Stubbing `installed_version` requires extracting the subprocess call into a mockable helper or using a test double via trait object — the preferred approach is a `#[cfg(test)]` helper that accepts a closure for `installed_version`, keeping the production code free of test plumbing. Alternatively, tests can write a `MockHomebrew` struct that hard-codes the return value and implements `PackageProvider` directly.

**Unit tests (`providers/aptitude.rs`):**

- Same three cases as homebrew: not installed → install step with `git=1:2.43.0-1ubuntu7`; correct → skip; wrong → error

**Unit tests (`providers/snapcraft.rs`):**

- `installed_version` output parsing: verify correct column (index 3) is extracted from `snap list` output
- Same three install cases: not installed → step with `--channel=stable`; correct channel → skip; wrong channel → error

No live package manager tests. All version-path tests mock the subprocess output. Existing tests that exercise the unversioned code paths must continue to pass unchanged.
