use std::fs;
use std::path::PathBuf;

use crate::automove::AutoMoveResult;
use crate::config::Config;

pub fn execute(
    config: &Config,
    target: Option<PathBuf>,
    list: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    // Setup automove
    let parent = target.map(fs::canonicalize).transpose()?;
    let automove = crate::automove::from_config(config, parent)?;

    // Run
    let results = automove.run();
    let mut first_it = true;
    for result in results {
        if first_it {
            first_it = false;
        } else if !list {
            println!("");
        }

        match result {
            AutoMoveResult::DirDoesNotExist { directory } if !list => {
                eprintln!("Directory {} does not exist", directory.to_string_lossy());
            }
            AutoMoveResult::Ok { entries } => {
                if list {
                    let line_entries = entries
                        .iter()
                        .map(|entry| {
                            format!(
                                "{} {}",
                                entry.file.to_string_lossy().replace(" ", "\\ "),
                                entry.move_to.to_string_lossy().replace(" ", "\\ ")
                            )
                        })
                        .collect::<Vec<_>>();
                    println!("{}", line_entries.join("\n"));
                }
            }
            _ => {}
        };
    }

    Ok(())
}
