use crate::actions::Action;
use crate::contexts::{to_tera, Contexts};
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
    // Used by plan() in Task 8; allow until then.
    #[allow(dead_code)]
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

    fn plan(&self, _manifest: &Manifest, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;

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
}
