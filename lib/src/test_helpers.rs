use crate::contexts::Contexts;
use crate::manifests::Manifest;
use std::ffi::{OsStr, OsString};
use std::path::Path;

pub fn make_manifest(dir: &Path) -> Manifest {
    Manifest {
        root_dir: Some(dir.to_path_buf()),
        ..Default::default()
    }
}

pub fn make_contexts() -> Contexts {
    Contexts::default()
}

/// Sets or removes one environment variable and restores its previous value on drop.
///
/// Every test that holds one must be `#[serial]`; see the SAFETY note on `apply`.
#[derive(Debug)]
#[must_use = "bind to a named variable (let _g = ...); let _ = drops and restores at once"]
pub struct EnvGuard {
    key: OsString,
    previous: Option<OsString>,
}

impl EnvGuard {
    /// Sets `key` to `value`, remembering the previous value.
    pub fn set(key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        let previous = std::env::var_os(&key);
        apply(&key, Some(value.as_ref()));
        Self { key, previous }
    }

    /// Removes `key`, remembering the previous value.
    pub fn remove(key: impl AsRef<OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        let previous = std::env::var_os(&key);
        apply(&key, None);
        Self { key, previous }
    }

    /// Prepends `dir` to `PATH`: the mock-binary pattern used by 115 tests.
    pub fn prepend_path(dir: &Path) -> Self {
        let previous = std::env::var_os("PATH");
        let mut entries = vec![dir.to_path_buf()];
        if let Some(current) = &previous {
            entries.extend(std::env::split_paths(current));
        }
        let joined = std::env::join_paths(entries).expect("PATH entries must not contain ':'");
        apply(OsStr::new("PATH"), Some(&joined));
        Self {
            key: OsString::from("PATH"),
            previous,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        apply(&self.key, self.previous.as_deref());
    }
}

/// The only environment mutation in the crate's tests.
///
/// SAFETY: `set_var` and `remove_var` are sound when no other thread reads or writes the
/// environment concurrently. Under `cargo nextest` (`make test`) each test is its own
/// process, so that holds. Under `cargo test` and tarpaulin, tests are threads in one
/// process: `#[serial]` orders guard holders against each other, but a non-`#[serial]`
/// test that spawns a `Command` or calls `env::var` still reads the environment
/// concurrently. That residual race is unchanged from a bare `set_var`; serialising every
/// process-spawning test is out of scope.
fn apply(key: &OsStr, value: Option<&OsStr>) {
    #[expect(
        unsafe_code,
        reason = "test-only env mutation; serialised by #[serial], see SAFETY note"
    )]
    unsafe {
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnvGuard;
    use serial_test::serial;
    use std::path::Path;

    #[test]
    #[serial]
    fn set_restores_previous_value() {
        let key = "ETCH_ENVGUARD_T_SET_PREV";
        let _outer = EnvGuard::set(key, "original");
        {
            let _g = EnvGuard::set(key, "changed");
            assert_eq!(std::env::var(key).unwrap(), "changed");
        }
        assert_eq!(std::env::var(key).unwrap(), "original");
    }

    #[test]
    #[serial]
    fn set_restores_absence() {
        let key = "ETCH_ENVGUARD_T_SET_ABSENT";
        let _clear = EnvGuard::remove(key);
        {
            let _g = EnvGuard::set(key, "temporary");
            assert_eq!(std::env::var(key).unwrap(), "temporary");
        }
        assert!(std::env::var_os(key).is_none());
    }

    #[test]
    #[serial]
    fn remove_restores_value() {
        let key = "ETCH_ENVGUARD_T_REMOVE";
        let _outer = EnvGuard::set(key, "kept");
        {
            let _g = EnvGuard::remove(key);
            assert!(std::env::var_os(key).is_none());
        }
        assert_eq!(std::env::var(key).unwrap(), "kept");
    }

    #[test]
    #[serial]
    fn prepend_path_puts_dir_first_and_restores() {
        let before = std::env::var_os("PATH");
        {
            let _g = EnvGuard::prepend_path(Path::new("/etch-envguard-t-dir"));
            let now = std::env::var_os("PATH").expect("PATH set");
            let first = std::env::split_paths(&now).next().expect("non-empty PATH");
            assert_eq!(first, Path::new("/etch-envguard-t-dir"));
        }
        assert_eq!(std::env::var_os("PATH"), before);
    }

    #[test]
    #[serial]
    fn restore_happens_on_panic() {
        let key = "ETCH_ENVGUARD_T_PANIC";
        let _outer = EnvGuard::set(key, "before");
        let result = std::panic::catch_unwind(|| {
            let _g = EnvGuard::set(key, "during");
            panic!("boom");
        });
        assert!(result.is_err());
        assert_eq!(std::env::var(key).unwrap(), "before");
    }
}
