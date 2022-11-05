use std::fs;
use std::path::PathBuf;

use colored::Colorize;

use crate::checker::{CheckerResult, Report, ReportIssue};
use crate::config::{AutoMoveReportInfo, Config, Settings};

pub fn execute(
    config: &Config,
    config_dir: PathBuf,
    target: Option<PathBuf>,
    list: bool,
) -> anyhow::Result<()> {
    // Setup checker
    let parent = target.map(fs::canonicalize).transpose()?;
    let checker = crate::checker::from_config(&config, parent.clone())?;

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
                print_report(&config.settings, report);
            }
        } else if !list {
            if config.settings.color {
                eprintln!(
                    "{} {}",
                    result.path().to_string_lossy().red(),
                    result.format_err()
                );
            } else {
                eprintln!(
                    "{}: {}",
                    result.path().to_string_lossy(),
                    result.format_err()
                );
            }
        }
    }

    // Automove info
    let automove = crate::automove::from_config(config, config_dir, parent)?;
    match config.automove.report_info {
        AutoMoveReportInfo::Any if automove.would_move_any() => {
            if config.settings.color {
                println!(
                    "\n{} {}",
                    "Some files can be automatically moved!".yellow().bold(),
                    "(Run auto-move command)".white().dimmed()
                );
            } else {
                println!("\nSome files can be automatically moved! (Run auto-move command)");
            }
        }
        AutoMoveReportInfo::Count => {
            let count = automove.rules.iter().fold(0, |a, b| a + b.count_move());
            if count > 0 {
                if config.settings.color {
                    println!(
                        "\n{} {} {}",
                        format!("{}", count).bright_yellow().bold(),
                        "files can be automatically moved!".yellow().bold(),
                        "(Run auto-move command)".white().dimmed()
                    );
                } else {
                    println!(
                        "\n{} files can be automatically moved! (Run auto-move command)",
                        count
                    );
                }
            }
        }
        _ => {}
    };

    Ok(())
}

fn print_report(settings: &Settings, report: Report) {
    if report.issues.is_empty() {
        let checkmark = if settings.unicode { "\u{f00c}" } else { "OK" };
        if settings.color {
            println!(
                "{} {}",
                report.path.to_string_lossy().blue(),
                checkmark.green().bold()
            )
        } else {
            println!("{} {}", report.path.to_string_lossy(), checkmark);
        }
        return;
    }

    let xmark = if settings.unicode { "\u{f467}" } else { "X" };
    let total_files = report.issues.iter().count();
    let misplaced_files_str = format!("{} misplaced files", total_files);
    if settings.color {
        println!(
            "{} {} {}",
            report.path.to_string_lossy().blue(),
            xmark.red().bold(),
            misplaced_files_str.bright_yellow()
        );
    } else {
        println!(
            "{} {} {}",
            report.path.to_string_lossy(),
            xmark,
            misplaced_files_str
        );
    }

    let (directories_str, directories_count) =
        joined_rel_files(settings, &report, |issue| issue.file_metadata().is_dir());
    let (files_str, files_count) =
        joined_rel_files(settings, &report, |issue| issue.file_metadata().is_file());
    if settings.color {
        if directories_count > 0 {
            println!(
                "{} {}{} {}",
                "Directories".bright_white().bold(),
                format!("({})", directories_count).bright_yellow().bold(),
                ":".bright_white().bold(),
                directories_str
            );
        }
        if files_count > 0 {
            println!(
                "{} {}{} {}",
                "Files".bright_white().bold(),
                format!("({})", files_count).bright_yellow().bold(),
                ":".bright_white().bold(),
                files_str
            );
        }
    } else {
        if directories_count > 0 {
            println!("Directories ({}): {}", directories_count, directories_str);
        }
        if files_count > 0 {
            println!("Files ({}): {}", files_count, files_str);
        }
    }
}

fn joined_rel_files<P>(settings: &Settings, report: &Report, predicate: P) -> (String, usize)
where
    P: FnMut(&&ReportIssue) -> bool,
{
    let it = report
        .issues
        .iter()
        .filter(predicate)
        .flat_map(|issue| issue.path().strip_prefix(&report.path).ok())
        .map(|path| path.to_string_lossy());
    if settings.color {
        let mut tmp = it
            .map(|path| format!("{}", path.white()))
            .collect::<Vec<_>>();
        let count = tmp.len();
        tmp.sort();
        (tmp.join(&format!("{} ", ",".bright_black())), count)
    } else {
        let mut tmp = it.collect::<Vec<_>>();
        let count = tmp.len();
        tmp.sort();
        (tmp.join(", "), count)
    }
}
