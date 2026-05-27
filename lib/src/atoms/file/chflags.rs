use crate::atoms::{Atom, Outcome};
use anyhow::anyhow;
use std::ffi::CString;
use std::path::PathBuf;

pub const UF_HIDDEN: u32 = 0x8000;
pub const UF_IMMUTABLE: u32 = 0x0002;

pub struct Chflags {
    pub path: PathBuf,
    pub flags: Vec<String>,
}

pub(crate) fn compute_desired(current: u32, flags: &[String]) -> anyhow::Result<u32> {
    let mut desired = current;
    for flag in flags {
        match flag.as_str() {
            "hidden" => desired |= UF_HIDDEN,
            "nohidden" => desired &= !UF_HIDDEN,
            "uchg" => desired |= UF_IMMUTABLE,
            "nouchg" => desired &= !UF_IMMUTABLE,
            other => return Err(anyhow!("unknown flag: {other}")),
        }
    }
    Ok(desired)
}

fn get_st_flags(path: &std::path::Path) -> anyhow::Result<u32> {
    let cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| anyhow!("invalid path: {e}"))?;
    let mut sb: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::stat(cstr.as_ptr(), &mut sb) } != 0 {
        return Err(anyhow!(
            "stat({:?}) failed: {}",
            path,
            std::io::Error::last_os_error()
        ));
    }
    Ok(sb.st_flags)
}

impl std::fmt::Display for Chflags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Set BSD flags {:?} on {}",
            self.flags,
            self.path.display()
        )
    }
}

impl Atom for Chflags {
    fn plan(&self) -> anyhow::Result<Outcome> {
        let current = get_st_flags(&self.path)?;
        let desired = compute_desired(current, &self.flags)?;
        Ok(Outcome {
            side_effects: vec![],
            should_run: current != desired,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let current = get_st_flags(&self.path)?;
        let desired = compute_desired(current, &self.flags)?;
        let cstr = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| anyhow!("invalid path: {e}"))?;
        if unsafe { libc::chflags(cstr.as_ptr(), desired as libc::c_uint) } != 0 {
            return Err(anyhow!(
                "chflags({:?}, {:#x}) failed: {}",
                self.path,
                desired,
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    #[test]
    fn compute_desired_hidden_sets_bit() {
        let flags = vec!["hidden".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, UF_HIDDEN);
    }

    #[test]
    fn compute_desired_nohidden_clears_bit() {
        let flags = vec!["nohidden".to_string()];
        let desired = compute_desired(UF_HIDDEN, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, 0);
    }

    #[test]
    fn compute_desired_uchg_sets_bit() {
        let flags = vec!["uchg".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_IMMUTABLE, UF_IMMUTABLE);
    }

    #[test]
    fn compute_desired_nouchg_clears_bit() {
        let flags = vec!["nouchg".to_string()];
        let desired = compute_desired(UF_IMMUTABLE, &flags).unwrap();
        assert_eq!(desired & UF_IMMUTABLE, 0);
    }

    #[test]
    fn compute_desired_combined_flags() {
        let flags = vec!["hidden".to_string(), "uchg".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, UF_HIDDEN);
        assert_eq!(desired & UF_IMMUTABLE, UF_IMMUTABLE);
    }

    #[test]
    fn compute_desired_nohidden_noop_when_already_clear() {
        let flags = vec!["nohidden".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired, 0);
    }

    #[test]
    fn compute_desired_unknown_flag_errors() {
        let flags = vec!["badname".to_string()];
        let result = compute_desired(0, &flags);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown flag: badname"));
    }

    #[test]
    fn plan_should_run_false_when_already_at_desired_state() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();
        let atom = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_true_when_flag_not_set() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();
        let atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_errors_on_nonexistent_path() {
        let atom = Chflags {
            path: std::path::PathBuf::from("/nonexistent/path/file.txt"),
            flags: vec!["hidden".to_string()],
        };
        assert!(atom.plan().is_err());
    }

    #[test]
    fn execute_sets_flag_and_plan_returns_false_after() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();
        let mut atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };
        assert!(atom.plan().unwrap().should_run);
        atom.execute().unwrap();
        assert!(!atom.plan().unwrap().should_run);
        // Clean up: clear the flag before tempdir is dropped
        let mut cleanup = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        cleanup.execute().unwrap();
    }

    #[test]
    fn execute_clears_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();
        let mut set_atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };
        set_atom.execute().unwrap();
        let mut clear_atom = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        assert!(clear_atom.plan().unwrap().should_run);
        clear_atom.execute().unwrap();
        assert!(!clear_atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_includes_flags_and_path() {
        let atom = Chflags {
            path: std::path::PathBuf::from("/tmp/myfile"),
            flags: vec!["hidden".to_string()],
        };
        let s = format!("{atom}");
        assert!(s.contains("myfile"));
        assert!(s.contains("hidden"));
    }
}
