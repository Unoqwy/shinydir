use anyhow::Error;
use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::automove::{AutoMoveResult, AutoMoveResultEntry, AutoMoveRule};
use crate::config::{Config, Settings};

pub fn execute(
    config: &Config,
    config_dir: &Path,
    target: Option<PathBuf>,
    list: bool,
    mut dry_run: bool,
) -> anyhow::Result<()> {
    // Setup automove
    let parent = target.map(fs::canonicalize).transpose()?;
    let automove = crate::automove::from_config(config, config_dir, parent)?;

    automove.check_empty(config)?;
    let script_warning = automove.script_warning(config);
    dry_run_warning(config, &mut dry_run);

    let mut results = automove.run(); // Get entries to move

    if (script_warning || dry_run) && !list {
        eprintln!(); // Print newline after info message
    }

    // Move files
    for result in &mut results {
        if let AutoMoveResult::Ok { entries, .. } = result {
            process_automove_result_entry(config, &mut dry_run, entries);
        }
    }

    // Display output
    let mut first_entry = true;
    let mut hidden = 0;
    let mut any_move = false;
    for result in &results {
        match result {
            AutoMoveResult::DirDoesNotExist { rule } if !list => {
                if first_entry {
                    first_entry = false;
                } else {
                    println!();
                }
                let display_name = if rule.custom_name.is_none() && config.settings.color {
                    format!("{}", rule.display_name().italic())
                } else {
                    rule.display_name()
                };
                if config.settings.color {
                    eprintln!("{} Directory does not exist!", display_name.red());
                } else {
                    eprintln!("{}: Directory does not exist!", display_name);
                }
            }
            AutoMoveResult::Ok { rule, entries } => {
                if list {
                    let line_entries = entries
                        .iter()
                        .filter_map(|entry| entry.as_ref().ok())
                        .map(|entry| {
                            format!(
                                "{} {}",
                                entry.file.to_string_lossy().replace(' ', "\\ "),
                                entry.move_to.to_string_lossy().replace(' ', "\\ ")
                            )
                        })
                        .collect::<Vec<_>>();
                    if !line_entries.is_empty() {
                        println!("{}", line_entries.join("\n"));
                    }
                } else if config.settings.hide_ok_directories && entries.is_empty() {
                    hidden += 1;
                } else {
                    if first_entry {
                        first_entry = false;
                    } else {
                        println!();
                    }
                    print_entries(&config.settings, rule, entries);
                    any_move = true;
                }
            }
            AutoMoveResult::DirDoesNotExist { .. } => {}
        };
    }

    if hidden > 0 && !list {
        if hidden != results.len() {
            println!();
        }
        show_hidden_info(config, hidden);
    }

    if config.automove.force_dry_run && any_move {
        if config.settings.color {
            eprintln!("\n\n{}", "No files were actually moved as you are a new user. Please refer to the \"Info!\" note at the beginning of this output.".italic());
        } else {
            eprintln!("\n\nNo files were actually moved as you are a new user. Please refer to the \"Info!\" note at the beginning of this output.");
        }
    }

    Ok(())
}

fn show_hidden_info(config: &Config, hidden: usize) {
    if config.settings.color {
        println!(
            "{} {}",
            if config.settings.unicode {
                format!("\u{f00c} {} rules", hidden)
            } else {
                format!("{} rules", hidden)
            }
            .bright_white()
            .bold()
            .italic(),
            "were hidden from the output (nothing to move)"
                .bright_white()
                .italic(),
        );
    } else {
        println!(
            "{} rules were hidden from the output (nothing to move)",
            if config.settings.unicode {
                format!("\u{f00c} {}", hidden)
            } else {
                format!("{}", hidden)
            },
        );
    }
}

fn process_automove_result_entry(
    config: &Config,
    dry_run: &mut bool,
    entries: &mut Vec<Result<AutoMoveResultEntry, Error>>,
) {
    for entry_res in entries {
        let entry = if let Ok(entry) = entry_res.as_ref() {
            entry
        } else {
            continue;
        };
        if !*dry_run {
            if let Some(parent) = entry.move_to.parent() {
                if let Err(err) = fs::create_dir_all(parent).map_err(|err| {
                    anyhow::format_err!(
                        "Couldn't create directory {}: {}",
                        parent.to_string_lossy(),
                        err
                    )
                }) {
                    *entry_res = Err(err);
                    continue;
                }
            }
        }
        let new_err = match entry.move_to.try_exists() {
            Ok(true) if !config.automove.allow_overwrite => Some(anyhow::format_err!(
                "Moving to {} would overwrite a file",
                entry.move_to.to_string_lossy()
            )),
            Err(err) => Some(anyhow::format_err!(
                "Cannot check overwrite status for {}: {}",
                entry.move_to.to_string_lossy(),
                err
            )),
            _ if !*dry_run => fs::rename(&entry.file, &entry.move_to)
                .map_err(|err| {
                    anyhow::format_err!(
                        "Couldn't move {} to {}: {}",
                        entry.file.to_string_lossy(),
                        entry.move_to.to_string_lossy(),
                        err
                    )
                })
                .err(),
            _ => None,
        };
        if let Some(err) = new_err {
            *entry_res = Err(err);
        }
    }
}

/// Warn user about dry run
fn dry_run_warning(config: &Config, dry_run: &mut bool) {
    if config.automove.force_dry_run {
        *dry_run = true;
        if config.settings.color {
            eprintln!(
                "{} Dry run is enabled for newly copied configs as a security measure. Turn off {} in the config file to disable this security. {}",
                "Info!".bright_yellow().bold(),
                "force-dry-run".dimmed(),
                "Until then, no file will actually be moved!".bold(),
            );
        } else {
            eprintln!("INFO! Dry run is enabled for newly copied configs as a security measure. Turn off 'force-dry-run' in the config file to disable this security. Until then, no file will actually be moved!");
        }
    } else if *dry_run {
        if config.settings.color {
            eprintln!(
                "{} Auto-move running in {}, no files will actually be moved.",
                "Info!".bright_blue().bold(),
                "dry mode".white().bold()
            );
        } else {
            eprintln!("INFO! Auto-move running in dry mode, no files will actually be moved.");
        }
    }
}

fn print_entries(
    settings: &Settings,
    rule: &AutoMoveRule,
    entries: &[Result<AutoMoveResultEntry, anyhow::Error>],
) {
    let display_name = if rule.custom_name.is_none() && settings.color {
        format!("{}", rule.display_name().italic())
    } else {
        rule.display_name()
    };

    if entries.is_empty() {
        let checkmark = if settings.unicode { "\u{f00c}" } else { "OK" };
        if settings.color {
            println!("{} {}", display_name.blue(), checkmark.green().bold());
        } else {
            println!("{} {}", display_name, checkmark);
        }
        return;
    }

    let valid_entries = entries.iter().filter(|entry| entry.is_ok()).count();
    let errors = entries.iter().filter(|entry| entry.is_err()).count();

    let dot = if settings.unicode { "\u{f444}" } else { "-" };
    let mut info = Vec::new();
    if valid_entries > 0 {
        let msg = format!("{} files moved", valid_entries);
        if settings.color {
            info.push(format!("{}", msg.bright_yellow()));
        } else {
            info.push(msg);
        }
    }
    if errors > 0 {
        let msg = format!("{} errors", errors);
        if settings.color {
            info.push(format!("{}", msg.bright_red()));
        } else {
            info.push(msg);
        }
    }
    let info_sep = if settings.color { " " } else { ", " };
    if settings.color {
        println!(
            "{} {} {}",
            display_name.blue(),
            dot.white().dimmed(),
            info.join(info_sep)
        );
    } else {
        println!("{} {} {}", display_name, dot, info.join(info_sep));
    }

    let moved_to_dirs_no_dedup = entries
        .iter()
        .filter_map(|entry| entry.as_ref().ok())
        .filter_map(|entry| entry.move_to.parent())
        .map(std::path::Path::to_path_buf)
        .collect::<Vec<_>>();
    let mut moved_to_dirs = moved_to_dirs_no_dedup.clone();
    moved_to_dirs.sort();
    moved_to_dirs.dedup();

    if moved_to_dirs.is_empty() {
        for err in entries.iter().filter_map(|entry| entry.as_ref().err()) {
            eprintln!("{}", format!("{}", err).bright_red().italic());
        }
        return;
    }

    let arrow = "=>";
    let rel_dirs_it = moved_to_dirs
        .iter()
        .map(|path| {
            let count = moved_to_dirs_no_dedup
                .iter()
                .filter(|&dir| path.eq(dir))
                .count();
            (path, count)
        })
        .map(|(path, count)| (path.strip_prefix(&rule.directory).unwrap_or(path), count));
    if settings.color {
        let tmp = rel_dirs_it
            .map(|(path, count)| {
                format!(
                    "{} {}",
                    path.to_string_lossy().bright_blue(),
                    format!("({})", count).dimmed()
                )
            })
            .collect::<Vec<_>>();
        println!(
            "{} {} {}",
            arrow.black(),
            "Moved To".bright_white().bold(),
            tmp.join(&format!("{}", ", ".bright_black()))
        );
    } else {
        let tmp = rel_dirs_it
            .map(|(path, count)| format!("{} {}", path.to_string_lossy(), count))
            .collect::<Vec<_>>();
        println!("{} Moved To: {}", arrow, tmp.join(", "));
    }

    for err in entries.iter().filter_map(|entry| entry.as_ref().err()) {
        eprintln!("{}", format!("{}", err).bright_red().italic());
    }
}
