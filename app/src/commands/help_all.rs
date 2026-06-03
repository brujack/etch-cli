use super::EtchCommand;
use crate::config::GlobalArgs;
use crate::Runtime;

use clap::{CommandFactory, Parser};

#[derive(Parser, Debug)]
#[command()]
pub(crate) struct HelpAll {}

impl EtchCommand for HelpAll {
    fn execute(&self, _: &Runtime) -> anyhow::Result<()> {
        let cmd = GlobalArgs::command();
        let skip = ["help", "help-all"];
        let names: Vec<String> = cmd
            .get_subcommands()
            .filter(|s| !skip.contains(&s.get_name()))
            .map(|s| s.get_name().to_string())
            .collect();

        for name in names {
            let mut app = GlobalArgs::command().bin_name("etch");
            let subcmd = app.find_subcommand_mut(&name).unwrap();
            println!("=== {} ===", name);
            subcmd.print_help()?;
            println!("\n");
        }
        Ok(())
    }
}
