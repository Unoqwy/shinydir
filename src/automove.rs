use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use crate::checker::FileRule;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct AutoMove {
    pub parent: Option<PathBuf>,
    pub rules: Vec<AutoMoveRule>,
}

#[derive(Debug, Clone)]
pub struct AutoMoveRule {
    pub directory: PathBuf,
    pub to: PathBuf,
    pub match_rules: FileRule,
}

#[derive(Debug, Clone)]
pub enum AutoMoveResult {
    DirDoesNotExist { directory: PathBuf },
    Ok { entries: Vec<AutoMoveResultEntry> },
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
    /// Returns entries that should be moved if it didn't encounter any error
    pub fn run(&self) -> AutoMoveResult {
        self.run_on_path(&self.directory, false)
    }

    /// Checks if any file would be moved without getting a full list of entries
    pub fn would_move(&self) -> bool {
        let result = self.run_on_path(&self.directory, true);
        match result {
            AutoMoveResult::Ok { entries } if !entries.is_empty() => true,
            _ => false,
        }
    }

    fn run_on_path(&self, path: &Path, exit_on_first: bool) -> AutoMoveResult {
        let dir_entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => {
                return AutoMoveResult::DirDoesNotExist {
                    directory: path.to_path_buf(),
                }
            }
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
                let mut move_to = self.to.clone();
                move_to.push(dir_entry.file_name());
                let entry = AutoMoveResultEntry {
                    file: dir_entry.path(),
                    file_metadata,
                    move_to,
                };
                result_entries.push(entry);
                if exit_on_first {
                    break;
                }
            }
        }

        AutoMoveResult::Ok {
            entries: result_entries,
        }
    }
}

pub fn from_config(config: &Config, parent: Option<PathBuf>) -> anyhow::Result<AutoMove> {
    let mut rules = Vec::new();
    for config_rule in config.automove.rules.iter() {
        let mut match_rules = FileRule::Any;
        if !config_rule.match_rules.is_empty() {
            let compiled = crate::config::compile_match_rules(&config_rule.match_rules)?;
            match_rules = FileRule::Name(compiled);
        }

        rules.push(AutoMoveRule {
            directory: PathBuf::from(shellexpand::env(&config_rule.parent)?.as_ref()),
            to: PathBuf::from(shellexpand::env(&config_rule.to)?.as_ref()),
            match_rules,
        });
    }

    rules.sort_by_cached_key(|rule| rule.directory.clone());
    Ok(AutoMove { parent, rules })
}
