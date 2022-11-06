use std::fs::{self, DirEntry, Metadata};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::rules::{self, FileMatchRule, FileType};

#[derive(Debug, Clone)]
pub struct Checker {
    pub parent: Option<PathBuf>,
    pub directories: Vec<DirectoryChecker>,
}

/// Checker configuration for a directory
#[derive(Debug, Clone)]
pub struct DirectoryChecker {
    /// Path of the directory
    pub path: PathBuf,
    /// Valid file rules. Any file that doesn't match will be reported as misplaced
    pub rules: FileMatchRule,
    /// Whether to check rules recursively
    pub recursive: bool,
    /// Children directories to ignore when `recursive` is `true`
    pub recursive_ignore_rules: FileMatchRule,
}

/// Result from attempting to check a directory
#[derive(Debug, Clone)]
pub enum CheckerResult {
    Ok(Report),
    MissingDirectory { path: PathBuf },
    NotADirectory { path: PathBuf },
}

/// A report for a directory that was checked
#[derive(Debug, Clone)]
pub struct Report {
    /// Path of the checked file
    pub path: PathBuf,
    /// Reported issues for the directory itself and children
    pub issues: Vec<ReportIssue>,
}

/// A misplaced file
#[derive(Debug, Clone)]
pub struct ReportIssue {
    /// Path of the misplaced file
    path: PathBuf,
    /// Current metadata of the file
    metadata: Metadata,
}

impl Checker {
    /// Executes directory rules to get a list of misplaced files
    pub fn run(&self) -> Vec<CheckerResult> {
        let mut results = Vec::new();
        for directory in self.directories.iter() {
            if let Some(parent) = &self.parent {
                if !directory.path.starts_with(parent) {
                    continue;
                }
            }
            results.push(directory.check());
        }
        results
    }
}

impl DirectoryChecker {
    pub fn check(&self) -> CheckerResult {
        match fs::metadata(&self.path) {
            Ok(md) if md.is_dir() => (),
            Ok(_) => {
                return CheckerResult::NotADirectory {
                    path: self.path.to_path_buf(),
                }
            }
            Err(_) => {
                return CheckerResult::MissingDirectory {
                    path: self.path.to_path_buf(),
                }
            }
        };
        self.check_dir(&self.path)
    }

    fn check_dir(&self, path: &Path) -> CheckerResult {
        let dir_entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => {
                return CheckerResult::NotADirectory {
                    path: self.path.to_path_buf(),
                }
            }
        };

        let mut issues = Vec::new();
        for entry in dir_entries {
            if let Ok(entry) = entry {
                if let Ok(Some(issue)) = self.rules.test_from_dir_entry(&entry) {
                    issues.push(issue);
                }
                if self.recursive && entry.file_type().ok().map_or(false, |ft| ft.is_dir()) {
                    if self
                        .recursive_ignore_rules
                        .matches_dir_entry(&entry)
                        .ok()
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    if let CheckerResult::Ok(report) = self.check_dir(&entry.path()) {
                        issues.extend(report.issues);
                    }
                }
            }
        }
        CheckerResult::Ok(Report {
            path: path.to_path_buf(),
            issues,
        })
    }
}

impl FileMatchRule {
    pub fn test_from_dir_entry(&self, dir_entry: &DirEntry) -> anyhow::Result<Option<ReportIssue>> {
        if self.matches_dir_entry(dir_entry)? {
            Ok(None)
        } else {
            Ok(Some(ReportIssue {
                path: dir_entry.path(),
                metadata: crate::rules::resolve_metadata(dir_entry)?,
            }))
        }
    }
}

impl CheckerResult {
    pub fn path(&self) -> &Path {
        match self {
            CheckerResult::Ok(report) => &report.path,
            CheckerResult::MissingDirectory { path } => path,
            CheckerResult::NotADirectory { path } => path,
        }
    }

    pub fn format_err(&self) -> String {
        match self {
            CheckerResult::Ok(_) => format!("Ok"),
            CheckerResult::MissingDirectory { .. } => {
                format!("Directory does not exist!")
            }
            CheckerResult::NotADirectory { .. } => {
                format!("File is not a directory!")
            }
        }
    }
}

impl ReportIssue {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

/// Sets up a [`Checker`] from config
pub fn from_config(config: &Config, parent: Option<PathBuf>) -> anyhow::Result<Checker> {
    let mut directories = Vec::new();
    for (dir_path, dir_config) in config.directories.iter() {
        let raw_path = shellexpand::env(dir_path)?;
        let path = PathBuf::from(raw_path.as_ref());

        let mut rules_dir = vec![FileMatchRule::Type(FileType::Directory)];
        if let Some(rules) = &dir_config.allowed_dirs {
            rules_dir.push(rules::compile_config_rules(rules)?);
        }

        let mut rules_file = vec![FileMatchRule::Type(FileType::File)];
        if let Some(rules) = &dir_config.allowed_files {
            rules_file.push(rules::compile_config_rules(rules)?);
        }

        // recursive ignore only applies on directories anyway, no need to ignore FileType::File here
        let recursive_ignore_children =
            rules::compile_config_rules(&dir_config.recursive_ignore_children)?;

        let rules = FileMatchRule::MergeOr(vec![
            FileMatchRule::MergeAnd(rules_dir),
            FileMatchRule::MergeAnd(rules_file),
        ]);
        directories.push(DirectoryChecker {
            path,
            recursive: dir_config.recursive,
            recursive_ignore_rules: recursive_ignore_children,
            rules,
        });
    }

    directories.sort_by_cached_key(|dir| dir.path.clone());
    Ok(Checker {
        parent,
        directories,
    })
}
