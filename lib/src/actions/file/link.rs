use super::{FileAction, FileActionConfig};
use crate::manifests::Manifest;
use crate::steps::initializers::FileExists;
use crate::steps::initializers::FlowControl::Ensure;
use crate::steps::Step;
use crate::utilities;
use crate::{actions::Action, contexts::Contexts};
use glob::glob as glob_expand;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec;
use tracing::error;

// TODO: Next Major Version - Deprecate from and to
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLink {
    pub from: Option<String>,
    pub source: Option<String>,

    pub target: Option<String>,
    pub to: Option<String>,

    pub glob: Option<String>,

    #[serde(default = "walk_dir_default")]
    pub walk_dir: bool,

    #[serde(flatten)]
    pub config: FileActionConfig,
}

fn walk_dir_default() -> bool {
    false
}

impl FileLink {
    fn source(&self) -> String {
        if self.source.is_none() && self.from.is_none() {
            error!("Field 'source' is required for file.link");
        }

        if let Some(ref source) = self.source {
            source.to_string()
        } else {
            // .unwrap() is safe here because we already checked for None
            self.from.clone().unwrap()
        }
    }

    fn target(&self) -> String {
        if self.target.is_none() && self.to.is_none() {
            error!("Field 'target' is required for file.link");
        }
        if let Some(ref target) = self.target {
            target.to_string()
        } else {
            // .unwrap() is safe here because we already checked for None
            self.to.clone().unwrap()
        }
    }

    pub fn plan_no_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        match to.parent() {
            Some(parent) => {
                vec![
                    Step {
                        atom: Box::new(DirCreate {
                            path: parent.to_path_buf(),
                        }),
                        initializers: vec![],
                        finalizers: vec![],
                    },
                    Step {
                        atom: Box::new(Link {
                            source: from.to_owned(),
                            target: to,
                        }),
                        initializers: vec![Ensure(Box::new(FileExists(from)))],
                        finalizers: vec![],
                    },
                ]
            }
            None => vec![],
        }
    }

    pub fn plan_privileged(
        from: PathBuf,
        to: PathBuf,
        privilege_provider: &str,
        walk_dir: bool,
    ) -> Vec<Step> {
        use crate::atoms::command::Exec;

        let make_mkdir = |parent: &std::path::Path| -> Step {
            Step {
                atom: Box::new(Exec {
                    command: "mkdir".into(),
                    arguments: vec!["-p".to_string(), parent.display().to_string()],
                    privileged: true,
                    privilege_provider: privilege_provider.to_string(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }
        };

        let make_ln = |src: &std::path::Path, tgt: &std::path::Path| -> Step {
            Step {
                atom: Box::new(Exec {
                    command: "ln".into(),
                    arguments: vec![
                        "-sf".to_string(),
                        src.display().to_string(),
                        tgt.display().to_string(),
                    ],
                    privileged: true,
                    privilege_provider: privilege_provider.to_string(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }
        };

        if from.is_file() || !walk_dir {
            match to.parent() {
                Some(parent) => vec![make_mkdir(parent), make_ln(&from, &to)],
                None => vec![],
            }
        } else {
            let mut steps = vec![];
            if let Ok(entries) = std::fs::read_dir(&from) {
                for entry in entries.flatten() {
                    let src_item = entry.path();
                    if let Some(file_name) = src_item.file_name() {
                        let tgt_item = to.join(file_name);
                        if let Some(parent) = tgt_item.parent() {
                            steps.push(make_mkdir(parent));
                        }
                        steps.push(make_ln(&src_item, &tgt_item));
                    }
                }
            }
            steps
        }
    }

    pub fn plan_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        let mut steps = vec![Step {
            atom: Box::new(DirCreate { path: to.clone() }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Ok(paths) = std::fs::read_dir(from) {
            paths.for_each(|path| {
                if let Ok(path) = path {
                    let p = path.path();

                    if let Some(file_name) = p.file_name() {
                        steps.push(Step {
                            atom: Box::new(Link {
                                source: p.clone(),
                                target: to.join(file_name),
                            }),
                            initializers: vec![Ensure(Box::new(FileExists(p.clone())))],
                            finalizers: vec![],
                        })
                    }
                }
            })
        }

        steps
    }
}

impl FileAction for FileLink {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileLink {
    fn state_key(&self) -> String {
        self.target()
    }

    fn summarize(&self) -> String {
        if let Some(ref pattern) = self.glob {
            return format!(
                "Linking files matching {} to {}",
                pattern,
                self.target
                    .clone()
                    .or_else(|| self.to.clone())
                    .unwrap_or_default()
            );
        }
        format!(
            "Linking file {} to {}",
            self.source
                .clone()
                .or_else(|| self.from.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            self.target
                .clone()
                .or_else(|| self.to.clone())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }

    fn plan(&self, manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if let Some(ref pattern) = self.glob {
            if self.source.is_some() || self.from.is_some() {
                return Err(anyhow::anyhow!(
                    "file.link: 'glob' and 'source'/'from' are mutually exclusive"
                ));
            }

            let glob_root = manifest
                .root_dir
                .clone()
                .ok_or_else(|| anyhow::anyhow!("file.link: manifest has no root_dir"))?
                .join("files");

            let full_pattern = glob_root.join(pattern);
            let full_pattern_str = full_pattern
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("file.link: pattern contains invalid UTF-8"))?;

            let matched: Vec<PathBuf> = glob_expand(full_pattern_str)?
                .filter_map(|r| r.ok())
                .filter(|p| p.is_file())
                .collect();

            if matched.is_empty() {
                return Err(anyhow::anyhow!(
                    "file.link: glob pattern '{}' matched no files in '{}'",
                    pattern,
                    glob_root.display()
                ));
            }

            let target_base = PathBuf::from(self.target());
            let privilege_provider = if self.config.privileged {
                Some(
                    utilities::get_privilege_provider(contexts)
                        .unwrap_or_else(|| "sudo".to_string()),
                )
            } else {
                None
            };

            let mut steps = Vec::new();
            for matched_path in matched {
                let relative = matched_path
                    .strip_prefix(&glob_root)
                    .map_err(|e| anyhow::anyhow!("file.link: strip_prefix failed: {}", e))?;
                let link_target = target_base.join(relative);

                if let Some(ref provider) = privilege_provider {
                    steps.extend(FileLink::plan_privileged(
                        matched_path,
                        link_target,
                        provider,
                        false,
                    ));
                } else {
                    steps.extend(FileLink::plan_no_walk(matched_path, link_target));
                }
            }
            return Ok(steps);
        }

        let from: PathBuf = self.resolve(manifest, self.source().as_str())?;

        let to = PathBuf::from(self.target());

        if self.config.privileged {
            let privilege_provider =
                utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
            let walk = self.walk_dir && !from.is_file();
            return Ok(FileLink::plan_privileged(
                from,
                to,
                &privilege_provider,
                walk,
            ));
        }

        // Can't walk a file
        if from.is_file() {
            return Ok(FileLink::plan_no_walk(from, to));
        }

        match self.walk_dir {
            false => Ok(FileLink::plan_no_walk(from, to)),
            true => Ok(FileLink::plan_walk(from, to)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::{Action, Actions},
        config::Config,
        contexts::build_contexts,
        manifests::Manifest,
    };

    use super::FileLink;

    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.link
  source: /opt/bin/tool
  target: /usr/local/bin/tool
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileLink didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_exec_steps_when_privileged() {
        use super::FileLink;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        use crate::config::Config;
        use crate::contexts::build_contexts;

        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let source_file = files_dir.join("mytool");
        std::fs::write(&source_file, b"binary").unwrap();

        let manifest = crate::manifests::Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());

        let action = FileLink {
            source: Some("mytool".to_string()),
            target: Some(real_tmp.join("linked").display().to_string()),
            config: FileActionConfig { privileged: true },
            ..Default::default()
        };
        let steps = action.plan(&manifest, &contexts).unwrap();
        // 2 steps: mkdir -p + ln -sf
        assert_eq!(2, steps.len());
        // Both steps are Exec atoms (not native DirCreate/Link atoms)
        assert!(steps[0].atom.to_string().contains("mkdir"));
        assert!(steps[1].atom.to_string().contains("ln"));
    }

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.link
  source: a
  target: b
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source());
                assert_eq!("b", action.action.target());
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };

        // Old style format
        let yaml = r#"
- action: file.link
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source());
                assert_eq!("b", action.action.target());
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_links_directories() {
        let source_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let manifest: Manifest = Manifest {
            r#where: None,
            root_dir: Some(source_dir.path().to_path_buf()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
            ..Default::default()
        };

        let config = Config::default();

        let contexts = build_contexts(&config);

        let target: String = source_dir
            .path()
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let file_link_action: FileLink = FileLink {
            source: Some(source_dir.path().to_str().unwrap().to_string()),
            target: Some(target),
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn it_links_file_directly() {
        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let source_file = files_dir.join("myfile.txt");
        std::fs::write(&source_file, b"hello").unwrap();

        let manifest: Manifest = Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };

        let config = Config::default();
        let contexts = build_contexts(&config);

        let target_path = real_tmp.join("linked_file");
        let file_link_action: FileLink = FileLink {
            source: Some("myfile.txt".to_string()),
            target: Some(target_path.display().to_string()),
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts).unwrap();
        // When source is a file: plan_no_walk returns 2 steps
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn it_can_walk_link_directories() {
        let source_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                panic!("could not create tempdir");
            }
        }
        .keep();

        // We'll expect 2 extra Atoms
        use rand::RngExt;
        use std::io::Write;

        let mut rng = rand::rng();
        let number_of_files: usize = rng.random_range(3..9);

        for i in 0..number_of_files {
            let path = source_dir.clone().join(format!("{i}.txt"));
            let mut file = std::fs::File::create(path).unwrap();
            writeln!(file, "Random {i}").unwrap();
        }

        let manifest: Manifest = Manifest {
            r#where: None,
            root_dir: Some(source_dir.clone()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
            ..Default::default()
        };

        let config = Config::default();

        let contexts = build_contexts(&config);

        let target: String = source_dir.parent().unwrap().to_str().unwrap().to_string();

        let file_link_action: FileLink = FileLink {
            source: Some(source_dir.to_str().unwrap().to_string()),
            target: Some(target),
            walk_dir: true,
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), number_of_files + 1);
    }

    #[test]
    fn glob_no_match_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        // No files — pattern matches nothing

        let manifest = Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());
        let action = FileLink {
            glob: Some("*.txt".to_string()),
            target: Some(real_tmp.join("dest").display().to_string()),
            ..Default::default()
        };
        let result = action.plan(&manifest, &contexts);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.to_string().contains("matched no files"),
            "error message should mention 'matched no files'"
        );
    }

    #[test]
    fn glob_and_source_both_set_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = Manifest {
            root_dir: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());
        let action = FileLink {
            glob: Some("*.txt".to_string()),
            source: Some("myfile.txt".to_string()),
            target: Some("/tmp/dest".to_string()),
            ..Default::default()
        };
        assert!(action.plan(&manifest, &contexts).is_err());
    }

    #[test]
    fn glob_summarize() {
        let action = FileLink {
            glob: Some("claude/*".to_string()),
            target: Some("~/.claude".to_string()),
            ..Default::default()
        };
        let summary = action.summarize();
        assert!(
            summary.contains("claude/*"),
            "summary should include pattern"
        );
        assert!(
            summary.contains("~/.claude"),
            "summary should include target"
        );
    }

    #[test]
    fn summarize_uses_source_and_target_fields() {
        let action = FileLink {
            source: Some("/opt/dotfiles/config".to_string()),
            target: Some("~/.config".to_string()),
            ..Default::default()
        };
        let summary = action.summarize();
        assert!(
            summary.contains("/opt/dotfiles/config"),
            "summary should include source, got: {summary}"
        );
        assert!(
            summary.contains("~/.config"),
            "summary should include target, got: {summary}"
        );
        assert!(
            !summary.contains("unknown"),
            "summary should not contain 'unknown', got: {summary}"
        );
    }

    #[test]
    fn glob_double_star_preserves_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let top_dir = real_tmp.join("files").join("claude");
        let sub_dir = top_dir.join("sub");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(top_dir.join("top.txt"), b"top").unwrap();
        std::fs::write(sub_dir.join("nested.txt"), b"nested").unwrap();

        let manifest = Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());
        let dest = real_tmp.join("dest");
        let action = FileLink {
            glob: Some("claude/**/*".to_string()),
            target: Some(dest.display().to_string()),
            ..Default::default()
        };
        let steps = action.plan(&manifest, &contexts).unwrap();
        // 2 files × 2 steps each = 4
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn glob_matches_top_level_files() {
        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files").join("claude");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("a.txt"), b"a").unwrap();
        std::fs::write(files_dir.join("b.txt"), b"b").unwrap();
        std::fs::write(files_dir.join("c.txt"), b"c").unwrap();

        let manifest = Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());
        let dest = real_tmp.join("dest");
        let action = FileLink {
            glob: Some("claude/*".to_string()),
            target: Some(dest.display().to_string()),
            ..Default::default()
        };
        let steps = action.plan(&manifest, &contexts).unwrap();
        // 2 steps per file: DirCreate + Link
        assert_eq!(steps.len(), 6);
    }

    #[test]
    fn glob_with_privileged_produces_exec_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files").join("conf");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("a.conf"), b"a").unwrap();
        std::fs::write(files_dir.join("b.conf"), b"b").unwrap();

        let manifest = crate::manifests::Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());
        let dest = real_tmp.join("etc");
        let action = FileLink {
            glob: Some("conf/*.conf".to_string()),
            target: Some(dest.display().to_string()),
            config: crate::actions::file::FileActionConfig { privileged: true },
            ..Default::default()
        };
        let steps = action.plan(&manifest, &contexts).unwrap();
        // 2 files × 2 steps each (mkdir + ln) = 4 steps
        assert_eq!(4, steps.len());
        // All steps are Exec atoms (privileged path)
        for step in &steps {
            let s = step.atom.to_string();
            assert!(
                s.contains("mkdir") || s.contains("ln"),
                "expected exec atom, got: {s}"
            );
        }
    }

    #[test]
    fn glob_deserialization() {
        let yaml = r#"
- action: file.link
  glob: "claude/*"
  target: /tmp/dest
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!(action.action.glob, Some("claude/*".to_string()));
                assert_eq!(action.action.target, Some("/tmp/dest".to_string()));
            }
            _ => panic!("FileLink with glob didn't deserialize"),
        }
    }
}
