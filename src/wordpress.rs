use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::html_repair::normalize_html_body;
use crate::model::{BlogSnapshot, PostRecord};
use crate::storage::{BlogRepository, RedbPostStore};

pub struct WordPressImportOptions<'a> {
    pub base_url: &'a str,
    pub database: &'a Path,
    pub slug: &'a str,
    pub languages: &'a [String],
    pub wordpress_default_language: &'a str,
}

#[derive(Debug, Deserialize)]
struct WordPressPost {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    date_gmt: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    title: RenderedField,
    #[serde(default)]
    content: RenderedField,
    #[serde(default)]
    categories: Vec<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct RenderedField {
    #[serde(default)]
    rendered: String,
}

#[derive(Debug, Deserialize)]
struct WordPressCategory {
    id: u64,
    slug: String,
}

pub fn import_localizations(options: &WordPressImportOptions<'_>) -> Result<()> {
    let base_url = options.base_url.trim_end_matches('/');
    let client = Client::builder()
        .user_agent("rustyblogger-wordpress-import/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .context("cannot build WordPress HTTP client")?;
    let categories = match fetch_categories(&client, base_url) {
        Ok(categories) => categories,
        Err(error) => {
            eprintln!("warning: cannot load WordPress categories: {error:#}");
            HashMap::new()
        }
    };
    let mut imported_posts = Vec::new();
    let mut missing_languages = Vec::new();
    for language in options.languages {
        let normalized_language = language.trim().to_ascii_lowercase();
        if normalized_language.is_empty() {
            continue;
        }
        match fetch_post(
            &client,
            base_url,
            &normalized_language,
            options.wordpress_default_language,
            options.slug,
        )? {
            Some(post) => imported_posts.push(to_post_record(
                post,
                options.slug,
                &normalized_language,
                &categories,
            )),
            None => missing_languages.push(normalized_language),
        }
    }
    if imported_posts.is_empty() {
        bail!(
            "no localizations found for slug \"{}\" at {}",
            options.slug,
            options.base_url
        );
    }
    let store = RedbPostStore::open(options.database).with_context(|| {
        format!(
            "cannot open {}; stop the running server before importing",
            options.database.display()
        )
    })?;
    let mut posts = load_existing_posts(&store)?;
    let configured_languages: Vec<String> = store
        .list_languages()?
        .into_iter()
        .map(|language| language.code)
        .collect();
    let imported_count = imported_posts.len();
    let mut updated_count = 0;
    let mut added_count = 0;
    for imported_post in imported_posts {
        if let Some(existing_post) = posts
            .iter_mut()
            .find(|post| post.slug == imported_post.slug && post.language == imported_post.language)
        {
            existing_post.title = imported_post.title;
            existing_post.body_html = imported_post.body_html;
            updated_count += 1;
        } else {
            posts.push(imported_post);
            added_count += 1;
        }
    }
    let mut default_language = store.default_language()?;
    if !posts.iter().any(|post| post.language == default_language) {
        if let Some(first_post) = posts.first() {
            default_language = first_post.language.clone();
        }
    }
    let settings = store.settings()?;
    let snapshot =
        BlogSnapshot::from_posts(posts, &configured_languages, default_language, settings);
    store.replace_snapshot(&snapshot)?;
    println!(
        "imported {imported_count} localizations for \"{}\" into {} ({updated_count} updated, {added_count} added)",
        options.slug,
        options.database.display()
    );
    if !missing_languages.is_empty() {
        println!("no translation found for: {}", missing_languages.join(", "));
    }
    Ok(())
}

fn fetch_post(
    client: &Client,
    base_url: &str,
    language: &str,
    wordpress_default_language: &str,
    slug: &str,
) -> Result<Option<WordPressPost>> {
    let endpoint = if language == wordpress_default_language {
        format!("{base_url}/wp-json/wp/v2/posts")
    } else {
        format!("{base_url}/{language}/wp-json/wp/v2/posts")
    };
    let response = client
        .get(endpoint)
        .query(&[("slug", slug), ("per_page", "1")])
        .send()
        .with_context(|| format!("cannot request WordPress post \"{slug}\" in {language}"))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        bail!(
            "WordPress returned {} for slug \"{}\" in {}",
            response.status(),
            slug,
            language
        );
    }
    let posts: Vec<WordPressPost> = response
        .json()
        .with_context(|| format!("cannot parse WordPress post \"{slug}\" in {language}"))?;
    Ok(posts.into_iter().next())
}

fn fetch_categories(client: &Client, base_url: &str) -> Result<HashMap<u64, String>> {
    let response = client
        .get(format!("{base_url}/wp-json/wp/v2/categories"))
        .query(&[("per_page", "100"), ("_fields", "id,slug")])
        .send()
        .context("cannot request WordPress categories")?;
    if !response.status().is_success() {
        bail!("WordPress returned {} for categories", response.status());
    }
    let categories: Vec<WordPressCategory> = response
        .json()
        .context("cannot parse WordPress categories")?;
    Ok(categories
        .into_iter()
        .map(|category| (category.id, category.slug.to_ascii_lowercase()))
        .collect())
}

fn to_post_record(
    post: WordPressPost,
    requested_slug: &str,
    language: &str,
    categories: &HashMap<u64, String>,
) -> PostRecord {
    let slug = if post.slug.trim().is_empty() {
        requested_slug
    } else {
        post.slug.trim()
    };
    let mut seen_categories = HashSet::new();
    let post_categories = post
        .categories
        .iter()
        .filter_map(|category_id| categories.get(category_id))
        .filter(|category| seen_categories.insert((*category).clone()))
        .cloned()
        .collect();
    let (published_unix_seconds, published_date) = wordpress_date(&post.date_gmt, &post.date);
    PostRecord {
        slug: slug.to_string(),
        language: language.to_string(),
        title: decode_html_entities(post.title.rendered.trim()),
        categories: post_categories,
        body_html: normalize_html_body(&post.content.rendered),
        source_path: post.link,
        published_unix_seconds,
        published_date,
    }
}

fn load_existing_posts(store: &RedbPostStore) -> Result<Vec<PostRecord>> {
    let mut posts = Vec::new();
    for slug_record in store.list_slugs()? {
        for language in &slug_record.languages {
            if let Some(post) = store.get_post(&slug_record.slug, language)? {
                posts.push(post);
            }
        }
    }
    Ok(posts)
}

fn wordpress_date(date_gmt: &str, date: &str) -> (u64, String) {
    let published_date = date
        .get(..10)
        .filter(|short_date| short_date.len() == 10)
        .unwrap_or(date)
        .to_string();
    let timestamp = parse_iso_utc(date_gmt)
        .or_else(|| parse_iso_utc(date))
        .unwrap_or(0);
    (timestamp, published_date)
}

fn parse_iso_utc(value: &str) -> Option<u64> {
    if value.len() < 19 {
        return None;
    }
    let year = value.get(0..4)?.parse::<i64>().ok()?;
    let month = value.get(5..7)?.parse::<i64>().ok()?;
    let day = value.get(8..10)?.parse::<i64>().ok()?;
    let hour = value.get(11..13)?.parse::<i64>().ok()?;
    let minute = value.get(14..16)?.parse::<i64>().ok()?;
    let second = value.get(17..19)?.parse::<i64>().ok()?;
    let total_seconds =
        days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second;
    u64::try_from(total_seconds).ok()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_part = (month + 9) % 12;
    let day_of_year = (153 * month_part + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn decode_html_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(entity_start) = remaining.find('&') {
        decoded.push_str(&remaining[..entity_start]);
        let after_ampersand = &remaining[entity_start + 1..];
        let Some(entity_end) = after_ampersand.find(';') else {
            decoded.push('&');
            remaining = after_ampersand;
            continue;
        };
        let entity = &after_ampersand[..entity_end];
        if let Some(character) = decode_entity(entity) {
            decoded.push(character);
            remaining = &after_ampersand[entity_end + 1..];
        } else {
            decoded.push('&');
            remaining = after_ampersand;
        }
    }
    decoded.push_str(remaining);
    decoded
}

fn decode_entity(entity: &str) -> Option<char> {
    if let Some(hexadecimal) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hexadecimal, 16)
            .ok()
            .and_then(char::from_u32);
    }
    if let Some(decimal) = entity.strip_prefix('#') {
        return decimal.parse::<u32>().ok().and_then(char::from_u32);
    }
    match entity {
        "amp" => Some('&'),
        "apos" => Some('\''),
        "copy" => Some('©'),
        "gt" => Some('>'),
        "hellip" => Some('…'),
        "laquo" => Some('«'),
        "ldquo" => Some('“'),
        "lsquo" => Some('‘'),
        "lt" => Some('<'),
        "mdash" => Some('—'),
        "nbsp" => Some('\u{a0}'),
        "ndash" => Some('–'),
        "quot" => Some('"'),
        "raquo" => Some('»'),
        "rdquo" => Some('”'),
        "reg" => Some('®'),
        "rsquo" => Some('’'),
        "trade" => Some('™'),
        _ => None,
    }
}
