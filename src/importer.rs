use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::model::{
    BlogSettings, BlogSnapshot, ImportReport, LanguageRecord, PostRecord, PostSummary, SlugRecord,
    ValidationIssue,
};
use crate::parser::{is_valid_language_code, parse_post_file};

pub struct SnapshotOptions<'a> {
    pub posts_directory: &'a Path,
    pub media_directory: Option<&'a Path>,
    pub default_language: &'a str,
    pub configured_languages: Vec<String>,
    pub posts_per_page: usize,
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
                    .to_string();
                posts.push(PostRecord {
                    slug: parsed_post.metadata.slug,
                    language: parsed_post.metadata.language,
                    title: parsed_post.metadata.title,
                    categories: parsed_post.metadata.categories,
                    body_html: parsed_post.body_html,
                    source_path,
                    modified_unix_seconds: parsed_post.modified_unix_seconds,
                });
            }
            Err(issues) => {
                report.skipped_files += 1;
                report.issues.extend(issues);
            }
        }
    }
    let mut language_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut languages_by_slug: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for post in &posts {
        *language_counts.entry(post.language.clone()).or_default() += 1;
        languages_by_slug
            .entry(post.slug.clone())
            .or_default()
            .insert(post.language.clone());
    }
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
        language_counts.entry(normalized_language).or_insert(0);
    }
    let default_language = resolve_default_language(
        options.default_language,
        &language_counts,
        options.posts_directory,
        &mut report,
    );
    let mut languages: Vec<LanguageRecord> = language_counts
        .into_iter()
        .map(|(code, post_count)| LanguageRecord { code, post_count })
        .collect();
    languages.sort_by(|left, right| compare_languages(left, right, &default_language));
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
            .modified_unix_seconds
            .cmp(&left.modified_unix_seconds)
            .then_with(|| left.slug.cmp(&right.slug))
    });
    let summaries: Vec<PostSummary> = posts.iter().map(PostRecord::summary).collect();
    let imported_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok((
        BlogSnapshot {
            posts,
            summaries,
            slugs,
            languages,
            settings: BlogSettings {
                posts_per_page: options.posts_per_page,
            }
            .normalized(),
            default_language,
            imported_unix_seconds,
        },
        report,
    ))
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

fn compare_languages(
    left: &LanguageRecord,
    right: &LanguageRecord,
    default_language: &str,
) -> std::cmp::Ordering {
    (left.code != default_language, &left.code).cmp(&(right.code != default_language, &right.code))
}
