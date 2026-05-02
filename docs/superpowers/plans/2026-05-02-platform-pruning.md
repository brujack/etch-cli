# Platform Pruning Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all code that supports platforms other than Ubuntu 24.04/26.04 and macOS, leaving exactly three package managers (apt, snap, homebrew) and two OS-specific provider families (linux, macos).

**Architecture:** Hard delete — 11 provider files are removed, 3 provider registry `mod.rs` files are rewritten to match, and 6 files with Windows/BSD platform guards are cleaned up. No stubs, no feature flags. Each task compiles and passes `make test` before committing.

**Tech Stack:** Rust, cargo, `os_info` crate for OS detection, `make test` = cargo fmt check + clippy -D warnings + cargo test

---

## File Map

**Delete (11 files):**

- `lib/src/actions/package/providers/bsdpkg.rs`
- `lib/src/actions/package/providers/dnf.rs`
- `lib/src/actions/package/providers/macports.rs`
- `lib/src/actions/package/providers/paru.rs`
- `lib/src/actions/package/providers/pkgin.rs`
- `lib/src/actions/package/providers/winget.rs`
- `lib/src/actions/package/providers/xbps.rs`
- `lib/src/actions/package/providers/yay.rs`
- `lib/src/actions/package/providers/zypper.rs`
- `lib/src/actions/group/providers/freebsd.rs`
- `lib/src/actions/user/providers/freebsd.rs`

**Rewrite (3 files):**

- `lib/src/actions/package/providers/mod.rs`
- `lib/src/actions/group/providers/mod.rs`
- `lib/src/actions/user/providers/mod.rs`

**Edit (6 files):**

- `lib/src/atoms/file/link.rs`
- `lib/src/actions/file/chown.rs`
- `lib/src/actions/directory/copy.rs`
- `lib/src/contexts/os.rs`
- `lib/src/steps/initializers/command_found.rs`
- `lib/src/manifests/mod.rs`

---

## Task 1: Delete package providers and rewrite package registry

**Files:**

- Delete: `lib/src/actions/package/providers/bsdpkg.rs`, `dnf.rs`, `macports.rs`, `paru.rs`, `pkgin.rs`, `winget.rs`, `xbps.rs`, `yay.rs`, `zypper.rs`
- Rewrite: `lib/src/actions/package/providers/mod.rs`

- [ ] **Step 1: Delete the 9 unsupported provider files**

```bash
cd lib/src/actions/package/providers
rm bsdpkg.rs dnf.rs macports.rs paru.rs pkgin.rs winget.rs xbps.rs yay.rs zypper.rs
```

- [ ] **Step 2: Rewrite `lib/src/actions/package/providers/mod.rs`**

Replace the entire file with:

```rust
mod aptitude;
use self::aptitude::Aptitude;
mod homebrew;
use self::homebrew::Homebrew;
mod snapcraft;
use self::snapcraft::Snapcraft;
use super::{repository::PackageRepository, PackageVariant};
use crate::contexts::Contexts;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(rename = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,

    #[serde(rename = "homebrew", alias = "brew")]
    Homebrew,

    #[serde(rename = "snapcraft", alias = "snap")]
    Snapcraft,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::Homebrew => Box::new(Homebrew {}),
            PackageProviders::Snapcraft => Box::new(Snapcraft {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        println!("Info: {info:?}");

        match info.os_type() {
            os_info::Type::Ubuntu => PackageProviders::Aptitude,
            os_info::Type::Macos => PackageProviders::Homebrew,
            _ => panic!(
                "Unsupported OS. Use provider: apt, snap, or brew explicitly."
            ),
        }
    }
}

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
}
```

- [ ] **Step 3: Verify compilation and tests pass**

```bash
make test
```

Expected: all checks pass, no unused import warnings, no dead code warnings.

- [ ] **Step 4: Commit**

```bash
git add lib/src/actions/package/providers/
git commit -m "chore: drop unsupported package providers; keep apt, snap, brew"
```

---

## Task 2: Delete FreeBSD group/user providers and rewrite their registries

**Files:**

- Delete: `lib/src/actions/group/providers/freebsd.rs`, `lib/src/actions/user/providers/freebsd.rs`
- Rewrite: `lib/src/actions/group/providers/mod.rs`, `lib/src/actions/user/providers/mod.rs`

- [ ] **Step 1: Delete the two FreeBSD provider files**

```bash
rm lib/src/actions/group/providers/freebsd.rs
rm lib/src/actions/user/providers/freebsd.rs
```

- [ ] **Step 2: Rewrite `lib/src/actions/group/providers/mod.rs`**

Replace the entire file with:

```rust
use crate::steps::Step;
mod none;
use self::none::NoneGroupProvider;
use super::GroupVariant;
use crate::contexts::Contexts;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod linux;
use self::linux::LinuxGroupProvider;
mod macos;
use self::macos::MacOsGroupProvider;

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum GroupProviders {
    #[serde(alias = "none")]
    None,

    #[serde(alias = "linux")]
    Linux,

    #[serde(alias = "macos")]
    MacOs,
}

impl GroupProviders {
    pub fn get_provider(self) -> Box<dyn GroupProvider> {
        match self {
            GroupProviders::None => Box::new(NoneGroupProvider {}),
            GroupProviders::Linux => Box::new(LinuxGroupProvider {}),
            GroupProviders::MacOs => Box::new(MacOsGroupProvider {}),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for GroupProviders {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        GroupProviders::Linux
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::Macos => GroupProviders::MacOs,
            _ => GroupProviders::None,
        }
    }
}

pub trait GroupProvider {
    fn add_group(&self, group: &GroupVariant, contexts: &Contexts) -> Vec<Step>;
}
```

- [ ] **Step 3: Rewrite `lib/src/actions/user/providers/mod.rs`**

Replace the entire file with:

```rust
use crate::steps::Step;
mod none;
use self::none::NoneUserProvider;
use super::{add_group::UserAddGroup, UserVariant};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod linux;
use self::linux::LinuxUserProvider;
mod macos;
use self::macos::MacOSUserProvider;

use crate::contexts::Contexts;

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum UserProviders {
    #[serde(alias = "none")]
    None,

    #[serde(alias = "linux")]
    Linux,

    #[serde(alias = "macos")]
    MacOs,
}

impl UserProviders {
    pub fn get_provider(self) -> Box<dyn UserProvider> {
        match self {
            UserProviders::None => Box::new(NoneUserProvider {}),
            UserProviders::Linux => Box::new(LinuxUserProvider {}),
            UserProviders::MacOs => Box::new(MacOSUserProvider {}),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for UserProviders {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        UserProviders::Linux
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::Macos => UserProviders::MacOs,
            _ => UserProviders::None,
        }
    }
}

pub trait UserProvider {
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
}
```

- [ ] **Step 4: Verify compilation and tests pass**

```bash
make test
```

Expected: all checks pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/group/providers/ lib/src/actions/user/providers/
git commit -m "chore: drop FreeBSD group and user providers"
```

---

## Task 3: Strip Windows branches from atoms and actions

**Files:**

- Edit: `lib/src/atoms/file/link.rs`
- Edit: `lib/src/actions/file/chown.rs`
- Edit: `lib/src/actions/directory/copy.rs`

- [ ] **Step 1: Edit `lib/src/atoms/file/link.rs` — remove Windows path prefix branch**

Find and replace this block in `plan()` (around line 71):

```rust
        let source = if cfg!(target_os = "windows") {
            const PREFIX: &str = r"\\?\";
            PathBuf::from(&self.source.display().to_string().replace(PREFIX, ""))
        } else {
            self.source.to_owned()
        };
```

Replace with:

```rust
        let source = self.source.to_owned();
```

- [ ] **Step 2: Edit `lib/src/atoms/file/link.rs` — remove Windows execute impl and drop cfg guard on unix impl**

Find and replace this block (around line 85):

```rust
    #[cfg(unix)]
    fn execute(&mut self) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }

    #[cfg(windows)]
    fn execute(&mut self) -> anyhow::Result<()> {
        if self.target.is_dir() {
            std::os::windows::fs::symlink_dir(&self.source, &self.target)?;
        } else {
            std::os::windows::fs::symlink_file(&self.source, &self.target)?;
        }

        Ok(())
    }
```

Replace with:

```rust
    fn execute(&mut self) -> anyhow::Result<()> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;

        Ok(())
    }
```

- [ ] **Step 3: Edit `lib/src/actions/file/chown.rs` — remove Windows warning stub and drop guards**

The file currently has two `plan` impls gated by `#[cfg(not(unix))]` and `#[cfg(unix)]`. Remove the `#[cfg(not(unix))]` stub entirely and remove the `#[cfg(unix)]` attribute from the remaining impl. Also remove the `#[cfg(unix)]` attribute from the `use crate::atoms::file::Chown;` import.

The top of the file should become:

```rust
use crate::actions::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::FileAction;
use crate::atoms::file::Chown;
```

And the `impl Action for FileChown` block should become:

```rust
impl Action for FileChown {
    fn summarize(&self) -> String {
        format!("Changing ownership for file {}", self.path)
    }

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        let steps = vec![crate::steps::Step {
            atom: Box::new(Chown {
                path: self.path.clone().parse()?,
                owner: self.user.clone().unwrap_or("".to_string()),
                group: self.group.clone().unwrap_or("".to_string()),
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        Ok(steps)
    }
}
```

- [ ] **Step 4: Edit `lib/src/actions/directory/copy.rs` — remove Windows Xcopy impl and drop cfg guard on unix impl**

Delete the entire `#[cfg(target_family = "windows")]` impl block (lines 19–38):

```rust
#[cfg(target_family = "windows")]
impl Action for DirectoryCopy {
    fn summarize(&self) -> String {
        format!("Copying {} to {}", self.from, self.to)
    }

    fn plan(&self, manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let from: String = self.resolve(manifest, &self.from).display().to_string();

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("Xcopy"),
                arguments: vec!["/E".to_string(), "/I".to_string(), from, self.to.clone()],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
```

Then change the remaining impl's attribute from `#[cfg(target_family = "unix")]` to nothing (unconditional):

```rust
impl Action for DirectoryCopy {
    fn summarize(&self) -> String {
        format!("Copying {} to {}", self.from, self.to)
    }
    // ... rest unchanged
```

- [ ] **Step 5: Verify compilation and tests pass**

```bash
make test
```

Expected: all checks pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/atoms/file/link.rs \
        lib/src/actions/file/chown.rs \
        lib/src/actions/directory/copy.rs
git commit -m "chore: remove Windows platform branches from atoms and actions"
```

---

## Task 4: Strip Windows and FreeBSD test blocks from support files

**Files:**

- Edit: `lib/src/contexts/os.rs`
- Edit: `lib/src/steps/initializers/command_found.rs`
- Edit: `lib/src/manifests/mod.rs`

- [ ] **Step 1: Edit `lib/src/contexts/os.rs` — remove Windows and FreeBSD test blocks**

In the `#[cfg(test)]` module, delete the `#[cfg(windows)]` test block:

```rust
    #[test]
    #[cfg(windows)]
    fn it_can_windows() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("windows")),
                "name" => assert_eq!(v.to_string(), String::from("windows")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }
```

And delete the `#[cfg(target_os = "freebsd")]` test block:

```rust
    #[test]
    #[cfg(target_os = "freebsd")]
    fn it_can_linux() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("freebsd")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }
```

- [ ] **Step 2: Edit `lib/src/steps/initializers/command_found.rs` — remove Windows-only tests**

Delete both `#[cfg(target_family = "windows")]` test blocks:

```rust
    #[cfg(target_family = "windows")]
    #[test]
    fn it_returns_true_when_found() {
        let initializer = CommandFound("cmd.exe");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }

    #[cfg(target_family = "windows")]
    #[test]
    fn return_true_windows_xcopy() {
        let initializer = CommandFound("Xcopy");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
```

The remaining unix test (`#[cfg(target_family = "unix")]`) stays as-is.

- [ ] **Step 3: Edit `lib/src/manifests/mod.rs` — remove Windows path test module**

Find and delete the entire Windows test module at the bottom of the file:

```rust
#[cfg(test)]
#[cfg(windows)]
mod test {
    use super::*;

    #[test]
    fn test_main_yaml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\main.yaml");

        assert_eq!(
            "test",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }
}
```

- [ ] **Step 4: Verify compilation and tests pass**

```bash
make test
```

Expected: all checks pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/contexts/os.rs \
        lib/src/steps/initializers/command_found.rs \
        lib/src/manifests/mod.rs
git commit -m "chore: remove Windows and FreeBSD test blocks"
```

---

## Task 5: Final verification and coverage check

- [ ] **Step 1: Run full test suite with coverage**

```bash
cargo tarpaulin --fail-under 25
```

Expected: passes (coverage likely higher than 25% since dead untested code was removed).

- [ ] **Step 2: Update CLAUDE.md to remove stale provider references**

In `CLAUDE.md`, find the Action Catalog section and update the `package.install` row's "Key fields" to reflect that the only providers are `apt`, `snap`, and `brew`. No other changes needed.

- [ ] **Step 3: Update the superpowers index**

In `docs/superpowers/README.md`, update the `platform-pruning` row status from `Pending` to `Done` and add a `> **Status: DONE**` banner at the top of this plan file.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md docs/superpowers/README.md docs/superpowers/plans/2026-05-02-platform-pruning.md
git commit -m "chore: mark platform pruning complete; update docs"
```
