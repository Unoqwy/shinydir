use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use anyhow::Context;
use cli::{Commands, CLI};
use config::Config;

mod automove;
mod checker;
mod cli;
mod commands;
mod config;
mod rules;

fn main() -> anyhow::Result<()> {
    let cli: CLI = CLI::parse();

    // Read config
    let config_path = match &cli.config {
        Some(path) => Some(path.clone()),
        _ => env::var("SHINYDIR_CONFIG_FILE").ok().map(PathBuf::from),
    };
    let config_path = match config_path {
        Some(path) => path,
        None => {
            let project = directories::ProjectDirs::from("", "", "Shiny Dir")
                .with_context(|| "unable to find config directory")?;
            let file_path = project.config_dir().join("shinydir.toml");
            if !file_path.try_exists().unwrap_or(true) {
                eprintln!("Copying default configuration because no config file was found...");
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = File::create(&file_path)?;
                let default_config = include_str!("../shinydir.toml").as_bytes();
                file.write_all(default_config)?;
                file.flush()?;
                eprintln!("");
            }
            file_path
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
