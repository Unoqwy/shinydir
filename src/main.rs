#![deny(clippy::pedantic)]

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use anyhow::Context;
use cli::{Cli, Commands};
use config::Config;

mod automove;
mod checker;
mod cli;
mod commands;
mod config;
mod rules;

fn main() {
    match run() {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();

    // Read config
    let config_path = find_config_file_path(&cli)?;

    let config_contents = fs::read_to_string(&config_path)
        .map_err(|err| anyhow::format_err!("Could not read config file: {}", err))?;
    let config_dir = config_path
        .parent()
        .map_or(PathBuf::new(), Path::to_path_buf);
    let config: Config = toml::from_str(&config_contents)?;

    // Run command
    match cli.command {
        Commands::Check { target, list } => {
            commands::check::execute(&config, &config_dir, target, list)
        }
        Commands::AutoMove {
            target,
            list,
            dry_run,
        } => commands::automove::execute(&config, &config_dir, target, list, dry_run),
    }?;

    Ok(())
}

fn find_config_file_path(cli: &Cli) -> anyhow::Result<PathBuf> {
    if let Some(path) = &cli.config {
        return Ok(path.clone());
    }
    if let Ok(path) = env::var("SHINYDIR_CONFIG_FILE") {
        Ok(PathBuf::from(path))
    } else {
        create_config_file()
    }
}

fn create_config_file() -> anyhow::Result<PathBuf> {
    let project = directories::ProjectDirs::from("", "", "Shiny Dir")
        .with_context(|| "unable to find config directory")?;
    let file_path = project.config_dir().join("shinydir.toml");
    if !file_path.try_exists().unwrap_or(true) {
        eprintln!(
            "Copying default configuration to {:?} because no config file was found...",
            file_path
        );
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&file_path)?;
        let default_config = include_str!("../shinydir.toml").as_bytes();
        file.write_all(default_config)?;
        file.flush()?;
        eprintln!();
    }
    Ok(file_path)
}
