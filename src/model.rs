use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

impl fmt::Display for IssueSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueSeverity::Error => formatter.write_str("error"),
            IssueSeverity::Warning => formatter.write_str("warning"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(
        path: impl Into<PathBuf>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: IssueSeverity::Error,
            path: path.into(),
            line,
            message: message.into(),
        }
    }

    pub fn warning(
        path: impl Into<PathBuf>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            path: path.into(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(
                formatter,
                "{} {}:{}: {}",
                self.severity,
                self.path.display(),
                line,
                self.message
            ),
            None => write!(
                formatter,
                "{} {}: {}",
                self.severity,
                self.path.display(),
                self.message
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetadata {
    pub format_code: String,
    pub language: String,
    pub title: String,
    pub slug: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedPost {
    pub metadata: PostMetadata,
    pub body_html: String,
    pub file_modified_unix_seconds: u64,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRecord {
    pub slug: String,
    pub language: String,
    pub title: String,
    pub categories: Vec<String>,
    pub body_html: String,
    pub source_path: String,
    pub published_unix_seconds: u64,
    pub published_date: String,
}

impl PostRecord {
    pub fn summary(&self) -> PostSummary {
        PostSummary {
            slug: self.slug.clone(),
            language: self.language.clone(),
            title: self.title.clone(),
            categories: self.categories.clone(),
            published_unix_seconds: self.published_unix_seconds,
            published_date: self.published_date.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSummary {
    pub slug: String,
    pub language: String,
    pub title: String,
    pub categories: Vec<String>,
    pub published_unix_seconds: u64,
    pub published_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlugRecord {
    pub slug: String,
    pub languages: Vec<String>,
    pub default_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageRecord {
    pub code: String,
    pub post_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogSettings {
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default = "default_site_title")]
    pub title: String,
}

fn default_posts_per_page() -> usize {
    10
}

fn default_site_title() -> String {
    "Zefir1990".to_string()
}

impl Default for BlogSettings {
    fn default() -> Self {
        Self {
            posts_per_page: default_posts_per_page(),
            title: default_site_title(),
        }
    }
}

impl BlogSettings {
    pub fn normalized(self) -> Self {
        Self {
            posts_per_page: self.posts_per_page.max(1),
            title: self.title,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlogSnapshot {
    pub posts: Vec<PostRecord>,
    pub summaries: Vec<PostSummary>,
    pub slugs: Vec<SlugRecord>,
    pub languages: Vec<LanguageRecord>,
    pub settings: BlogSettings,
    pub default_language: String,
    pub imported_unix_seconds: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub accepted_posts: usize,
    pub skipped_files: usize,
    pub issues: Vec<ValidationIssue>,
}

impl ImportReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Warning)
            .count()
    }

    pub fn print(&self) {
        for issue in &self.issues {
            eprintln!("{issue}");
        }
        println!(
            "accepted posts: {}, skipped files: {}, errors: {}, warnings: {}",
            self.accepted_posts,
            self.skipped_files,
            self.error_count(),
            self.warning_count()
        );
    }
}
