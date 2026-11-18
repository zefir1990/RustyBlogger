use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::date::format_unix_date;
use crate::git_history::first_commit_dates;
use crate::model::{BlogSettings, BlogSnapshot, ImportReport, PostRecord, ValidationIssue};
use crate::parser::{is_valid_language_code, parse_post_file};

pub struct SnapshotOptions<'a> {
    pub posts_directory: &'a Path,
    pub media_directory: Option<&'a Path>,
    pub default_language: &'a str,
    pub configured_languages: Vec<String>,
    pub posts_per_page: usize,
    pub site_title: String,
}

pub fn build_snapshot(options: &SnapshotOptions<'_>) -> Result<(BlogSnapshot, ImportReport)> {
    let directory_entries = fs::read_dir(options.posts_directory).with_context(|| {
        format!(
            "cannot read posts directory {}",
            options.posts_directory.display()
        )
    })?;
    let mut post_paths = Vec::new();
    let mut report = ImportReport::default();
    for entry in directory_entries {
        let entry = entry.with_context(|| {
            format!("cannot read entry in {}", options.posts_directory.display())
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("txt") {
            post_paths.push(path);
        } else {
            report.issues.push(ValidationIssue::warning(
                path,
                None,
                "only .txt files are imported from the posts directory",
            ));
        }
    }
    post_paths.sort();
    if post_paths.is_empty() {
        report.issues.push(ValidationIssue::error(
            options.posts_directory,
            None,
            "no .txt posts found",
        ));
    }
    let commit_dates = match first_commit_dates(options.posts_directory) {
        Ok(commit_dates) => commit_dates,
        Err(error) => {
            report.issues.push(ValidationIssue::warning(
                options.posts_directory,
                None,
                format!("{error}; using file modification times"),
            ));
            BTreeMap::new()
        }
    };
    let mut posts = Vec::new();
    let mut known_sources: BTreeMap<(String, String), PathBuf> = BTreeMap::new();
    for path in post_paths {
        match parse_post_file(&path, options.media_directory) {
            Ok(parsed_post) => {
                let key = (
                    parsed_post.metadata.slug.clone(),
                    parsed_post.metadata.language.clone(),
                );
                if let Some(existing_path) = known_sources.get(&key) {
                    report.issues.push(ValidationIssue::error(
                        &path,
                        None,
                        format!(
                            "duplicate post slug \"{}\" for language \"{}\", already defined in {}",
                            parsed_post.metadata.slug,
                            parsed_post.metadata.language,
                            existing_path.display()
                        ),
                    ));
                    report.skipped_files += 1;
                    continue;
                }
                known_sources.insert(key, path.clone());
                report.issues.extend(parsed_post.issues);
                report.accepted_posts += 1;
                let source_path = path
                    .strip_prefix(options.posts_directory)
                    .unwrap_or(path.as_path())
                    .to_string_lossy()
                    .replace('\\', "/");
                let (published_unix_seconds, published_date) = match commit_dates.get(&source_path)
                {
                    Some(commit_date) => (commit_date.timestamp, commit_date.date.clone()),
                    None => {
                        if !commit_dates.is_empty() {
                            report.issues.push(ValidationIssue::warning(
                                &path,
                                None,
                                "no git creation commit found, using file modification time",
                            ));
                        }
                        let file_timestamp = parsed_post.file_modified_unix_seconds;
                        (file_timestamp, format_unix_date(file_timestamp))
                    }
                };
                posts.push(PostRecord {
                    slug: parsed_post.metadata.slug,
                    language: parsed_post.metadata.language,
                    title: parsed_post.metadata.title,
                    categories: parsed_post.metadata.categories,
                    body_html: parsed_post.body_html,
                    source_path,
                    published_unix_seconds,
                    published_date,
                });
            }
            Err(issues) => {
                report.skipped_files += 1;
                report.issues.extend(issues);
            }
        }
    }
    let mut language_counts: BTreeMap<String, usize> = BTreeMap::new();
    for post in &posts {
        *language_counts.entry(post.language.clone()).or_default() += 1;
    }
    let mut valid_configured_languages = Vec::new();
    for configured_language in &options.configured_languages {
        let normalized_language = configured_language.trim().to_ascii_lowercase();
        if normalized_language.is_empty() {
            continue;
        }
        if !is_valid_language_code(&normalized_language) {
            report.issues.push(ValidationIssue::warning(
                options.posts_directory,
                None,
                format!(
                    "configured language \"{configured_language}\" is not a valid language code"
                ),
            ));
            continue;
        }
        language_counts
            .entry(normalized_language.clone())
            .or_insert(0);
        valid_configured_languages.push(normalized_language);
    }
    let default_language = resolve_default_language(
        options.default_language,
        &language_counts,
        options.posts_directory,
        &mut report,
    );
    let snapshot = BlogSnapshot::from_posts(
        posts,
        &valid_configured_languages,
        default_language,
        BlogSettings {
            posts_per_page: options.posts_per_page,
            title: options.site_title.clone(),
        },
    );
    Ok((snapshot, report))
}

fn resolve_default_language(
    requested_default_language: &str,
    language_counts: &BTreeMap<String, usize>,
    posts_directory: &Path,
    report: &mut ImportReport,
) -> String {
    if language_counts.contains_key(requested_default_language) {
        return requested_default_language.to_string();
    }
    if let Some((first_language, _)) = language_counts.iter().next() {
        report.issues.push(ValidationIssue::warning(
            posts_directory,
            None,
            format!(
                "default language \"{requested_default_language}\" has no posts, using \"{first_language}\""
            ),
        ));
        return first_language.clone();
    }
    requested_default_language.to_string()
}
