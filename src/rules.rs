use std::fs;

use regex::{RegexSet, RegexSetBuilder};

use crate::config;

/// A rule check whether a file matches configured criteria
#[derive(Debug, Clone)]
pub enum FileMatchRule {
    /// No file is valid
    None,

    /// All the nested rules must match for the file to be valid
    MergeAnd(Vec<FileMatchRule>),
    /// Any of the nested rules needs to match for the file to be valid
    MergeOr(Vec<FileMatchRule>),

    /// The file needs to be of the given type to be valid
    Type(FileType),
    /// The file's name needs to match the given [`RegexSet`] to be valid
    Name(RegexSet),
}

/// A type of file
#[derive(Debug, Clone)]
pub enum FileType {
    /// A simple file
    File,
    /// A directory
    Directory,
}

impl FileMatchRule {
    /// Checks if a directory entry matches this rule
    pub fn matches_dir_entry(&self, dir_entry: &fs::DirEntry) -> anyhow::Result<bool> {
        let res = match self {
            Self::None => false,

            Self::MergeAnd(merge) => {
                let mut res = true;
                for rule in merge {
                    if !rule.matches_dir_entry(dir_entry)? {
                        res = false;
                        break;
                    }
                }
                res
            }
            Self::MergeOr(merge) => {
                let mut res = false;
                if merge.is_empty() {
                    res = true;
                }
                for rule in merge {
                    if rule.matches_dir_entry(dir_entry)? {
                        res = true;
                        break;
                    }
                }
                res
            }

            Self::Type(file_type) => {
                let metadata = Some(resolve_metadata(dir_entry)?);
                match file_type {
                    FileType::Directory => metadata.as_ref().unwrap().is_dir(),
                    FileType::File => metadata.as_ref().unwrap().is_file(),
                }
            }
            Self::Name(pattern) => pattern.is_match(dir_entry.file_name().to_str().unwrap()),
        };
        Ok(res)
    }
}

/// Returns a dir entry's file metadata after following symlinks
pub fn resolve_metadata(dir_entry: &fs::DirEntry) -> anyhow::Result<fs::Metadata> {
    let symlink = dir_entry.file_type()?.is_symlink();
    if symlink {
        Ok(fs::metadata(dir_entry.path())?)
    } else {
        Ok(dir_entry.metadata()?)
    }
}

/// Compiles a list of configuration match rules into a [`FileMatchRule`] for efficient checks
pub fn compile_config_rules(rules: &Vec<config::MatchRule>) -> anyhow::Result<FileMatchRule> {
    if rules.is_empty() {
        // empty rules, meaning no file can be valid
        return Ok(FileMatchRule::None);
    }

    let mut merge_rules = Vec::new();
    if let Some(pattern) = compile_filename_pattern(rules)? {
        merge_rules.push(FileMatchRule::Name(pattern));
    }

    let merged = if merge_rules.is_empty() {
        FileMatchRule::None
    } else if merge_rules.len() == 1 {
        merge_rules.swap_remove(0)
    } else {
        FileMatchRule::MergeOr(merge_rules)
    };
    Ok(merged)
}

fn compile_filename_pattern(rules: &Vec<config::MatchRule>) -> anyhow::Result<Option<RegexSet>> {
    let mut names = <Vec<&str>>::new();
    let mut extensions = <Vec<&str>>::new();
    let mut raw_patterns = Vec::new();

    for rule in rules {
        match rule {
            config::MatchRule::Name { name } => names.push(name),
            config::MatchRule::Extension { ext } => extensions.push(ext),
            config::MatchRule::Pattern { pattern } => raw_patterns.push(pattern.clone()),
        }
    }

    let mut patterns = Vec::new();
    if !names.is_empty() {
        let match_pat = names
            .into_iter()
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join("|");
        patterns.push(format!("^({})$", match_pat));
    }
    if !extensions.is_empty() {
        let match_pat = extensions
            .into_iter()
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join("|");
        patterns.push(format!("\\.({})$", match_pat));
    }
    patterns.extend(raw_patterns);

    if patterns.is_empty() {
        return Ok(None);
    }
    let filename_pattern = RegexSetBuilder::new(patterns)
        .unicode(true)
        .case_insensitive(false)
        .multi_line(false)
        .build()?;
    Ok(Some(filename_pattern))
}
