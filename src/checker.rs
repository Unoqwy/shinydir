use std::fs::{self, DirEntry, Metadata};
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
    pub recursive_ignore_rules: FileRule,
    pub rules: FileRule,
}

#[derive(Debug, Clone)]
pub enum FileRule {
    Any,
    None,

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
    UnexpectedFile { path: PathBuf, metadata: Metadata },
}

impl Checker {
    pub fn run(&self) -> Vec<CheckerResult> {
        let mut results = Vec::new();

        for directory in self.directories.iter() {
            if let Some(parent) = &self.parent {
                if !directory.path.starts_with(parent) {
                    continue;
                }
            }

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

impl FileRule {
    pub fn test_from_dir_entry(&self, dir_entry: &DirEntry) -> anyhow::Result<Option<ReportIssue>> {
        let mut metadata = None;
        let res = match self {
            Self::Any => true,
            Self::None => false,
            Self::MergeAnd(merge) => {
                for rule in merge {
                    if let Some(issue) = rule.test_from_dir_entry(dir_entry)? {
                        return Ok(Some(issue));
                    }
                }
                return Ok(None);
            }
            Self::MergeOr(merge) => {
                if merge.is_empty() {
                    return Ok(None);
                }
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
                metadata = Some(resolve_metadata(dir_entry)?);
                match file_type {
                    FileType::Directory => metadata.as_ref().unwrap().is_dir(),
                    FileType::File => metadata.as_ref().unwrap().is_file(),
                }
            }
            Self::Name(pattern) => pattern.is_match(dir_entry.file_name().to_str().unwrap()),
        };

        if res {
            Ok(None)
        } else {
            if metadata.is_none() {
                metadata = Some(resolve_metadata(dir_entry)?);
            }
            Ok(Some(ReportIssue::UnexpectedFile {
                path: dir_entry.path(),
                metadata: metadata.unwrap(),
            }))
        }
    }

    pub fn matches_dir_entry(&self, dir_entry: &DirEntry) -> anyhow::Result<bool> {
        self.test_from_dir_entry(dir_entry).map(|opt| opt.is_none())
    }
}

/// Returns a dir entry's file metadata after following symlinks
fn resolve_metadata(dir_entry: &DirEntry) -> anyhow::Result<Metadata> {
    let symlink = dir_entry.file_type()?.is_symlink();
    if symlink {
        Ok(fs::metadata(dir_entry.path())?)
    } else {
        Ok(dir_entry.metadata()?)
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
            ReportIssue::UnexpectedFile { path, .. } => &path,
        }
    }

    pub fn file_metadata(&self) -> &Metadata {
        match self {
            ReportIssue::UnexpectedFile { metadata, .. } => &metadata,
        }
    }
}

pub fn from_config(config: &Config, parent: Option<PathBuf>) -> Result<Checker, anyhow::Error> {
    let mut directories = Vec::new();
    for (dir_path, dir_config) in config.directories.iter() {
        let raw_path = shellexpand::env(dir_path)?;
        let path = PathBuf::from(raw_path.as_ref());

        let mut rules_dir = vec![FileRule::Type(FileType::Directory)];
        match &dir_config.allowed_dirs {
            None => {}
            Some(allowed) if allowed.is_empty() => rules_dir.push(FileRule::None),
            Some(allowed) => {
                let compiled = crate::config::compile_match_rules(allowed)?;
                rules_dir.push(FileRule::Name(compiled));
            }
        };

        let mut rules_file = vec![FileRule::Type(FileType::File)];
        match &dir_config.allowed_files {
            None => {}
            Some(allowed) if allowed.is_empty() => rules_file.push(FileRule::None),
            Some(allowed) => {
                let compiled = crate::config::compile_match_rules(allowed)?;
                rules_file.push(FileRule::Name(compiled));
            }
        };

        // recursive ignore only applies on directories anyway, no need to ignore FileType::File here
        let mut recursive_ignore_children = FileRule::Any;
        if !dir_config.recursive_ignore_children.is_empty() {
            let compiled =
                crate::config::compile_match_rules(&dir_config.recursive_ignore_children)?;
            recursive_ignore_children = FileRule::Name(compiled);
        }

        let rules = FileRule::MergeOr(vec![
            FileRule::MergeAnd(rules_dir),
            FileRule::MergeAnd(rules_file),
        ]);
        directories.push(DirectoryChecker {
            path,
            recursive: dir_config.recursive,
            recursive_ignore_rules: recursive_ignore_children,
            rules,
        });
    }

    directories.sort_by_cached_key(|dir| dir.path.to_path_buf());
    Ok(Checker {
        parent,
        directories,
    })
}
