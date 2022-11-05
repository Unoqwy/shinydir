use std::collections::HashMap;

use regex::{RegexSet, RegexSetBuilder};
use serde::Deserialize;

/// Configuration file
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,

    #[serde(rename = "dir")]
    pub directories: HashMap<String, DirectoryConfig>,

    pub automove: AutoMoveConfig,
}

/// General application settings
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    #[serde(default = "default_true")]
    pub color: bool,
    #[serde(rename = "use-unicode", default = "default_true")]
    pub unicode: bool,
}

/// Configuration for a directory
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DirectoryConfig {
    pub recursive: bool,
    pub recursive_ignore_children: Vec<MatchRule>,

    pub allowed_dirs: Option<Vec<MatchRule>>,
    pub allowed_files: Option<Vec<MatchRule>>,
}

/// A rule to check if the filename matches
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MatchRule {
    Name { name: String },
    Extension { ext: String },
    Pattern { pattern: String },
}

/// Auto-Move configuration
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AutoMoveConfig {
    /// Whether to show "scripts may slow down the execution" warning
    #[serde(default)]
    pub script_warning: bool,

    /// Level of auto-move report info
    #[serde(default)]
    pub report_info: AutoMoveReportInfo,

    /// Security for new users
    #[serde(default)]
    pub force_dry_run: bool,

    // Here be dragons
    #[serde(default)]
    pub allow_overwrite: bool,

    #[serde(default)]
    pub rules: Vec<AutoMoveRule>,
}

/// What kind of information about Auto-Move files to print
/// at the end of a report
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoMoveReportInfo {
    /// Disable this extra info
    No,
    /// Display if any file can be automatically moved
    Any,
    /// Display the number of files that can be automatically moved
    Count,
}

/// A rule to automatically move files
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AutoMoveRule {
    /// Custom rule name
    pub name: Option<String>,
    /// Parent directory
    pub parent: String,
    /// File matcher (applied of contents of parent directory)
    #[serde(alias = "match")]
    pub match_rules: Vec<MatchRule>,
    /// Which directory to move it to
    pub to: String,
    /// Path to a script that gives the output filename
    pub to_script: Option<String>,
}

impl Default for AutoMoveReportInfo {
    fn default() -> Self {
        Self::Count
    }
}

fn default_true() -> bool {
    true
}

/// Compiles a list of (filename) match rules into a [`RegexSet`] for fast checks
pub fn compile_match_rules(rules: &Vec<MatchRule>) -> anyhow::Result<RegexSet> {
    let mut names = <Vec<&str>>::new();
    let mut extensions = <Vec<&str>>::new();
    let mut raw_patterns = Vec::new();

    for rule in rules {
        match rule {
            MatchRule::Name { name } => names.push(name),
            MatchRule::Extension { ext } => extensions.push(ext),
            MatchRule::Pattern { pattern } => raw_patterns.push(pattern.clone()),
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

    let compiled = RegexSetBuilder::new(patterns)
        .unicode(true)
        .case_insensitive(false)
        .multi_line(false)
        .build()?;
    Ok(compiled)
}
