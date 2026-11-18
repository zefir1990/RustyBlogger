use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::model::{ParsedPost, PostMetadata, ValidationIssue};
use crate::render::render_body;

pub const SUPPORTED_FORMAT_CODE: &str = "Fall24-October10";
pub const ALLOWED_CATEGORIES: [&str; 2] = ["software", "notes"];

pub fn parse_post_file(
    path: &Path,
    media_directory: Option<&Path>,
) -> Result<ParsedPost, Vec<ValidationIssue>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(vec![ValidationIssue::error(
                path,
                None,
                format!("cannot read file: {error}"),
            )]);
        }
    };
    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(error) => {
            return Err(vec![ValidationIssue::error(
                path,
                None,
                format!("file is not valid UTF-8: {error}"),
            )]);
        }
    };
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    if lines.len() < 6 {
        return Err(vec![ValidationIssue::error(
            path,
            None,
            "post must contain five metadata lines and a body",
        )]);
    }
    let mut issues = Vec::new();
    for (index, line) in lines.iter().take(5).enumerate() {
        if line.trim_end() != *line {
            issues.push(ValidationIssue::warning(
                path,
                Some(index + 1),
                "trailing whitespace in metadata line",
            ));
        }
    }
    let format_code = parse_value_line(path, lines[0], 1, "Format: ", "Format", &mut issues);
    if let Some(format_code) = &format_code {
        if format_code != SUPPORTED_FORMAT_CODE {
            issues.push(ValidationIssue::error(
                path,
                Some(1),
                format!(
                    "unsupported format code \"{format_code}\", expected \"{SUPPORTED_FORMAT_CODE}\""
                ),
            ));
        }
    }
    let language = parse_value_line(path, lines[1], 2, "Language: ", "Language", &mut issues)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !language.is_empty() && !is_valid_language_code(&language) {
        issues.push(ValidationIssue::error(
            path,
            Some(2),
            format!("invalid language code \"{language}\""),
        ));
    }
    let title =
        parse_value_line(path, lines[2], 3, "Title: ", "Title", &mut issues).unwrap_or_default();
    let title = title
        .strip_prefix("[DO NOT PROCESS LINE]")
        .unwrap_or(&title)
        .trim()
        .to_string();
    if title.is_empty() {
        issues.push(ValidationIssue::error(
            path,
            Some(3),
            "title must not be empty",
        ));
    }
    let slug = parse_value_line(path, lines[3], 4, "Slug: ", "Slug", &mut issues)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !slug.is_empty() && !is_valid_slug(&slug) {
        issues.push(ValidationIssue::error(
            path,
            Some(4),
            format!("invalid slug \"{slug}\", only lowercase latin letters, digits and hyphens are allowed"),
        ));
    }
    let categories = parse_categories(path, lines[4], &mut issues);
    if format_code.is_none()
        || language.is_empty()
        || title.is_empty()
        || slug.is_empty()
        || categories.is_empty()
    {
        return Err(issues);
    }
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if file_stem != slug && file_stem != format!("{slug}.{language}") {
        issues.push(ValidationIssue::warning(
            path,
            Some(4),
            format!("file name \"{file_stem}\" does not match slug \"{slug}\""),
        ));
    }
    let body_lines = &lines[5..];
    if body_lines.iter().all(|line| line.trim().is_empty()) {
        issues.push(ValidationIssue::error(
            path,
            Some(6),
            "post body must not be empty",
        ));
        return Err(issues);
    }
    let rendered_body = render_body(body_lines, &language, path);
    issues.extend(rendered_body.issues);
    if rendered_body.html.trim().is_empty() {
        issues.push(ValidationIssue::error(
            path,
            Some(6),
            "post body produced no rendered content",
        ));
    }
    if let Some(media_directory) = media_directory {
        for media_file in &rendered_body.referenced_media {
            if !media_directory.join(media_file).is_file() {
                issues.push(ValidationIssue::error(
                    path,
                    None,
                    format!(
                        "referenced media file \"{media_file}\" is missing in {}",
                        media_directory.display()
                    ),
                ));
            }
        }
    }
    if issues
        .iter()
        .any(|issue| issue.severity == crate::model::IssueSeverity::Error)
    {
        return Err(issues);
    }
    let metadata = PostMetadata {
        format_code: format_code.unwrap_or_default(),
        language,
        title,
        slug,
        categories,
    };
    Ok(ParsedPost {
        metadata,
        body_html: rendered_body.html,
        modified_unix_seconds: modified_unix_seconds(path),
        issues,
    })
}

fn parse_value_line(
    path: &Path,
    line: &str,
    line_number: usize,
    prefix: &str,
    field_name: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<String> {
    let trimmed_line = line.trim_end();
    match trimmed_line.strip_prefix(prefix) {
        Some(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                issues.push(ValidationIssue::error(
                    path,
                    Some(line_number),
                    format!("{field_name} line must not be empty"),
                ));
            }
            Some(value)
        }
        None => {
            issues.push(ValidationIssue::error(
                path,
                Some(line_number),
                format!("{field_name} line must start with \"{prefix}\""),
            ));
            None
        }
    }
}

fn parse_categories(path: &Path, line: &str, issues: &mut Vec<ValidationIssue>) -> Vec<String> {
    let Some(value) = line.trim_end().strip_prefix("Categories: ") else {
        issues.push(ValidationIssue::error(
            path,
            Some(5),
            "Categories line must start with \"Categories: \"",
        ));
        return Vec::new();
    };
    let categories: Vec<String> = value
        .split(',')
        .map(|category| category.trim().to_string())
        .filter(|category| !category.is_empty())
        .collect();
    if categories.is_empty() {
        issues.push(ValidationIssue::error(
            path,
            Some(5),
            "at least one category is required",
        ));
    }
    for category in &categories {
        if !ALLOWED_CATEGORIES.contains(&category.as_str()) {
            issues.push(ValidationIssue::warning(
                path,
                Some(5),
                format!(
                    "category \"{category}\" is not listed in AGENTS.md, expected software or notes"
                ),
            ));
        }
    }
    categories
}

pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        })
}

pub fn is_valid_language_code(language: &str) -> bool {
    let mut parts = language.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&primary.len())
        || !primary
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return false;
    }
    parts.all(|part| {
        (2..=8).contains(&part.len())
            && part
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    })
}

fn modified_unix_seconds(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
