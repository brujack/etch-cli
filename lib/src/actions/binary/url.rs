use crate::actions::Action;
use crate::atoms::binary::{ArchiveFormat, BinaryExtract, BinaryVerify};
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::contexts::{to_tera, Contexts};
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tera::Tera;

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryUrl {
    pub name: String,
    pub url: String,
    pub directory: String,
    pub version: Option<String>,
    pub file: Option<String>,
    pub sha256: Option<String>,
    pub privileged: Option<bool>,
}

impl BinaryUrl {
    fn render_url(&self, contexts: &Contexts) -> anyhow::Result<String> {
        let mut tera = Tera::default();
        let mut ctx = to_tera(contexts);
        if let Some(ref v) = self.version {
            ctx.insert("version", v);
        }
        tera.render_str(&self.url, &ctx)
            .map_err(|e| anyhow::anyhow!("binary.url: failed to render URL: {e}"))
    }
}

impl Action for BinaryUrl {
    fn summarize(&self) -> String {
        format!("Downloading binary from {} to {}", self.url, self.directory)
    }

    fn plan(&self, _manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let dest = PathBuf::from(format!("{}/{}", self.directory, self.name));
        if dest.exists() {
            return Ok(vec![]);
        }

        let url = self.render_url(contexts)?;
        let format = ArchiveFormat::detect(&url);

        if !matches!(format, ArchiveFormat::Raw) && self.file.is_none() {
            return Err(anyhow::anyhow!(
                "binary.url: 'file' is required for archive URLs (got: {})",
                url
            ));
        }

        let temp_path = PathBuf::from(format!("{}/{}.etch-tmp", self.directory, self.name));

        let mut steps: Vec<Step> = vec![Step {
            atom: Box::new(Download {
                url: url.clone(),
                to: temp_path.clone(),
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Some(ref expected) = self.sha256 {
            steps.push(Step {
                atom: Box::new(BinaryVerify {
                    path: temp_path.clone(),
                    expected: expected.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        steps.push(Step {
            atom: Box::new(BinaryExtract {
                src: temp_path,
                dest: dest.clone(),
                file: self.file.clone(),
                format,
            }),
            initializers: vec![],
            finalizers: vec![],
        });

        steps.push(Step {
            atom: Box::new(Chmod {
                path: dest,
                mode: 0o755,
            }),
            initializers: vec![],
            finalizers: vec![],
        });

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn render_url_no_template() {
        let action = BinaryUrl {
            name: String::from("mytool"),
            url: String::from("https://example.com/mytool"),
            directory: String::from("/usr/local/bin"),
            ..Default::default()
        };
        assert_eq!(
            "https://example.com/mytool",
            action.render_url(&Contexts::default()).unwrap()
        );
    }

    #[test]
    fn render_url_with_version() {
        let action = BinaryUrl {
            name: String::from("go"),
            url: String::from("https://go.dev/dl/go{{ version }}.linux-amd64.tar.gz"),
            directory: String::from("/usr/local/bin"),
            version: Some(String::from("1.22.0")),
            ..Default::default()
        };
        assert_eq!(
            "https://go.dev/dl/go1.22.0.linux-amd64.tar.gz",
            action.render_url(&Contexts::default()).unwrap()
        );
    }

    #[test]
    fn summarize_contains_url_and_dir() {
        let action = BinaryUrl {
            name: String::from("mytool"),
            url: String::from("https://example.com/mytool"),
            directory: String::from("/usr/local/bin"),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("example.com"), "summarize was: {s}");
        assert!(s.contains("/usr/local/bin"), "summarize was: {s}");
    }

    #[test]
    fn plan_skips_if_binary_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"existing").unwrap();
        let action = BinaryUrl {
            name: String::from("mytool"),
            url: String::from("https://example.com/mytool"),
            directory: tmp.path().display().to_string(),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_raw_binary_produces_three_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryUrl {
            name: String::from("mytool"),
            url: String::from("https://example.com/mytool"),
            directory: tmp.path().display().to_string(),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len()); // Download + BinaryExtract(Raw) + Chmod
    }

    #[test]
    fn plan_raw_binary_with_sha256_produces_four_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryUrl {
            name: String::from("mytool"),
            url: String::from("https://example.com/mytool"),
            directory: tmp.path().display().to_string(),
            sha256: Some(String::from("abc123")),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(4, steps.len()); // Download + BinaryVerify + BinaryExtract(Raw) + Chmod
    }

    #[test]
    fn plan_archive_produces_three_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryUrl {
            name: String::from("go"),
            url: String::from("https://go.dev/dl/go1.22.0.linux-amd64.tar.gz"),
            directory: tmp.path().display().to_string(),
            file: Some(String::from("go/bin/go")),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len()); // Download + BinaryExtract + Chmod
    }

    #[test]
    fn plan_archive_with_sha256_produces_four_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryUrl {
            name: String::from("go"),
            url: String::from("https://go.dev/dl/go1.22.0.linux-amd64.tar.gz"),
            directory: tmp.path().display().to_string(),
            file: Some(String::from("go/bin/go")),
            sha256: Some(String::from("abc123")),
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(4, steps.len()); // Download + BinaryVerify + BinaryExtract + Chmod
    }

    #[test]
    fn plan_archive_without_file_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryUrl {
            name: String::from("go"),
            url: String::from("https://go.dev/dl/go1.22.0.linux-amd64.tar.gz"),
            directory: tmp.path().display().to_string(),
            file: None,
            ..Default::default()
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("'file' is required"), "msg was: {msg}");
    }

    #[test]
    fn deserialization_minimal() {
        use crate::actions::Actions;
        let yaml = r#"
- action: binary.url
  name: mytool
  url: "https://example.com/mytool"
  directory: /usr/local/bin
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BinaryUrl(a)) => {
                assert_eq!("mytool", a.action.name);
                assert_eq!("https://example.com/mytool", a.action.url);
                assert_eq!("/usr/local/bin", a.action.directory);
                assert!(a.action.version.is_none());
                assert!(a.action.file.is_none());
                assert!(a.action.sha256.is_none());
            }
            _ => panic!("BinaryUrl didn't deserialize"),
        }
    }

    #[test]
    fn deserialization_full_with_alias() {
        use crate::actions::Actions;
        let yaml = r#"
- action: bin.url
  name: go
  url: "https://go.dev/dl/go{{ version }}.linux-amd64.tar.gz"
  directory: /usr/local/bin
  version: "1.22.0"
  file: go/bin/go
  sha256: "abc123"
  privileged: false
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BinaryUrl(a)) => {
                assert_eq!("go", a.action.name);
                assert_eq!(Some(String::from("1.22.0")), a.action.version);
                assert_eq!(Some(String::from("go/bin/go")), a.action.file);
                assert_eq!(Some(String::from("abc123")), a.action.sha256);
                assert_eq!(Some(false), a.action.privileged);
            }
            _ => panic!("bin.url alias didn't deserialize"),
        }
    }
}
