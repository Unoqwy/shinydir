use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use regex::RegexSet;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Checker {
    pub parent: Option<PathBuf>,
    pub directories: Vec<DirectoryChecker>,
}

#[derive(Debug, Clone)]
pub struct DirectoryChecker {
    pub path: PathBuf,
    pub recursive: bool,
    pub rules: FileRule,
}

#[derive(Debug, Clone)]
pub enum FileRule {
    MergeAnd(Vec<FileRule>),
    MergeOr(Vec<FileRule>),

    Type(FileType),
    Name(RegexSet),
}

#[derive(Debug, Clone)]
pub enum FileType {
    File,
    Directory,
}

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

/// The type of issue of a report
#[derive(Debug, Clone)]
pub enum ReportIssue {
    UnexpectedFile { path: PathBuf },
}

impl Checker {
    pub fn run(&self) -> Vec<CheckerResult> {
        let mut results = Vec::new();

        for directory in self.directories.iter() {
            let result = directory.check();
            results.push(result);
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
        self.check_dir(&self.path, self.recursive)
    }

    fn check_dir(&self, path: &Path, recursive: bool) -> CheckerResult {
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
                if recursive && entry.file_type().ok().map_or(false, |ft| ft.is_dir()) {
                    if let CheckerResult::Ok(report) = self.check_dir(&entry.path(), recursive) {
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

impl FileRule {
    pub fn test_from_dir_entry(&self, dir_entry: &DirEntry) -> anyhow::Result<Option<ReportIssue>> {
        let res = match self {
            Self::MergeAnd(merge) => {
                for rule in merge {
                    if let Some(issue) = rule.test_from_dir_entry(dir_entry)? {
                        return Ok(Some(issue));
                    }
                }
                return Ok(None);
            }
            Self::MergeOr(merge) => {
                let mut issue = None;
                for rule in merge {
                    // FIXME : This doesn't properly keep track of what issue to report to the user
                    match rule.test_from_dir_entry(dir_entry)? {
                        some @ Some(_) => issue = some,
                        None => return Ok(None),
                    }
                }
                return Ok(issue);
            }

            Self::Type(file_type) => {
                let symlink = dir_entry.file_type()?.is_symlink();
                let metadata = if symlink {
                    fs::metadata(dir_entry.path())?
                } else {
                    dir_entry.metadata()?
                };
                match file_type {
                    FileType::Directory => metadata.is_dir(),
                    FileType::File => metadata.is_file(),
                }
            }
            Self::Name(pattern) => pattern.is_match(dir_entry.file_name().to_str().unwrap()),
        };

        if res {
            Ok(None)
        } else {
            Ok(Some(ReportIssue::UnexpectedFile {
                path: dir_entry.path(),
            }))
        }
    }
}

impl CheckerResult {
    pub fn format_err(&self) -> String {
        match self {
            CheckerResult::Ok(_) => format!("Ok"),
            CheckerResult::MissingDirectory { path } => {
                format!("Directory {} does not exist", path.to_string_lossy())
            }
            CheckerResult::NotADirectory { path } => {
                format!("File {} is not a directory", path.to_string_lossy())
            }
        }
    }
}

impl ReportIssue {
    pub fn path(&self) -> &Path {
        match self {
            ReportIssue::UnexpectedFile { path } => &path,
        }
    }
}

pub fn from_config(config: &Config, parent: Option<PathBuf>) -> Result<Checker, anyhow::Error> {
    let mut directories = Vec::new();
    for (dir_path, dir_config) in config.directories.iter() {
        let raw_path = shellexpand::env(dir_path)?;
        let path = PathBuf::from(raw_path.as_ref());

        let mut rules_dir = vec![FileRule::Type(FileType::Directory)];
        if !dir_config.allowed_dirs.is_empty() {
            let compiled = crate::config::compile_match_rules(&dir_config.allowed_dirs)?;
            rules_dir.push(FileRule::Name(compiled));
        }

        let mut rules_file = vec![FileRule::Type(FileType::File)];
        if !dir_config.allowed_files.is_empty() {
            let compiled = crate::config::compile_match_rules(&dir_config.allowed_files)?;
            rules_file.push(FileRule::Name(compiled));
        }

        let rules = FileRule::MergeOr(vec![
            FileRule::MergeAnd(rules_dir),
            FileRule::MergeAnd(rules_file),
        ]);
        directories.push(DirectoryChecker {
            path,
            recursive: dir_config.recursive,
            rules,
        });
    }

    Ok(Checker {
        parent,
        directories,
    })
}
