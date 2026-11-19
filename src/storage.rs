use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use redb::{Database, ReadableTable, TableDefinition};

use crate::model::{BlogSettings, LanguageRecord, PostRecord, PostSummary, SlugRecord};

const POSTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("posts");
const SLUGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("slugs");
const LANGUAGE_INDEX_TABLE: TableDefinition<&str, &str> = TableDefinition::new("language_index");
const LANGUAGES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("languages");
const METADATA_TABLE: TableDefinition<&str, &str> = TableDefinition::new("metadata");
const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");

const VARIANT_SEPARATOR: char = '\u{1f}';
const DEFAULT_LANGUAGE_KEY: &str = "default_language";
const BLOG_SETTINGS_KEY: &str = "blog_settings";

pub trait BlogRepository: Send + Sync {
    fn list_languages(&self) -> Result<Vec<LanguageRecord>>;
    fn default_language(&self) -> Result<String>;
    fn list_slugs(&self) -> Result<Vec<SlugRecord>>;
    fn get_slug(&self, slug: &str) -> Result<Option<SlugRecord>>;
    fn get_post(&self, slug: &str, language: &str) -> Result<Option<PostRecord>>;
    fn upsert_post(&self, post: &PostRecord) -> Result<()>;
    fn delete_post(&self, slug: &str, language: &str) -> Result<bool>;
    fn list_posts(&self, language: &str) -> Result<Vec<PostSummary>>;
    fn settings(&self) -> Result<BlogSettings>;
    fn check_health(&self) -> Result<()>;
}

pub struct RedbPostStore {
    database: Database,
}

impl RedbPostStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent_directory) = path.parent() {
            if !parent_directory.as_os_str().is_empty() {
                fs::create_dir_all(parent_directory).with_context(|| {
                    format!(
                        "cannot create database directory {}",
                        parent_directory.display()
                    )
                })?;
            }
        }
        let database = Database::create(path)
            .with_context(|| format!("cannot open redb database {}", path.display()))?;
        {
            let write_transaction = database.begin_write()?;
            {
                write_transaction.open_table(POSTS_TABLE)?;
                write_transaction.open_table(SLUGS_TABLE)?;
                write_transaction.open_table(LANGUAGE_INDEX_TABLE)?;
                write_transaction.open_table(LANGUAGES_TABLE)?;
                write_transaction.open_table(METADATA_TABLE)?;
                write_transaction.open_table(SETTINGS_TABLE)?;
            }
            write_transaction.commit()?;
        }
        Ok(Self { database })
    }
}

impl BlogRepository for RedbPostStore {
    fn list_languages(&self) -> Result<Vec<LanguageRecord>> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(LANGUAGES_TABLE)?;
        let mut languages = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            languages.push(serde_json::from_str(value.value())?);
        }
        Ok(languages)
    }

    fn default_language(&self) -> Result<String> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(METADATA_TABLE)?;
        match table.get(DEFAULT_LANGUAGE_KEY)? {
            Some(value) => Ok(value.value().to_string()),
            None => Ok("ru".to_string()),
        }
    }

    fn list_slugs(&self) -> Result<Vec<SlugRecord>> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(SLUGS_TABLE)?;
        let mut slugs: Vec<SlugRecord> = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            slugs.push(serde_json::from_str(value.value())?);
        }
        slugs.sort_by(|left, right| left.slug.cmp(&right.slug));
        Ok(slugs)
    }

    fn get_slug(&self, slug: &str) -> Result<Option<SlugRecord>> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(SLUGS_TABLE)?;
        match table.get(slug)? {
            Some(value) => Ok(Some(serde_json::from_str(value.value())?)),
            None => Ok(None),
        }
    }

    fn get_post(&self, slug: &str, language: &str) -> Result<Option<PostRecord>> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(POSTS_TABLE)?;
        let key = variant_key(slug, language);
        match table.get(key.as_str())? {
            Some(value) => Ok(Some(serde_json::from_str(value.value())?)),
            None => Ok(None),
        }
    }

    fn upsert_post(&self, post: &PostRecord) -> Result<()> {
        let post_json = serde_json::to_string(post)?;
        let summary_json = serde_json::to_string(&post.summary())?;
        let write_transaction = self.database.begin_write()?;
        let post_key = variant_key(&post.slug, &post.language);
        let is_new_post = {
            let posts = write_transaction.open_table(POSTS_TABLE)?;
            let existing_post = posts.get(post_key.as_str())?;
            existing_post.is_none()
        };
        {
            let mut posts = write_transaction.open_table(POSTS_TABLE)?;
            posts.insert(post_key.as_str(), post_json.as_str())?;
        }
        {
            let mut slugs = write_transaction.open_table(SLUGS_TABLE)?;
            let existing_slug_json = slugs
                .get(post.slug.as_str())?
                .map(|value| value.value().to_string());
            let mut slug_record = match existing_slug_json {
                Some(json) => serde_json::from_str::<SlugRecord>(&json)?,
                None => SlugRecord {
                    slug: post.slug.clone(),
                    languages: Vec::new(),
                    default_language: post.language.clone(),
                },
            };
            if !slug_record
                .languages
                .iter()
                .any(|language| language == &post.language)
            {
                slug_record.languages.push(post.language.clone());
                slug_record.languages.sort();
            }
            if slug_record.default_language.is_empty() {
                slug_record.default_language = post.language.clone();
            }
            let slug_json = serde_json::to_string(&slug_record)?;
            slugs.insert(post.slug.as_str(), slug_json.as_str())?;
        }
        {
            let mut language_index = write_transaction.open_table(LANGUAGE_INDEX_TABLE)?;
            let index_key = language_index_key(&post.language, &post.slug);
            language_index.insert(index_key.as_str(), summary_json.as_str())?;
        }
        {
            let mut languages = write_transaction.open_table(LANGUAGES_TABLE)?;
            let existing_language_json = languages
                .get(post.language.as_str())?
                .map(|value| value.value().to_string());
            let mut language_record = match existing_language_json {
                Some(json) => serde_json::from_str::<LanguageRecord>(&json)?,
                None => LanguageRecord {
                    code: post.language.clone(),
                    post_count: 0,
                },
            };
            if is_new_post {
                language_record.post_count += 1;
            }
            let language_json = serde_json::to_string(&language_record)?;
            languages.insert(post.language.as_str(), language_json.as_str())?;
        }
        write_transaction.commit()?;
        Ok(())
    }

    fn delete_post(&self, slug: &str, language: &str) -> Result<bool> {
        let write_transaction = self.database.begin_write()?;
        let post_key = variant_key(slug, language);
        let existed = {
            let mut posts = write_transaction.open_table(POSTS_TABLE)?;
            let removed_post = posts.remove(post_key.as_str())?;
            removed_post.is_some()
        };
        if !existed {
            return Ok(false);
        }
        {
            let mut language_index = write_transaction.open_table(LANGUAGE_INDEX_TABLE)?;
            let index_key = language_index_key(language, slug);
            language_index.remove(index_key.as_str())?;
        }
        {
            let mut languages = write_transaction.open_table(LANGUAGES_TABLE)?;
            let existing_language_json = languages
                .get(language)?
                .map(|value| value.value().to_string());
            if let Some(json) = existing_language_json {
                let mut language_record: LanguageRecord = serde_json::from_str(&json)?;
                language_record.post_count = language_record.post_count.saturating_sub(1);
                let language_json = serde_json::to_string(&language_record)?;
                languages.insert(language, language_json.as_str())?;
            }
        }
        {
            let mut slugs = write_transaction.open_table(SLUGS_TABLE)?;
            let existing_slug_json = slugs.get(slug)?.map(|value| value.value().to_string());
            if let Some(json) = existing_slug_json {
                let mut slug_record: SlugRecord = serde_json::from_str(&json)?;
                slug_record.languages.retain(|code| code != language);
                if slug_record.languages.is_empty() {
                    slugs.remove(slug)?;
                } else {
                    if slug_record.default_language == language {
                        if let Some(first_language) = slug_record.languages.first() {
                            slug_record.default_language = first_language.clone();
                        }
                    }
                    let slug_json = serde_json::to_string(&slug_record)?;
                    slugs.insert(slug, slug_json.as_str())?;
                }
            }
        }
        write_transaction.commit()?;
        Ok(true)
    }

    fn list_posts(&self, language: &str) -> Result<Vec<PostSummary>> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(LANGUAGE_INDEX_TABLE)?;
        let prefix = format!("{language}{VARIANT_SEPARATOR}");
        let mut summaries: Vec<PostSummary> = Vec::new();
        for entry in table.range::<&str>(prefix.as_str()..)? {
            let (key, value) = entry?;
            if !key.value().starts_with(prefix.as_str()) {
                break;
            }
            summaries.push(serde_json::from_str(value.value())?);
        }
        summaries.sort_by(|left, right| {
            right
                .published_unix_seconds
                .cmp(&left.published_unix_seconds)
                .then_with(|| left.slug.cmp(&right.slug))
        });
        Ok(summaries)
    }

    fn settings(&self) -> Result<BlogSettings> {
        let read_transaction = self.database.begin_read()?;
        let table = read_transaction.open_table(SETTINGS_TABLE)?;
        match table.get(BLOG_SETTINGS_KEY)? {
            Some(value) => Ok(serde_json::from_str(value.value())?),
            None => Ok(BlogSettings::default()),
        }
    }

    fn check_health(&self) -> Result<()> {
        let read_transaction = self.database.begin_read()?;
        let _ = read_transaction.open_table(METADATA_TABLE)?;
        let _ = read_transaction.open_table(SETTINGS_TABLE)?;
        Ok(())
    }
}

fn variant_key(slug: &str, language: &str) -> String {
    format!("{slug}{VARIANT_SEPARATOR}{language}")
}

fn language_index_key(language: &str, slug: &str) -> String {
    format!("{language}{VARIANT_SEPARATOR}{slug}")
}
