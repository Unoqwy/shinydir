use std::fs;

use checker::{CheckerResult, Report};
use config::Config;

mod automove;
mod checker;
mod config;

fn main() -> Result<(), anyhow::Error> {
    // Read config
    let config_path = "shinydir.toml";
    let config_contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_contents)?;

    // Setup checker
    let pwd = fs::canonicalize(".")?;
    let checker = checker::from_config(&config, Some(pwd))?;

    // Run & display results
    let results = checker.run();
    let mut first_it = true;
    for result in results {
        if first_it {
            first_it = false;
        } else {
            println!("");
        }

        if let CheckerResult::Ok(report) = result {
            print_report(report);
        } else {
            eprintln!("{}", result.format_err());
        }
    }

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
