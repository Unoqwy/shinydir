use std::ffi::OsString;
use std::fs::{self, Metadata};
use std::os::unix::prelude::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::format_err;

use crate::checker::FileRule;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct AutoMove {
    pub parent: Option<PathBuf>,
    pub rules: Vec<AutoMoveRule>,
}

#[derive(Debug, Clone)]
pub struct AutoMoveRule {
    pub custom_name: Option<String>,
    pub directory: PathBuf,
    pub to: PathBuf,
    pub to_script: Option<PathBuf>,
    pub match_rules: FileRule,
}

#[derive(Debug)]
pub enum AutoMoveResult<'a> {
    DirDoesNotExist {
        rule: &'a AutoMoveRule,
    },
    Ok {
        rule: &'a AutoMoveRule,
        entries: Vec<Result<AutoMoveResultEntry, anyhow::Error>>,
    },
}

#[derive(Debug, Clone)]
pub struct AutoMoveResultEntry {
    pub file: PathBuf,
    pub file_metadata: Metadata,

    pub move_to: PathBuf,
}

impl AutoMove {
    pub fn run(&self) -> Vec<AutoMoveResult> {
        let mut results = Vec::new();
        for rule in self.rules.iter() {
            if let Some(parent) = &self.parent {
                if !rule.directory.starts_with(parent) {
                    continue;
                }
            }
            results.push(rule.run());
        }
        results
    }

    pub fn would_move_any(&self) -> bool {
        for rule in self.rules.iter() {
            if rule.would_move() {
                return true;
            }
        }
        false
    }
}

impl AutoMoveRule {
    pub fn display_name(&self) -> String {
        self.custom_name
            .clone()
            .unwrap_or_else(|| self.directory.to_path_buf().to_string_lossy().to_string())
    }

    /// Returns entries that should be moved if it didn't encounter any error
    pub fn run(&self) -> AutoMoveResult {
        self.run_on_path(&self.directory)
    }

    /// Checks if any file would be moved without getting a full list of entries
    pub fn would_move(&self) -> bool {
        let result = self.count_matches_on_path(&self.directory, true);
        result >= 1
    }

    pub fn count_move(&self) -> usize {
        self.count_matches_on_path(&self.directory, false)
    }

    fn count_matches_on_path(&self, path: &Path, exit_on_first: bool) -> usize {
        let dir_entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return 0,
        };

        let mut count = 0;
        for dir_entry in dir_entries {
            if dir_entry.is_err() {
                continue;
            }
            let dir_entry = dir_entry.unwrap();
            if !self
                .match_rules
                .matches_dir_entry(&dir_entry)
                .ok()
                .unwrap_or(false)
            {
                continue;
            }
            if dir_entry.metadata().is_ok() {
                count += 1;
            }
            if exit_on_first {
                break;
            }
        }
        count
    }

    fn run_on_path(&self, path: &Path) -> AutoMoveResult {
        let dir_entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return AutoMoveResult::DirDoesNotExist { rule: self },
        };

        let mut result_entries = Vec::new();
        for dir_entry in dir_entries {
            if dir_entry.is_err() {
                continue;
            }
            let dir_entry = dir_entry.unwrap();
            if !self
                .match_rules
                .matches_dir_entry(&dir_entry)
                .ok()
                .unwrap_or(false)
            {
                continue;
            }
            if let Ok(file_metadata) = dir_entry.metadata() {
                let mut output_filename = dir_entry.file_name();
                if let Some(to_script) = &self.to_script {
                    let output = Command::new(to_script)
                        .arg(dir_entry.path().to_string_lossy().as_ref())
                        .output();
                    if let Err(err) = output {
                        result_entries.push(Err(format_err!(
                            "Could not execute to_script for '{}': {}: {}",
                            dir_entry.file_name().to_string_lossy(),
                            to_script.to_string_lossy(),
                            err
                        )));
                        continue;
                    }
                    let output = output.unwrap();
                    let utf8_filename = String::from_utf8_lossy(&output.stdout);
                    let utf8_filename_trimmed = utf8_filename.trim_end();
                    let trim_end_bytes = utf8_filename.len() - utf8_filename_trimmed.len();
                    let trim_start_bytes =
                        utf8_filename_trimmed.len() - utf8_filename_trimmed.trim_start().len();
                    output_filename = OsString::from_vec(
                        (&output.stdout[trim_start_bytes
                            ..(output.stdout.len() - trim_start_bytes) - trim_end_bytes])
                            .to_vec(),
                    );
                }

                let move_to = if Path::new(&output_filename).is_absolute() {
                    PathBuf::from(output_filename)
                } else {
                    let mut buf = self.to.clone();
                    buf.push(output_filename);
                    buf
                };
                let entry = AutoMoveResultEntry {
                    file: dir_entry.path(),
                    file_metadata,
                    move_to,
                };
                result_entries.push(Ok(entry));
            }
        }

        AutoMoveResult::Ok {
            rule: self,
            entries: result_entries,
        }
    }
}

pub fn from_config(
    config: &Config,
    config_dir: PathBuf,
    parent: Option<PathBuf>,
) -> anyhow::Result<AutoMove> {
    let mut rules = Vec::new();
    for config_rule in config.automove.rules.iter() {
        let mut match_rules = FileRule::Any;
        if !config_rule.match_rules.is_empty() {
            let compiled = crate::config::compile_match_rules(&config_rule.match_rules)?;
            match_rules = FileRule::Name(compiled);
        }

        rules.push(AutoMoveRule {
            custom_name: config_rule.name.clone(),
            directory: PathBuf::from(shellexpand::env(&config_rule.parent)?.as_ref()),
            to: PathBuf::from(shellexpand::env(&config_rule.to)?.as_ref()),
            to_script: if let Some(path) = &config_rule.to_script {
                let expanded_path = shellexpand::env(path)?;
                let expanded_path = Path::new(expanded_path.as_ref());
                if expanded_path.is_absolute() {
                    Some(expanded_path.to_path_buf())
                } else {
                    let mut path = config_dir.clone();
                    path.push(expanded_path);
                    Some(path)
                }
            } else {
                None
            },
            match_rules,
        });
    }

    rules.sort_by_cached_key(|rule| rule.display_name());
    Ok(AutoMove { parent, rules })
}
