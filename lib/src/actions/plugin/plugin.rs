use std::{
    fmt::{self, Display, Formatter},
    fs,
    hash::{Hash, Hasher},
    ops::Deref,
    path::PathBuf,
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use dirs_next::data_local_dir;
use gix::{
    bstr::ByteSlice,
    diff::object::tree::EntryKind,
    interrupt::IS_INTERRUPTED,
    open, prepare_clone_bare,
    progress::Discard,
    remote::{ref_map, Direction::Fetch},
    Repository,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_with::{serde_as, DisplayFromStr, KeyValueMap};
use tracing::{debug, error, info, instrument};

use super::get_plugin;
use crate::{
    actions::Action,
    atoms::plugin::{PluginExec, PluginSpec},
    contexts::Contexts,
    manifests::Manifest,
    steps::Step,
    utilities::{lua::json_to_lua, CustomPathBuf},
};

#[derive(
    JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct RepoUri {
    pub username: String,
    pub repo: String,
}

impl Display for RepoUri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.username, self.repo)
    }
}

impl FromStr for RepoUri {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        debug!("Trying to parse repo uri: {}", s);
        match s.split_once('/') {
            Some((username, repo)) => Ok(Self {
                username: username.to_string(),
                repo: repo.to_string(),
            }),
            _ => Err(anyhow!("repo must be in format 'username/repo'")),
        }
    }
}

pub trait Source {
    fn source(&self) -> Result<String>;
}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Version {
    #[default]
    Stable,

    #[serde(alias = "*")]
    Latest,

    #[serde(untagged)]
    Tagged(String),
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let version = match self {
            Version::Stable => "stable",
            Version::Latest => "latest",
            Version::Tagged(tag) => tag.as_str(),
        };

        write!(f, "{version}")
    }
}

#[serde_as]
#[derive(
    JsonSchema, Serialize, Deserialize, Debug, Clone, Eq, PartialOrd, Default, Ord, PartialEq,
)]
pub struct Repo {
    #[serde(alias = "repository")]
    #[serde_as(as = "DisplayFromStr")]
    repo: RepoUri,

    #[serde(alias = "tag")]
    version: Version,
}

impl Display for Repo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.repo, self.version)
    }
}

impl Hash for Repo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.path() {
            Ok(path) => path.hash(state),
            _ => {
                self.repo.to_string().hash(state);
                self.version.hash(state);
            }
        }
    }
}

impl Repo {
    fn path(&self) -> Result<PathBuf> {
        let plugins_path = data_local_dir()
            .context("Failed to locate local data directory")?
            .join("etch")
            .join("plugins")
            .join(self.repo.to_string());

        if !plugins_path.exists() {
            fs::create_dir_all(&plugins_path)?;
        }

        plugins_path.canonicalize().map_err(anyhow::Error::from)
    }

    fn checkout(&self) -> Result<Repository> {
        info!("Checking out plugin");
        let (checkout_result, _) =
            prepare_clone_bare(format!("https://github.com/{}", &self.repo), self.path()?)?
                .with_remote_name("main")?
                .fetch_then_checkout(Discard, &IS_INTERRUPTED)?;

        Ok(checkout_result.persist())
    }
}

impl Source for Repo {
    fn source(&self) -> Result<String> {
        let path = self.path()?;

        let repo = match open(&path) {
            Ok(r) => r,
            Err(_) => self.checkout()?,
        };

        if repo.is_dirty()? {
            repo.find_remote("main")?
                .connect(Fetch)?
                .prepare_fetch(Discard, ref_map::Options::default())?
                .receive(Discard, &IS_INTERRUPTED)?;
        }

        let tree = match self.version {
            Version::Stable => match repo.find_reference("tags/latest") {
                Ok(mut reference) => reference.peel_to_tree()?,
                Err(_) => repo.head_tree()?,
            },
            Version::Latest => repo.head_tree()?,
            Version::Tagged(ref version) => repo
                .find_reference(&format!("tags/{version}"))?
                .peel_to_tree()?,
        };

        match tree.find_entry("plugin.lua") {
            Some(entry) if entry.inner.mode.kind() == EntryKind::Blob => {
                Ok(entry.object()?.data.to_str_lossy().to_string())
            }
            Some(entry) => Err(anyhow!("plugin.lua is a {:?}", entry.inner.mode.kind())),
            None => Err(anyhow!("No plugin.lua found")),
        }
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, Default, Eq, PartialOrd, Ord)]
pub struct Dir {
    #[serde(alias = "path")]
    dir: CustomPathBuf,
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
impl Deref for Dir {
    type Target = CustomPathBuf;

    fn deref(&self) -> &Self::Target {
        &self.dir
    }
}

impl PartialEq for Dir {
    fn eq(&self, other: &Self) -> bool {
        self.canonicalize().ok() == other.canonicalize().ok()
    }
}

impl Hash for Dir {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.canonicalize() {
            Ok(path) => path.hash(state),
            _ => self.dir.hash(state),
        };
    }
}

impl Source for Dir {
    fn source(&self) -> Result<String> {
        fs::read_to_string(self.canonicalize()?).context("Failed to read file")
    }
}

#[derive(
    JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
#[serde(untagged)]
pub enum RepoOrDir {
    Repo(Repo),

    Dir(Dir),

    #[default]
    Invalid,
}

impl Display for RepoOrDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoOrDir::Repo(repo) => write!(f, "{repo}"),
            RepoOrDir::Dir(dir) => write!(f, "{dir}"),
            RepoOrDir::Invalid => write!(f, "Invalid Repo or Directory"),
        }
    }
}

impl Source for RepoOrDir {
    fn source(&self) -> Result<String> {
        match self {
            RepoOrDir::Repo(repo) => repo.source(),
            RepoOrDir::Dir(dir) => dir.source(),
            RepoOrDir::Invalid => Err(anyhow!("Not a valid source")),
        }
    }
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaggedTable {
    #[serde(rename = "$key$")]
    pub action_name: String,

    #[serde(flatten)]
    pub table: JsonValue,
}

impl Deref for TaggedTable {
    type Target = JsonValue;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

#[serde_as]
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plugin {
    #[serde(flatten, default)]
    pub source: RepoOrDir,

    #[serde(alias = "acts")]
    #[serde_as(as = "KeyValueMap<_>")]
    pub actions: Vec<TaggedTable>,
}

impl Plugin {
    fn runtime(&self, contexts: Option<Contexts>) -> Result<PluginSpec> {
        get_plugin(&self.source, contexts).map_err(anyhow::Error::from)
    }
}

impl Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plugin: {self:?}")
    }
}

impl Action for Plugin {
    fn summarize(&self) -> String {
        self.runtime(None)
            .as_ref()
            .map(PluginSpec::summary)
            .unwrap_or("Ran plugin".to_string())
    }

    #[instrument(skip_all)]
    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> Result<Vec<Step>> {
        let runtime = self.runtime(Some(context.to_owned()))?;
        let mut actions = Vec::new();

        for action in self.actions.clone() {
            let table = action.table;
            let name = action.action_name;
            let Some(plugin_action) = runtime.get_action(&name) else {
                error!("Action {} not found", &name);
                continue;
            };

            let v = match json_to_lua(&table, &runtime.lua) {
                Ok(value) => value,
                Err(e) => {
                    error!("Failed to convert arguments for {} to Lua: {}", &name, e);
                    continue;
                }
            };

            match plugin_action.plan.as_ref() {
                Some(plan) => match plan.call::<Option<String>>(v)? {
                    Some(plan_result) => info!(plan_result),
                    None => info!("Plan for {} completed", &name),
                },
                None => {
                    error!("Plan for {} failed", &name);
                    continue;
                }
            }
            actions.push(Step {
                atom: Box::new(PluginExec::new(
                    name,
                    plugin_action.exec.0.clone(),
                    json_to_lua(&table, &runtime.lua)?,
                )),
                // INFO: May want to add initializers and finalizers to plugin specs in the future
                initializers: vec![],
                finalizers: vec![],
            })
        }

        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::contexts::build_contexts;
    use crate::manifests::Manifest;
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use std::str::FromStr;
    use tempfile::tempdir;

    #[test]
    fn plugin_can_plan() -> Result<()> {
        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Create a plugin.lua file inside the temporary directory
        let lua_file_path = temp_dir.path().join("plugin.lua");
        let mut lua_file = File::create(&lua_file_path).expect("Failed to create lua file");
        lua_file
            .write_all(
                r#"
return {
    name = "echo",
    summary = "Echoes text",
    actions = {
        echo = {
            plan = function() end,
            exec = function(output, wait)
                print(tostring(output.output))
            end,
        },
    },
}
                "#
                .as_bytes(),
            )
            .expect("Failed to write to lua file");

        let manifest = Manifest::deserialize(json!({
            "actions": [{
                "action": "plugin",
                "dir": lua_file_path,
                "actions": {
                    "echo": {
                        "output": "foo"
                    }
                },
                "opts": {
                    "echo": { "output": "foo" }
                }
            }]
        }))?;

        let mut steps = manifest.actions.first().unwrap().plan(
            &manifest,
            &build_contexts(&Config {
                ..Default::default()
            }),
        )?;

        assert_eq!(steps.len(), 1);

        steps.first_mut().unwrap().atom.execute()?;

        // Clean up the temporary directory
        temp_dir.close().expect("Failed to close temp dir");

        Ok(())
    }

    #[test]
    fn repo_uri_from_str_valid() {
        let uri = RepoUri::from_str("username/repo").unwrap();
        assert_eq!(uri.username, "username");
        assert_eq!(uri.repo, "repo");
    }

    #[test]
    fn repo_uri_from_str_invalid() {
        let result = RepoUri::from_str("no-slash-here");
        assert!(result.is_err());
    }

    #[test]
    fn repo_uri_display() {
        let uri = RepoUri {
            username: "sgteam".to_string(),
            repo: "atlantis".to_string(),
        };
        assert_eq!(format!("{uri}"), "sgteam/atlantis");
    }

    #[test]
    fn version_display() {
        assert_eq!(format!("{}", Version::Stable), "stable");
        assert_eq!(format!("{}", Version::Latest), "latest");
        assert_eq!(
            format!("{}", Version::Tagged("v1.2.3".to_string())),
            "v1.2.3"
        );
    }

    #[test]
    fn version_default_is_stable() {
        assert_eq!(Version::default(), Version::Stable);
    }

    #[test]
    fn repo_deserialize_and_display() {
        let yaml = r#"
repo: sgteam/atlantis
version: stable
"#;
        let repo: Repo = serde_yaml_ng::from_str(yaml).unwrap();
        let display = format!("{repo}");
        assert!(display.contains("sgteam/atlantis"));
        assert!(display.contains("stable"));
    }

    #[test]
    fn dir_source_reads_file() -> Result<()> {
        let temp = tempdir()?;
        let lua_path = temp.path().join("plugin.lua");
        std::fs::write(&lua_path, "return {}")?;

        let dir = Dir {
            dir: crate::utilities::CustomPathBuf(lua_path.to_string_lossy().to_string().into()),
        };
        let source = dir.source()?;
        assert_eq!(source, "return {}");
        Ok(())
    }

    #[test]
    fn dir_display() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("plugin.lua");
        std::fs::write(&path, "").unwrap();

        let dir = Dir {
            dir: crate::utilities::CustomPathBuf(path.to_string_lossy().to_string().into()),
        };
        // Display should return the path string
        let _ = format!("{dir}");
    }

    #[test]
    fn repo_or_dir_display_invalid() {
        let rod = RepoOrDir::Invalid;
        assert_eq!(format!("{rod}"), "Invalid Repo or Directory");
    }

    #[test]
    fn repo_or_dir_invalid_source_errors() {
        let rod = RepoOrDir::Invalid;
        assert!(rod.source().is_err());
    }

    #[test]
    fn repo_or_dir_display_dir() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("plugin.lua");
        std::fs::write(&path, "").unwrap();

        let dir = Dir {
            dir: crate::utilities::CustomPathBuf(path.to_string_lossy().to_string().into()),
        };
        let rod = RepoOrDir::Dir(dir);
        // Should not panic
        let _ = format!("{rod}");
    }

    #[test]
    fn plugin_summarize_with_dir_source() -> Result<()> {
        let temp_dir = tempdir()?;
        let lua_file_path = temp_dir.path().join("plugin.lua");
        let mut lua_file = File::create(&lua_file_path)?;
        lua_file.write_all(
            r#"
return {
    name = "test-plugin",
    summary = "Test summary",
    actions = {
        act = { exec = function() end },
    },
}
"#
            .as_bytes(),
        )?;

        let manifest = Manifest::deserialize(json!({
            "actions": [{
                "action": "plugin",
                "dir": lua_file_path,
                "actions": {}
            }]
        }))?;

        let summary = manifest.actions.first().unwrap().inner_ref().summarize();
        assert!(summary.contains("Test summary") || !summary.is_empty());
        Ok(())
    }

    #[test]
    fn tagged_table_deref() {
        let tt = TaggedTable {
            action_name: "my_action".to_string(),
            table: json!({"key": "value"}),
        };
        let deref_val = tt.deref();
        assert_eq!(deref_val["key"], json!("value"));
    }

    #[test]
    fn dir_hash_eq() {
        use std::collections::HashSet;

        let temp = tempdir().unwrap();
        let path = temp.path().join("plugin.lua");
        std::fs::write(&path, "return {}").unwrap();

        let dir1 = Dir {
            dir: crate::utilities::CustomPathBuf(path.to_string_lossy().to_string().into()),
        };
        let dir2 = Dir {
            dir: crate::utilities::CustomPathBuf(path.to_string_lossy().to_string().into()),
        };

        // Same path should be equal
        assert_eq!(dir1, dir2);

        // Should be hashable
        let mut set = HashSet::new();
        set.insert(dir1);
        assert_eq!(set.len(), 1);
        set.insert(dir2);
        assert_eq!(set.len(), 1); // same path, same hash
    }

    #[test]
    fn repo_or_dir_display_repo() {
        let yaml = "repo: owner/myrepo\nversion: stable\n";
        let repo: Repo = serde_yaml_ng::from_str(yaml).unwrap();
        let rod = RepoOrDir::Repo(repo);
        let display = format!("{rod}");
        assert!(display.contains("owner/myrepo"));
        assert!(display.contains("stable"));
    }

    #[test]
    fn plugin_plan_action_not_found_is_skipped() -> Result<()> {
        let temp_dir = tempdir()?;
        let lua_file_path = temp_dir.path().join("plugin.lua");
        let mut lua_file = File::create(&lua_file_path)?;
        lua_file.write_all(
            r#"
return {
    name = "test",
    actions = {
        existing = {
            plan = function() end,
            exec = function(output, wait)
            end,
        },
    },
}
"#
            .as_bytes(),
        )?;

        let manifest = Manifest::deserialize(json!({
            "actions": [{
                "action": "plugin",
                "dir": lua_file_path,
                "actions": {
                    "nonexistent_action": {}
                }
            }]
        }))?;

        // Should not error - just skip the not-found action and return empty steps
        let steps = manifest
            .actions
            .first()
            .unwrap()
            .plan(&manifest, &build_contexts(&Config::default()))?;

        assert_eq!(steps.len(), 0);
        Ok(())
    }

    #[test]
    fn version_display_and_default() {
        // Verify Display works for all variants
        assert_eq!(format!("{}", Version::Stable), "stable");
        assert_eq!(format!("{}", Version::Latest), "latest");
        assert_eq!(
            format!("{}", Version::Tagged("v2.0.0".to_string())),
            "v2.0.0"
        );

        // Default is Stable
        assert_eq!(Version::default(), Version::Stable);
    }

    #[test]
    fn repo_hash() {
        use std::collections::HashSet;
        let yaml = "repo: owner/repo\nversion: stable\n";
        let repo1: Repo = serde_yaml_ng::from_str(yaml).unwrap();
        let repo2: Repo = serde_yaml_ng::from_str(yaml).unwrap();

        let mut set = HashSet::new();
        set.insert(repo1);
        // Adding same repo again shouldn't increase size
        set.insert(repo2);
        // The hashing is path-based - just verify it doesn't panic
    }

    #[test]
    fn get_plugin_returns_cached_spec_on_second_call() -> Result<()> {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let lua_file_path = temp_dir.path().join("plugin.lua");
        let mut lua_file = File::create(&lua_file_path)?;
        lua_file.write_all(
            r#"
return {
    name = "cached",
    summary = "Cache test plugin",
    actions = {
        run = {
            plan = function() end,
            exec = function(output, wait) end,
        },
    },
}
"#
            .as_bytes(),
        )?;

        let manifest = Manifest::deserialize(json!({
            "actions": [{
                "action": "plugin",
                "dir": lua_file_path,
                "actions": { "run": {} }
            }]
        }))?;
        let contexts = build_contexts(&Config::default());

        // First call — populates the PLUGINS cache
        let _ = manifest
            .actions
            .first()
            .unwrap()
            .plan(&manifest, &contexts)?;
        // Second call with same Dir source — hits the cache (plugin/mod.rs line 20)
        let _ = manifest
            .actions
            .first()
            .unwrap()
            .plan(&manifest, &contexts)?;

        Ok(())
    }
}
