mod apply;
pub(crate) use apply::Apply;

pub(crate) mod status;

mod version;
pub(crate) use version::Version;

mod contexts;
pub(crate) use contexts::Contexts;

mod gen_completions;
pub(crate) use gen_completions::GenCompletions;

mod update;
pub(crate) use update::Update;

use crate::Runtime;

pub trait EtchCommand {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}
