use std::fs;
use std::path::PathBuf;

use clap::Parser;

use cli::{Commands, CLI};
use config::Config;

mod automove;
mod checker;
mod cli;
mod commands;
mod config;

fn main() -> anyhow::Result<()> {
    let cli = CLI::parse();

    // Read config
    let config_path = match cli.config {
        Some(path) => path,
        None => {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("shinydir")?;
            xdg_dirs.get_config_file("shinydir.toml")
        }
    };
    let config_contents = fs::read_to_string(&config_path)
        .map_err(|err| anyhow::format_err!("Could not read config file: {}", err))?;
    let config_dir = config_path
        .parent()
        .map_or(PathBuf::new(), |file| file.to_path_buf());
    let config: Config = toml::from_str(&config_contents)?;

    // Run command
    match cli.command {
        Commands::Check { target, list } => {
            crate::commands::check::execute(&config, config_dir, target, list)
        }
        Commands::AutoMove {
            target,
            list,
            dry_run,
        } => crate::commands::automove::execute(&config, config_dir, target, list, dry_run),
    }?;

    Ok(())
}
