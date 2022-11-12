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

    set_missing_env_vars();

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

/// Defines xdg directories environment variables in a cross-platform manner
fn set_missing_env_vars() {
    if let Some(dirs) = directories::UserDirs::new() {
        set_env_var_if_missing("HOME", Some(dirs.home_dir()));
        set_env_var_if_missing("XDG_HOME_DIR", Some(dirs.home_dir()));
        set_env_var_if_missing("XDG_DOCUMENTS_DIR", dirs.document_dir());
        set_env_var_if_missing("XDG_DOWNLOAD_DIR", dirs.download_dir());
        set_env_var_if_missing("XDG_PICTURES_DIR", dirs.picture_dir());
        set_env_var_if_missing("XDG_MUSIC_DIR", dirs.audio_dir());
        set_env_var_if_missing("XDG_VIDEOS_DIR", dirs.video_dir());
        set_env_var_if_missing("XDG_DESKTOP_DIR", dirs.desktop_dir());
    } else {
        eprintln!("No valid home directory path could be retrieved. XDG_*_DIR environment variables won't be accessible.");
    }
}

fn set_env_var_if_missing(var_name: &str, value: Option<&Path>) {
    if let Some(value) = value {
        if env::var(var_name) == Err(env::VarError::NotPresent) {
            env::set_var(var_name, value);
        }
    }
}
