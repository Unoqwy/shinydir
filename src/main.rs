use std::fs;
use std::path::PathBuf;

use clap::Parser;

use checker::{CheckerResult, Report};
use cli::{Commands, CLI};
use config::Config;

mod automove;
mod checker;
mod cli;
mod config;

fn main() -> anyhow::Result<()> {
    let cli = CLI::parse();

    // Read config
    let config_path = match cli.config {
        Some(path) => path,
        None => "shinydir.toml".into(),
    };
    let config_contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_contents)?;

    // Run command
    match cli.command {
        Commands::Check { target, list } => command_check(&config, target, list),
        Commands::AutoMove { target, dry_run } => command_automove(&config, target, dry_run),
    }?;

    Ok(())
}

fn command_check(config: &Config, target: Option<PathBuf>, list: bool) -> anyhow::Result<()> {
    // Setup checker
    let parent = target.map(fs::canonicalize).transpose()?;
    let checker = checker::from_config(&config, parent)?;

    // Run & display results
    let results = checker.run();
    let mut first_it = true;
    for result in results {
        if first_it {
            first_it = false;
        } else if !list {
            println!("");
        }

        if let CheckerResult::Ok(report) = result {
            if list {
                let abs_files = report
                    .issues
                    .iter()
                    .map(|issue| issue.path().to_string_lossy())
                    .collect::<Vec<_>>();
                if abs_files.is_empty() {
                    continue;
                }
                println!("{}", abs_files.join("\n"));
            } else {
                print_report(report);
            }
        } else {
            eprintln!("{}", result.format_err());
        }
    }

    Ok(())
}

fn command_automove(config: &Config, target: Option<PathBuf>, dry_run: bool) -> anyhow::Result<()> {
    let parent = target.map(fs::canonicalize).transpose()?;

    Ok(())
}

fn print_report(report: Report) {
    if report.issues.is_empty() {
        println!("Directory {} is all good!", report.path.to_string_lossy());
        return;
    }

    let rel_files = report
        .issues
        .iter()
        .flat_map(|issue| issue.path().strip_prefix(&report.path).ok())
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "Unexpected Files ({}): {}",
        report.path.to_string_lossy(),
        rel_files
    );
}
