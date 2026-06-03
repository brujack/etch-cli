mod binary;
mod brew;
mod command;
mod directory;
mod file;
pub use file::link::FileLink;
pub use file::FileAction;
mod gem;
mod git;
mod group;
mod macos;
mod mas;
mod npm;
mod package;
mod pip;
mod plugin;
mod pyenv;
mod ruby;
mod systemd;
mod user;

use crate::actions::macos::{MacOSDefault, MacOSRosetta, MacOSService};
use crate::actions::mas::{MasInstall, MasUpgrade};
use crate::actions::systemd::SystemdService;
use crate::{contexts::Contexts, manifests::Manifest, steps::Step};
use anyhow::anyhow;
use binary::{BinaryGitHub, BinaryUrl};
use brew::{BrewBundle, BrewCleanup, BrewUpgrade};
use command::run::RunCommand;
use directory::{DirectoryCopy, DirectoryCreate, DirectoryRemove};
use file::chmod::FileChmod;
use file::chown::FileChown;
use file::copy::FileCopy;
use file::download::FileDownload;
use file::flags::FileFlags;
use file::remove::FileRemove;
use file::unarchive::FileUnarchive;
use gem::GemInstall;
use git::{GitClone, GitConfig, GitPull};
use group::add::GroupAdd;
use npm::NpmInstall;
use package::{PackageAutoremove, PackageInstall, PackageRepository, PackageUpgrade};
use pip::PipInstall;
use plugin::Plugin;
use pyenv::{PyenvInstall, PyenvVirtualenv};
use rhai::Engine;
use ruby::RubyInstall;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::Deref;
use tracing::{error, warn};
use user::add::UserAdd;

use self::user::add_group::UserAddGroup;

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConditionalVariantAction<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "where")]
    pub condition: Option<String>,

    #[serde(default)]
    pub variants: Vec<Variant<T>>,

    #[serde(default)]
    pub notify: Vec<String>,
}

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "where")]
    pub condition: Option<String>,
}

impl<T> Variant<T> {
    pub fn new(action: T, condition: Option<String>) -> Self {
        Self { action, condition }
    }
}

impl<T> Action for ConditionalVariantAction<T>
where
    T: Action,
{
    fn summarize(&self) -> String {
        self.action.summarize()
    }

    fn plan(&self, manifest: &Manifest, context: &Contexts) -> Result<Vec<Step>, anyhow::Error> {
        let engine = Engine::new();
        let mut scope = crate::contexts::to_rhai(context);

        let variant = self.variants.iter().find(|variant| {
            if variant.condition.is_none() {
                return false;
            }

            // .unwrap() is safe here because we checked for None above
            let condition = variant.condition.clone().unwrap();
            match engine.eval_with_scope::<bool>(&mut scope, condition.as_str()) {
                Ok(b) => b,
                Err(error) => {
                    error!("Failed execution condition for action: {}", error);
                    false
                }
            }
        });

        if let Some(variant) = variant {
            return variant.action.plan(manifest, context);
        }

        if self.condition.is_none() {
            return self.action.plan(manifest, context);
        }

        // .unwrap() is safe here because we checked for None above
        let condition = self.condition.as_ref().unwrap();

        match engine.eval_with_scope::<bool>(&mut scope, condition.as_str()) {
            Ok(true) => self.action.plan(manifest, context),
            Ok(false) => Ok(vec![]),
            Err(error) => Err(anyhow!("Failed execution condition for action: {error}")),
        }
    }
}

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(rename = "command.run", alias = "cmd.run")]
    CommandRun(ConditionalVariantAction<RunCommand>),

    #[serde(rename = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(ConditionalVariantAction<DirectoryCopy>),

    #[serde(rename = "directory.create", alias = "dir.create")]
    DirectoryCreate(ConditionalVariantAction<DirectoryCreate>),

    #[serde(rename = "file.chmod")]
    FileChmod(ConditionalVariantAction<FileChmod>),

    #[serde(rename = "file.flags")]
    FileFlags(ConditionalVariantAction<FileFlags>),

    #[serde(rename = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    #[serde(rename = "file.chown")]
    FileChown(ConditionalVariantAction<FileChown>),

    #[serde(rename = "file.download")]
    FileDownload(ConditionalVariantAction<FileDownload>),

    #[serde(rename = "file.link")]
    FileLink(ConditionalVariantAction<FileLink>),

    #[serde(rename = "file.remove")]
    FileRemove(ConditionalVariantAction<FileRemove>),

    #[serde(rename = "file.unarchive")]
    FileUnarchive(ConditionalVariantAction<FileUnarchive>),

    #[serde(rename = "directory.remove", alias = "dir.remove")]
    DirectoryRemove(ConditionalVariantAction<DirectoryRemove>),

    #[serde(
        rename = "binary.github",
        alias = "binary.gh",
        alias = "bin.github",
        alias = "bin.gh"
    )]
    BinaryGitHub(ConditionalVariantAction<BinaryGitHub>),

    #[serde(rename = "binary.url", alias = "bin.url")]
    BinaryUrl(ConditionalVariantAction<BinaryUrl>),

    #[serde(rename = "brew.bundle")]
    BrewBundle(ConditionalVariantAction<BrewBundle>),

    #[serde(rename = "brew.cleanup")]
    BrewCleanup(ConditionalVariantAction<BrewCleanup>),

    #[serde(rename = "brew.upgrade")]
    BrewUpgrade(ConditionalVariantAction<BrewUpgrade>),

    #[serde(rename = "mas.upgrade")]
    MasUpgrade(ConditionalVariantAction<MasUpgrade>),

    #[serde(rename = "git.clone")]
    GitClone(ConditionalVariantAction<GitClone>),

    #[serde(rename = "git.config", alias = "git.cfg")]
    GitConfig(ConditionalVariantAction<GitConfig>),

    #[serde(rename = "git.pull")]
    GitPull(ConditionalVariantAction<GitPull>),

    #[serde(rename = "group.add")]
    GroupAdd(ConditionalVariantAction<GroupAdd>),

    #[serde(rename = "macos.default")]
    MacOSDefault(ConditionalVariantAction<MacOSDefault>),

    #[serde(rename = "macos.rosetta")]
    MacOSRosetta(ConditionalVariantAction<MacOSRosetta>),

    #[serde(rename = "macos.service")]
    MacOSService(ConditionalVariantAction<MacOSService>),

    #[serde(rename = "systemd.service")]
    SystemdService(ConditionalVariantAction<SystemdService>),

    #[serde(rename = "mas.install")]
    MasInstall(ConditionalVariantAction<MasInstall>),

    #[serde(rename = "package.install", alias = "package.installed")]
    PackageInstall(ConditionalVariantAction<PackageInstall>),

    #[serde(rename = "package.repository", alias = "package.repo")]
    PackageRepository(ConditionalVariantAction<PackageRepository>),

    #[serde(rename = "package.autoremove")]
    PackageAutoremove(ConditionalVariantAction<PackageAutoremove>),

    #[serde(rename = "package.upgrade")]
    PackageUpgrade(ConditionalVariantAction<PackageUpgrade>),

    #[serde(rename = "user.add")]
    UserAdd(ConditionalVariantAction<UserAdd>),

    #[serde(rename = "user.group")]
    UserAddGroup(ConditionalVariantAction<UserAddGroup>),

    #[serde(rename = "plugin")]
    Plugin(ConditionalVariantAction<Plugin>),

    #[serde(rename = "ruby.install")]
    RubyInstall(ConditionalVariantAction<RubyInstall>),

    #[serde(rename = "gem.install")]
    GemInstall(ConditionalVariantAction<GemInstall>),

    #[serde(rename = "pip.install")]
    PipInstall(ConditionalVariantAction<PipInstall>),

    #[serde(rename = "npm.install")]
    NpmInstall(ConditionalVariantAction<NpmInstall>),

    #[serde(rename = "pyenv.install")]
    PyenvInstall(ConditionalVariantAction<PyenvInstall>),

    #[serde(rename = "pyenv.virtualenv")]
    PyenvVirtualenv(ConditionalVariantAction<PyenvVirtualenv>),
}

impl Actions {
    pub fn inner_ref(&self) -> &dyn Action {
        match self {
            Actions::BinaryGitHub(a) => a,
            Actions::BinaryUrl(a) => a,
            Actions::BrewBundle(a) => a,
            Actions::BrewCleanup(a) => a,
            Actions::BrewUpgrade(a) => a,
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::DirectoryCreate(a) => a,
            Actions::FileChmod(a) => a,
            Actions::FileFlags(a) => a,
            Actions::FileCopy(a) => a,
            Actions::FileChown(a) => a,
            Actions::FileDownload(a) => a,
            Actions::FileLink(a) => a,
            Actions::FileUnarchive(a) => a,
            Actions::GitClone(a) => a,
            Actions::GitConfig(a) => a,
            Actions::GitPull(a) => a,
            Actions::GroupAdd(a) => a,
            Actions::MacOSDefault(a) => a,
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
            Actions::SystemdService(a) => a,
            Actions::MasInstall(a) => a,
            Actions::MasUpgrade(a) => a,
            Actions::PackageInstall(a) => a,
            Actions::PackageRepository(a) => a,
            Actions::PackageAutoremove(a) => a,
            Actions::PackageUpgrade(a) => a,
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::FileRemove(a) => a,
            Actions::DirectoryRemove(a) => a,
            Actions::Plugin(a) => a,
            Actions::RubyInstall(a) => a,
            Actions::GemInstall(a) => a,
            Actions::PipInstall(a) => a,
            Actions::NpmInstall(a) => a,
            Actions::PyenvInstall(a) => a,
            Actions::PyenvVirtualenv(a) => a,
        }
    }

    pub fn notify(&self) -> &[String] {
        match self {
            Actions::BinaryGitHub(a) => &a.notify,
            Actions::BinaryUrl(a) => &a.notify,
            Actions::BrewBundle(a) => &a.notify,
            Actions::BrewCleanup(a) => &a.notify,
            Actions::BrewUpgrade(a) => &a.notify,
            Actions::CommandRun(a) => &a.notify,
            Actions::DirectoryCopy(a) => &a.notify,
            Actions::DirectoryCreate(a) => &a.notify,
            Actions::DirectoryRemove(a) => &a.notify,
            Actions::FileChmod(a) => &a.notify,
            Actions::FileFlags(a) => &a.notify,
            Actions::FileCopy(a) => &a.notify,
            Actions::FileChown(a) => &a.notify,
            Actions::FileDownload(a) => &a.notify,
            Actions::FileLink(a) => &a.notify,
            Actions::FileRemove(a) => &a.notify,
            Actions::FileUnarchive(a) => &a.notify,
            Actions::GitClone(a) => &a.notify,
            Actions::GitConfig(a) => &a.notify,
            Actions::GitPull(a) => &a.notify,
            Actions::GroupAdd(a) => &a.notify,
            Actions::MacOSDefault(a) => &a.notify,
            Actions::MacOSRosetta(a) => &a.notify,
            Actions::MacOSService(a) => &a.notify,
            Actions::SystemdService(a) => &a.notify,
            Actions::MasInstall(a) => &a.notify,
            Actions::MasUpgrade(a) => &a.notify,
            Actions::PackageInstall(a) => &a.notify,
            Actions::PackageRepository(a) => &a.notify,
            Actions::PackageAutoremove(a) => &a.notify,
            Actions::PackageUpgrade(a) => &a.notify,
            Actions::UserAdd(a) => &a.notify,
            Actions::UserAddGroup(a) => &a.notify,
            Actions::Plugin(a) => &a.notify,
            Actions::RubyInstall(a) => &a.notify,
            Actions::GemInstall(a) => &a.notify,
            Actions::PipInstall(a) => &a.notify,
            Actions::NpmInstall(a) => &a.notify,
            Actions::PyenvInstall(a) => &a.notify,
            Actions::PyenvVirtualenv(a) => &a.notify,
        }
    }
}

impl Deref for Actions {
    type Target = dyn Action;
    fn deref(&self) -> &Self::Target {
        match self {
            Actions::BinaryGitHub(a) => a,
            Actions::BinaryUrl(a) => a,
            Actions::BrewBundle(a) => a,
            Actions::BrewCleanup(a) => a,
            Actions::BrewUpgrade(a) => a,
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::DirectoryCreate(a) => a,
            Actions::FileChmod(a) => a,
            Actions::FileFlags(a) => a,
            Actions::FileCopy(a) => a,
            Actions::FileChown(a) => a,
            Actions::FileDownload(a) => a,
            Actions::FileLink(a) => a,
            Actions::FileUnarchive(a) => a,
            Actions::GitClone(a) => a,
            Actions::GitConfig(a) => a,
            Actions::GitPull(a) => a,
            Actions::GroupAdd(a) => a,
            Actions::MacOSDefault(a) => a,
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
            Actions::SystemdService(a) => a,
            Actions::MasInstall(a) => a,
            Actions::MasUpgrade(a) => a,
            Actions::PackageInstall(a) => a,
            Actions::PackageRepository(a) => a,
            Actions::PackageAutoremove(a) => a,
            Actions::PackageUpgrade(a) => a,
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::FileRemove(a) => a,
            Actions::DirectoryRemove(a) => a,
            Actions::Plugin(a) => a,
            Actions::RubyInstall(a) => a,
            Actions::GemInstall(a) => a,
            Actions::PipInstall(a) => a,
            Actions::NpmInstall(a) => a,
            Actions::PyenvInstall(a) => a,
            Actions::PyenvVirtualenv(a) => a,
        }
    }
}

impl Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Actions::CommandRun(_) => "command.run",
            Actions::DirectoryCopy(_) => "directory.copy",
            Actions::DirectoryCreate(_) => "directory.create",
            Actions::FileChmod(_) => "file.chmod",
            Actions::FileFlags(_) => "file.flags",
            Actions::FileCopy(_) => "file.copy",
            Actions::FileChown(_) => "file.chown",
            Actions::FileDownload(_) => "file.download",
            Actions::FileLink(_) => "file.link",
            Actions::FileRemove(_) => "file.remove",
            Actions::FileUnarchive(_) => "file.unarchive",
            Actions::DirectoryRemove(_) => "directory.remove",
            Actions::BinaryGitHub(_) => "github.binary",
            Actions::BinaryUrl(_) => "url.binary",
            Actions::BrewBundle(_) => "brew.bundle",
            Actions::BrewCleanup(_) => "brew.cleanup",
            Actions::BrewUpgrade(_) => "brew.upgrade",
            Actions::GitClone(_) => "git.clone",
            Actions::GitConfig(_) => "git.config",
            Actions::GitPull(_) => "git.pull",
            Actions::GroupAdd(_) => "group.add",
            Actions::MacOSDefault(_) => "macos.default",
            Actions::MacOSRosetta(_) => "macos.rosetta",
            Actions::MacOSService(_) => "macos.service",
            Actions::SystemdService(_) => "systemd.service",
            Actions::MasInstall(_) => "mas.install",
            Actions::MasUpgrade(_) => "mas.upgrade",
            Actions::PackageInstall(_) => "package.install",
            Actions::PackageRepository(_) => "package.repository",
            Actions::PackageAutoremove(_) => "package.autoremove",
            Actions::PackageUpgrade(_) => "package.upgrade",
            Actions::UserAdd(_) => "user.add",
            Actions::UserAddGroup(_) => "user.group",
            Actions::Plugin(_) => "plugin",
            Actions::RubyInstall(_) => "ruby.install",
            Actions::GemInstall(_) => "gem.install",
            Actions::PipInstall(_) => "pip.install",
            Actions::NpmInstall(_) => "npm.install",
            Actions::PyenvInstall(_) => "pyenv.install",
            Actions::PyenvVirtualenv(_) => "pyenv.virtualenv",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// Output / response
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionError {
    /// Error message
    pub message: String,
}

impl<E: std::error::Error> From<E> for ActionError {
    fn from(e: E) -> Self {
        ActionError {
            message: format!("{e}"),
        }
    }
}

pub trait Action {
    fn summarize(&self) -> String {
        warn!("need to define action summarize");
        "not found action summarize".to_string()
    }
    fn plan(&self, manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>>;
}

#[cfg(test)]
mod tests {
    use crate::actions::{command::run::RunCommand, Actions};
    use crate::manifests::Manifest;

    #[test]
    fn can_parse_some_advanced_stuff() {
        let content = r#"
actions:
- action: command.run
  command: echo
  args:
    - hi
  variants:
    - where: Debian
      command: halt
"#;
        let m: Manifest = serde_yaml_ng::from_str(content).unwrap();

        let action = &m.actions[0];

        let ext = match action {
            Actions::CommandRun(cr) => cr,
            _ => panic!("did not get a command to run"),
        };

        assert_eq!(
            ext.action,
            RunCommand {
                command: "echo".into(),
                args: vec!["hi".into()],
                privileged: false,
                dir: std::env::current_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
                ..Default::default()
            }
        );

        let variant = &ext.variants[0];
        assert_eq!(variant.condition, Some(String::from("Debian")));
        assert_eq!(variant.action.command, "halt");
    }

    #[test]
    fn notify_deserializes_from_yaml() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    notify: [restart-dock, reload-nginx]
"#;
        let m: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &m.actions[0] {
            Actions::CommandRun(a) => a,
            _ => panic!("expected CommandRun"),
        };
        assert_eq!(action.notify, vec!["restart-dock", "reload-nginx"]);
    }

    #[test]
    fn notify_defaults_empty() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
"#;
        let m: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &m.actions[0] {
            Actions::CommandRun(a) => a,
            _ => panic!("expected CommandRun"),
        };
        assert!(action.notify.is_empty());
    }

    #[test]
    fn all_major_action_variants_can_be_deserialized() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
  - action: directory.copy
    from: a
    to: b
  - action: directory.create
    path: /tmp/d
  - action: directory.remove
    target: /tmp/d
  - action: file.copy
    from: a
    to: b
  - action: file.chown
    path: /tmp/f
  - action: file.chmod
    path: /tmp/f
    mode: "700"
  - action: file.link
    source: a
    target: b
  - action: file.remove
    target: /tmp/f
  - action: file.unarchive
    from: a.tar.gz
    to: /tmp/dest
  - action: git.clone
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: git.pull
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: group.add
    group_name: mygroup
  - action: macos.default
    domain: com.example
    key: k
    kind: string
    value: v
  - action: mas.install
    name: "Better Rename 9"
    id: 414209656
  - action: package.install
    name: htop
  - action: user.add
    username: alice
  - action: brew.bundle
    file: /tmp/Brewfile
  - action: brew.upgrade
  - action: brew.cleanup
  - action: mas.upgrade
  - action: git.config
    scope: global
    key: user.email
    value: test@example.com
  - action: systemd.service
    unit: sshd.service
    enabled: true
  - action: file.flags
    path: /tmp/f
    flags: [hidden]
  - action: gem.install
    name: bundler
  - action: pip.install
    name: requests
  - action: package.upgrade
    provider: apt
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(27, manifest.actions.len());
    }

    #[test]
    fn actions_display_names() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
  - action: directory.copy
    from: a
    to: b
  - action: directory.create
    path: /tmp/d
  - action: directory.remove
    target: /tmp/d
  - action: file.copy
    from: a
    to: b
  - action: file.chown
    path: /tmp/f
  - action: file.chmod
    path: /tmp/f
    mode: "700"
  - action: file.flags
    path: /tmp/f
    flags: [hidden]
  - action: file.download
    from: https://example.com/file.txt
    to: /tmp/file.txt
  - action: file.link
    source: a
    target: b
  - action: file.remove
    target: /tmp/f
  - action: file.unarchive
    from: a.tar.gz
    to: /tmp/dest
  - action: git.clone
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: git.pull
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: group.add
    group_name: mygroup
  - action: macos.default
    domain: com.example
    key: k
    kind: string
    value: v
  - action: mas.install
    name: "Better Rename 9"
    id: 414209656
  - action: package.install
    name: htop
  - action: package.repository
    name: myrepo
  - action: user.add
    username: alice
  - action: user.group
    username: alice
    group_name: staff
  - action: brew.bundle
    file: /tmp/Brewfile
  - action: brew.upgrade
  - action: brew.cleanup
  - action: mas.upgrade
  - action: systemd.service
    unit: sshd.service
    enabled: true
  - action: gem.install
    name: bundler
  - action: pip.install
    name: requests
  - action: package.autoremove
  - action: npm.install
    name: typescript
  - action: pyenv.install
    version: "3.12.0"
  - action: pyenv.virtualenv
    python_version: "3.12.0"
    name: myproject
  - action: package.upgrade
    provider: snap
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();

        let expected_names = [
            "command.run",
            "directory.copy",
            "directory.create",
            "directory.remove",
            "file.copy",
            "file.chown",
            "file.chmod",
            "file.flags",
            "file.download",
            "file.link",
            "file.remove",
            "file.unarchive",
            "git.clone",
            "git.pull",
            "group.add",
            "macos.default",
            "mas.install",
            "package.install",
            "package.repository",
            "user.add",
            "user.group",
            "brew.bundle",
            "brew.upgrade",
            "brew.cleanup",
            "mas.upgrade",
            "systemd.service",
            "gem.install",
            "pip.install",
            "package.autoremove",
            "npm.install",
            "pyenv.install",
            "pyenv.virtualenv",
            "package.upgrade",
        ];

        for (action, expected) in manifest.actions.iter().zip(expected_names.iter()) {
            assert_eq!(format!("{action}"), *expected);
        }
    }

    #[test]
    fn actions_inner_ref_and_deref_summarize() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = &manifest.actions[0];

        // Test inner_ref() returns a dyn Action
        let inner = action.inner_ref();
        let _ = inner.summarize();

        // Test Deref to dyn Action
        use std::ops::Deref;
        let deref_action = action.deref();
        let _ = deref_action.summarize();
    }

    #[test]
    fn conditional_variant_action_with_no_condition_plans() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    args: [hello]
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn conditional_variant_action_with_false_condition_returns_empty() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    where: "false"
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 0);
    }

    #[test]
    fn conditional_variant_action_with_true_condition_plans() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    where: "true"
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn conditional_variant_action_with_invalid_condition_errors() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    where: "not valid rhai !!!"
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        assert!(manifest.actions[0].plan(&manifest, &contexts).is_err());
    }

    #[test]
    fn variant_new_constructor() {
        use crate::actions::command::run::RunCommand;
        use crate::actions::Variant;
        let action = RunCommand::default();
        let v = Variant::new(action.clone(), Some("true".to_string()));
        assert_eq!(v.condition, Some("true".to_string()));

        let v2 = Variant::new(action, None);
        assert_eq!(v2.condition, None);
    }

    #[test]
    fn binary_github_action_display() {
        let yaml = r#"
actions:
  - action: binary.github
    name: ripgrep
    directory: /usr/local/bin
    repository: BurntSushi/ripgrep
    version: "13.0.0"
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(format!("{}", manifest.actions[0]), "github.binary");
    }

    #[test]
    fn plugin_action_display() {
        let yaml = r#"
actions:
  - action: plugin
    dir: /tmp/fake
    actions:
      myaction:
        key: val
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(format!("{}", manifest.actions[0]), "plugin");
    }

    #[test]
    fn action_error_from_std_error() {
        use crate::actions::ActionError;
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ae = ActionError::from(err);
        assert!(ae.message.contains("file not found"));
    }

    #[test]
    fn all_action_variants_inner_ref_and_deref() {
        use std::ops::Deref;

        // Parse all action types to exercise inner_ref() and Deref for each variant
        let yaml = r#"
actions:
  - action: command.run
    command: echo
  - action: directory.copy
    from: a
    to: b
  - action: directory.create
    path: /tmp/d
  - action: directory.remove
    target: /tmp/d
  - action: file.chmod
    path: /tmp/f
    mode: "700"
  - action: file.flags
    path: /tmp/f
    flags: [hidden]
  - action: file.copy
    from: a
    to: b
  - action: file.chown
    path: /tmp/f
  - action: file.download
    from: https://example.com/file
    to: /tmp/file
  - action: file.link
    source: a
    target: b
  - action: file.remove
    target: /tmp/f
  - action: file.unarchive
    from: a.tar.gz
    to: /tmp/dest
  - action: binary.github
    name: tool
    directory: /usr/local/bin
    repository: owner/repo
  - action: git.clone
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: git.pull
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: group.add
    group_name: mygroup
  - action: macos.default
    domain: com.example
    key: k
    kind: string
    value: v
  - action: mas.install
    name: "Better Rename 9"
    id: 414209656
  - action: package.install
    name: htop
  - action: package.repository
    name: myrepo
  - action: user.add
    username: alice
  - action: user.group
    username: alice
    group_name: staff
  - action: brew.bundle
    file: /tmp/Brewfile
  - action: brew.upgrade
  - action: brew.cleanup
  - action: mas.upgrade
  - action: git.config
    scope: global
    key: user.email
    value: test@example.com
  - action: gem.install
    name: bundler
  - action: pip.install
    name: requests
  - action: package.autoremove
  - action: npm.install
    name: typescript
  - action: pyenv.install
    version: "3.12.0"
  - action: pyenv.virtualenv
    python_version: "3.12.0"
    name: myproject
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(33, manifest.actions.len());

        for action in &manifest.actions {
            // Exercise inner_ref() for every variant
            let inner = action.inner_ref();
            let _ = inner.summarize();

            // Exercise Deref for every variant
            let deref_action = action.deref();
            let _ = deref_action.summarize();
        }
    }

    #[test]
    fn variant_condition_skips_when_false() {
        // When variant condition is false, that variant is not selected
        let yaml = r#"
actions:
- action: command.run
  command: echo
  args: [fallback]
  variants:
    - where: "false"
      command: halt
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        // Should not error - false variant condition means fall through to main action
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn variant_condition_matches_when_true() {
        let yaml = r#"
actions:
- action: command.run
  command: echo
  args: [main]
  variants:
    - where: "true"
      command: echo
      args: [variant]
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        // The true variant should be selected
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn variant_error_in_condition_is_logged_not_panic() {
        // When variant condition evaluation errors (not a bool), it logs and skips
        let yaml = r#"
actions:
- action: command.run
  command: echo
  variants:
    - where: "1 + 1"
      command: halt
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        // Non-bool expression logs error and returns false (skip variant)
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn variant_with_no_condition_is_skipped_in_find() {
        // A variant with no "where" clause has condition=None; the find() closure
        // returns false immediately (line 79), skipping it. The base action runs.
        let yaml = r#"
actions:
- action: command.run
  command: echo
  args: [base]
  variants:
    - command: echo
      args: [variant-no-condition]
"#;
        use crate::config::Config;
        use crate::contexts::build_contexts;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let contexts = build_contexts(&Config::default());
        let steps = manifest.actions[0].plan(&manifest, &contexts).unwrap();
        // No variant matched, base action runs (1 step)
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn git_config_alias_deserializes() {
        let yaml = r#"
actions:
  - action: git.cfg
    scope: global
    key: user.name
    value: Bruce
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(1, manifest.actions.len());
        assert!(matches!(manifest.actions[0], Actions::GitConfig(_)));
    }

    #[test]
    fn git_config_display_name() {
        let yaml = r#"
actions:
  - action: git.config
    scope: global
    key: user.email
    value: foo@bar.com
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(format!("{}", manifest.actions[0]), "git.config");
    }

    #[test]
    fn git_pull_deserializes() {
        let yaml = r#"
actions:
  - action: git.pull
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(1, manifest.actions.len());
        assert!(matches!(manifest.actions[0], Actions::GitPull(_)));
    }

    #[test]
    fn actions_notify_method_returns_field() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
    notify: [restart-dock]
"#;
        let m: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.actions[0].notify(), &["restart-dock"]);
    }

    #[test]
    fn all_remaining_action_variants_notify_returns_slice() {
        // Covers notify() arms for variants not exercised elsewhere:
        // BinaryUrl, MacOSRosetta, MacOSService, SystemdService,
        // PackageUpgrade, RubyInstall, Plugin
        let yaml = r#"
actions:
  - action: binary.url
    name: tool
    directory: /usr/local/bin
    url: https://example.com/tool.tar.gz
  - action: macos.rosetta
  - action: macos.service
    plist: /Library/LaunchAgents/com.example.plist
    state: loaded
  - action: systemd.service
    unit: sshd.service
    enabled: true
  - action: package.upgrade
    provider: apt
  - action: ruby.install
    version: "3.2.0"
  - action: plugin
    dir: /tmp/fake
    actions:
      myaction:
        key: val
"#;
        let m: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(7, m.actions.len());
        for action in &m.actions {
            // Each notify() call exercises a match arm; all should return empty slice
            assert!(action.notify().is_empty());
        }
    }

    #[test]
    fn conditional_variant_action_false_condition_direct() {
        // Direct call on ConditionalVariantAction (not via Actions::Deref vtable) so
        // tarpaulin instruments the Ok(false) => Ok(vec![]) branch in plan().
        use super::Action;
        use crate::actions::command::run::RunCommand;
        use crate::config::Config;
        use crate::contexts::build_contexts;

        let action = super::ConditionalVariantAction::<RunCommand> {
            action: RunCommand {
                command: "echo".into(),
                ..Default::default()
            },
            condition: Some("false".to_string()),
            variants: vec![],
            notify: vec![],
        };
        let manifest = crate::manifests::Manifest::default();
        let contexts = build_contexts(&Config::default());
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 0);
    }
}
