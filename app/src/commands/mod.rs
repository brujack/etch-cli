mod apply;
pub(crate) use apply::Apply;

pub(crate) mod status;

mod help_all;
pub(crate) use help_all::HelpAll;

mod version;
pub(crate) use version::Version;

mod contexts;
pub(crate) use contexts::Contexts;

mod gen_completions;
pub(crate) use gen_completions::GenCompletions;

mod plugin;
pub(crate) use plugin::PluginCommands;

mod update;
pub(crate) use update::Update;

mod doctor;
pub(crate) use doctor::Doctor;

mod history;
pub(crate) use history::History;

use crate::Runtime;

pub trait EtchCommand {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}
