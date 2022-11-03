use std::collections::HashMap;

use regex::{RegexSet, RegexSetBuilder};
use serde::Deserialize;

/// Configuration file
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,

    #[serde(rename = "dir")]
    pub directories: HashMap<String, DirectoryConfig>,
}

/// General application settings
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    pub color: bool,
}

/// Configuration for a directory
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DirectoryConfig {
    pub recursive: bool,

    pub allowed_dirs: Vec<MatchRule>,
    pub allowed_files: Vec<MatchRule>,
}

/// A rule to check if the filename matches
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MatchRule {
    Name { name: String },
    Extension { ext: String },
    Pattern { pattern: String },
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
