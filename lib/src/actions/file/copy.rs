use super::FileAction;
use super::{default_chmod, from_octal};
#[cfg(unix)]
use crate::atoms::file::Chown;
use crate::atoms::file::Decrypt;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::tera_functions::register_functions;
use crate::{actions::Action, contexts::to_tera};
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::path::PathBuf;
use tera::Tera;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCopy {
    #[serde(alias = "source")]
    pub from: String,

    #[serde(alias = "target")]
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,

    pub passphrase: Option<String>,

    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,

    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,
}

fn default_template() -> bool {
    false
}

impl FileCopy {}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn summarize(&self) -> String {
        format!("Copy file from {} to {}", self.from, self.to)
    }

    fn plan(
        &self,
        manifest: &Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let contents = match self.load(manifest, &self.from) {
            Ok(contents) => {
                if self.template {
                    let mut tera = Tera::default();
                    register_functions(&mut tera);

                    let content_as_str = std::str::from_utf8(&contents)?;

                    match tera.render_str(content_as_str, &to_tera(context)) {
                        Ok(rendered) => rendered,
                        Err(err) => match err.source() {
                            Some(source) => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {source}"
                                ));
                            }
                            None => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {err}"
                                ));
                            }
                        },
                    }
                    .as_bytes()
                    .to_vec()
                } else {
                    contents
                }
            }
            Err(err) => {
                return Err(anyhow!("Failed to get contents for FileCopy action: {err}"));
            }
        };

        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::{Chmod, Create, SetContents};

        let mut path = PathBuf::from(&self.to);

        if path.is_dir() {
            if let Some(file_name) = PathBuf::from(self.from.clone()).file_name() {
                path = path.join(file_name);
            }
        }

        let parent = path.clone();
        let mut steps = vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent
                        .parent()
                        .ok_or_else(|| {
                            anyhow!("Failed to get parent directory for FileCopy action")
                        })?
                        .into(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Create { path: path.clone() }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Chmod {
                    path: path.clone(),
                    mode: self.chmod,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ];

        #[cfg(unix)]
        let path_for_chown = path.clone();
        if let Some(passphrase) = self.passphrase.to_owned() {
            steps.push(Step {
                atom: Box::new(Decrypt {
                    encrypted_content: contents,
                    path,
                    passphrase,
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        } else {
            steps.push(Step {
                atom: Box::new(SetContents { path, contents }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        #[cfg(unix)]
        if let Some(user) = self.owner_user.clone() {
            if let Some(group) = self.owner_group.clone() {
                steps.push(Step {
                    atom: Box::new(Chown {
                        path: path_for_chown,
                        owner: user.clone(),
                        group: group.clone(),
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                })
            }
        }

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.copy
  from: a
  to: b
  chmod: "0777"
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!(0o777, action.action.chmod);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_be_deserialized_chown() {
        let yaml = r#"
- action: file.copy
  from: a
  to: b
  chmod: "0777"
  owned_by_user: test
  owned_by_group: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!(0o777, action.action.chmod);
                assert_eq!("test", action.action.owner_user.as_deref().unwrap());
                assert_eq!("test", action.action.owner_group.as_deref().unwrap());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn plan_returns_steps_for_valid_source() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("source.txt");
        std::fs::write(&src, b"hello").unwrap();

        let action = FileCopy {
            from: src.display().to_string(),
            to: tmp.path().join("dest.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        // DirCreate + Create + Chmod + SetContents = 4 steps
        assert_eq!(4, steps.len());
    }

    #[test]
    fn plan_errors_when_source_missing() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let action = FileCopy {
            from: tmp.path().join("nonexistent.txt").display().to_string(),
            to: tmp.path().join("dest.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        assert!(action.plan(&manifest, &contexts).is_err());
    }

    #[test]
    fn plan_template_rendering() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        // Source file with a Tera template
        let files_dir = tmp.path().join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let src = files_dir.join("tmpl.txt");
        std::fs::write(&src, b"hello world").unwrap();

        let action = FileCopy {
            from: "tmpl.txt".to_string(),
            to: tmp.path().join("output.txt").display().to_string(),
            template: true,
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(4, steps.len()); // DirCreate + Create + Chmod + SetContents
    }

    #[test]
    fn plan_with_passphrase_uses_decrypt_step() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let files_dir = tmp.path().join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let src = files_dir.join("secret.txt");
        std::fs::write(&src, b"encrypted content").unwrap();

        let action = FileCopy {
            from: "secret.txt".to_string(),
            to: tmp.path().join("output.txt").display().to_string(),
            passphrase: Some("password".to_string()),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        // DirCreate + Create + Chmod + Decrypt = 4 steps
        assert_eq!(4, steps.len());
    }

    #[test]
    fn plan_with_to_as_directory() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let files_dir = tmp.path().join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let src = files_dir.join("myfile.txt");
        std::fs::write(&src, b"content").unwrap();

        // dest_dir is an existing directory
        let dest_dir = tmp.path().join("destdir");
        std::fs::create_dir_all(&dest_dir).unwrap();

        let action = FileCopy {
            from: "myfile.txt".to_string(),
            to: dest_dir.display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(4, steps.len());
    }

    #[test]
    fn summarize_includes_paths() {
        use super::FileCopy;
        use crate::actions::Action;
        let action = FileCopy {
            from: "src.txt".to_string(),
            to: "dst.txt".to_string(),
            ..Default::default()
        };
        let summary = action.summarize();
        assert!(summary.contains("src.txt"));
        assert!(summary.contains("dst.txt"));
    }

    #[test]
    fn plan_template_rendering_error_returns_err() {
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let files_dir = tmp.path().join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let src = files_dir.join("bad_tmpl.txt");
        // read_file_contents with a nonexistent path causes a Tera render error
        std::fs::write(
            &src,
            b"{{ read_file_contents(path=\"/no/such/path/xyz\") }}",
        )
        .unwrap();

        let action = FileCopy {
            from: "bad_tmpl.txt".to_string(),
            to: tmp.path().join("output.txt").display().to_string(),
            template: true,
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let result = action.plan(&manifest, &contexts);
        match result {
            Err(e) => assert!(e
                .to_string()
                .contains("Failed to render contents for FileCopy action")),
            Ok(_) => panic!("expected plan to fail on invalid template"),
        }
    }

    #[test]
    fn plan_errors_when_source_is_directory() {
        // fs::read() on a directory returns an OS error (not NotFound), covering
        // the _ => branch in FileAction::load()
        use super::FileCopy;
        use crate::actions::Action;
        let tmp = tempfile::tempdir().unwrap();
        let files_dir = tmp.path().join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        // Create a directory where a file is expected
        std::fs::create_dir(files_dir.join("mydir.txt")).unwrap();

        let action = FileCopy {
            from: "mydir.txt".to_string(),
            to: tmp.path().join("output.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let result = action.plan(&manifest, &contexts);
        assert!(result.is_err());
    }
}
