use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

impl BlogSnapshot {
    pub fn from_posts(
        mut posts: Vec<PostRecord>,
        configured_languages: &[String],
        default_language: String,
        settings: BlogSettings,
    ) -> Self {
        let mut language_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut languages_by_slug: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for post in &posts {
            *language_counts.entry(post.language.clone()).or_default() += 1;
            languages_by_slug
                .entry(post.slug.clone())
                .or_default()
                .insert(post.language.clone());
        }
        for configured_language in configured_languages {
            let normalized_language = configured_language.trim().to_ascii_lowercase();
            if !normalized_language.is_empty() {
                language_counts.entry(normalized_language).or_insert(0);
            }
        }
        let mut languages: Vec<LanguageRecord> = language_counts
            .into_iter()
            .map(|(code, post_count)| LanguageRecord { code, post_count })
            .collect();
        languages.sort_by(|left, right| {
            (left.code != default_language, &left.code)
                .cmp(&(right.code != default_language, &right.code))
        });
        let mut slugs = Vec::new();
        for (slug, language_set) in languages_by_slug {
            let mut slug_languages: Vec<String> = language_set.into_iter().collect();
            let slug_default_language = if slug_languages.contains(&default_language) {
                default_language.clone()
            } else {
                slug_languages.first().cloned().unwrap_or_default()
            };
            let default_language_reference = slug_default_language.clone();
            slug_languages.sort_by(|left, right| {
                (left != &default_language_reference, left)
                    .cmp(&(right != &default_language_reference, right))
            });
            slugs.push(SlugRecord {
                slug,
                languages: slug_languages,
                default_language: slug_default_language,
            });
        }
        posts.sort_by(|left, right| {
            right
                .published_unix_seconds
                .cmp(&left.published_unix_seconds)
                .then_with(|| left.slug.cmp(&right.slug))
        });
        let summaries: Vec<PostSummary> = posts.iter().map(PostRecord::summary).collect();
        let imported_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        Self {
            posts,
            summaries,
            slugs,
            languages,
            settings: settings.normalized(),
            default_language,
            imported_unix_seconds,
        }
    }
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
