use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{FileAction, FileActionConfig};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFlags {
    pub path: String,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

impl FileAction for FileFlags {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileFlags {
    fn summarize(&self) -> String {
        format!("Set BSD flags {:?} on {}", self.flags, self.path)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        #[cfg(not(target_os = "macos"))]
        return Err(anyhow!("file.flags is only supported on macOS"));

        #[cfg(target_os = "macos")]
        {
            use crate::atoms::file::chflags::compute_desired;

            // Validate flags eagerly so we fail before touching the filesystem.
            compute_desired(0, &self.flags)?;

            if self.config.privileged {
                use crate::atoms::command::Exec;
                use crate::utilities;
                use std::ffi::CString;
                use std::os::unix::ffi::OsStrExt;
                use std::path::Path;

                // Read current flags to determine the full desired flag set.
                // chflags replaces ALL user flags — passing only the delta would
                // clobber flags not mentioned in this action.
                let path = Path::new(&self.path);
                let cstr = CString::new(path.as_os_str().as_bytes())
                    .map_err(|e| anyhow!("invalid path: {e}"))?;
                let mut sb: libc::stat = unsafe { std::mem::zeroed() };
                if unsafe { libc::stat(cstr.as_ptr(), &mut sb) } != 0 {
                    return Err(anyhow!(
                        "stat({:?}) failed: {}",
                        self.path,
                        std::io::Error::last_os_error()
                    ));
                }
                let current = sb.st_flags;
                let desired = compute_desired(current, &self.flags)?;

                if current == desired {
                    return Ok(vec![]);
                }

                // Convert desired bitmask back to chflags flag names.
                // chflags replaces all user flags with exactly what you name,
                // so we must name the full desired state, not just the delta.
                let mut names: Vec<&str> = Vec::new();
                if desired & crate::atoms::file::chflags::UF_HIDDEN != 0 {
                    names.push("hidden");
                }
                if desired & crate::atoms::file::chflags::UF_IMMUTABLE != 0 {
                    names.push("uchg");
                }
                let flags_str = if names.is_empty() {
                    "none".to_string()
                } else {
                    names.join(",")
                };

                let privilege_provider = utilities::get_privilege_provider(contexts)
                    .unwrap_or_else(|| "sudo".to_string());
                return Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: "chflags".into(),
                        arguments: vec![flags_str, self.path.clone()],
                        privileged: true,
                        privilege_provider,
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }]);
            }

            Ok(vec![Step {
                atom: Box::new(crate::atoms::file::Chflags {
                    path: self.path.clone().into(),
                    flags: self.flags.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.flags
  path: /tmp/testfile
  flags: [hidden, uchg]
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileFlags(action)) => {
                assert_eq!("/tmp/testfile", action.action.path);
                assert_eq!(vec!["hidden", "uchg"], action.action.flags);
                assert!(!action.action.config.privileged);
            }
            _ => panic!("FileFlags didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.flags
  path: /tmp/testfile
  flags: [nohidden]
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileFlags(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileFlags didn't deserialize to the correct type"),
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn plan_errors_on_unknown_flag() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/testfile"),
            flags: vec!["badname".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let result = action.plan(
            &crate::manifests::Manifest::default(),
            &crate::contexts::Contexts::default(),
        );
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("unknown flag: badname"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn plan_returns_chflags_step_when_not_privileged() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        let action = FileFlags {
            path: path.display().to_string(),
            flags: vec!["hidden".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.to_string().contains("hidden"));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn plan_errors_on_non_macos() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/testfile"),
            flags: vec!["hidden".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let result = action.plan(
            &crate::manifests::Manifest::default(),
            &crate::contexts::Contexts::default(),
        );
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("only supported on macOS"));
    }

    #[test]
    fn summarize_includes_path_and_flags() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/myfile"),
            flags: vec!["uchg".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let s = action.summarize();
        assert!(s.contains("/tmp/myfile"));
        assert!(s.contains("uchg"));
    }
}
