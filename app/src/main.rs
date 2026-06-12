use crate::commands::EtchCommand;
use crate::config::{Commands, GlobalArgs};

use etch_lib::contexts::build_contexts;
use etch_lib::contexts::Contexts;
use etch_lib::manifests;

use clap::Parser;
use tracing::error;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Layer, Registry};

mod commands;
mod config;
use config::Config;

#[derive(Debug)]
pub struct Runtime {
    pub(crate) args: GlobalArgs,
    pub(crate) config: Config,
    pub(crate) contexts: Contexts,
}

pub(crate) fn execute(runtime: Runtime) -> anyhow::Result<()> {
    match &runtime.args.command {
        Commands::Apply(apply) => apply.execute(&runtime),
        Commands::Status(apply) => apply.status(&runtime),
        Commands::Version(version) => version.execute(&runtime),
        Commands::Contexts(contexts) => contexts.execute(&runtime),
        Commands::GenCompletions(gen_completions) => gen_completions.execute(&runtime),
        Commands::Update(update) => update.execute(&runtime),
        Commands::Plugin(plugin) => plugin.execute(&runtime),
        Commands::HelpAll(h) => h.execute(&runtime),
        Commands::Doctor(d) => d.execute(&runtime),
        Commands::History(h) => h.execute(&runtime),
    }
}

fn configure_tracing(args: &GlobalArgs) {
    let stdout_level = match args.verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    // Per-layer filter on fmt keeps stdout at the requested verbosity without
    // propagating that filter globally. journald receives all levels (TRACE+)
    // independently via PLF (per-layer filtering) in tracing-subscriber 0.3.
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(!args.no_color)
        .with_target(false)
        .without_time()
        .with_filter(stdout_level);

    #[cfg(target_os = "linux")]
    if let Ok(journald_layer) = tracing_journald::layer() {
        tracing::subscriber::set_global_default(
            Registry::default().with(fmt_layer).with(journald_layer),
        )
        .expect("Unable to set a global subscriber");
        return;
    }

    tracing::subscriber::set_global_default(Registry::default().with(fmt_layer))
        .expect("Unable to set a global subscriber");
}

fn main() -> anyhow::Result<()> {
    let args = GlobalArgs::parse();
    configure_tracing(&args);

    let config = match config::load_config(&args) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", error.to_string());
            panic!();
        }
    };

    if !config.disable_update_check {
        check_for_updates(args.no_color);
    }

    // Run Context Providers
    let contexts = build_contexts(&config);
    let runtime = Runtime {
        args,
        config,
        contexts,
    };

    execute(runtime)?;

    Ok(())
}

fn check_for_updates(no_color: bool) {
    use colored::*;
    use update_informer::{registry, Check};

    if no_color {
        control::set_override(false);
    }

    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::Crates, pkg_name, pkg_version);

    if let Some(new_version) = informer.check_version().ok().flatten() {
        let msg = format!(
            "A new version of {pkg_name} is available: v{pkg_version} -> {new_version}",
            pkg_name = pkg_name.italic().cyan(),
            new_version = new_version.to_string().green()
        );

        let release_url =
            format!("https://github.com/{pkg_name}/{pkg_name}/releases/tag/{new_version}").blue();
        let changelog = format!("Changelog: {release_url}",);

        let cmd = format!(
            "Run to update: {cmd}",
            cmd = "curl -fsSL https://get.etch-cli.dev | sh".green()
        );

        println!("\n{msg}\n{changelog}\n{cmd}");
    }
}
