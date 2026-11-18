use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use redb::{Database, ReadableTable, TableDefinition};

use crate::model::{
    BlogSettings, BlogSnapshot, LanguageRecord, PostRecord, PostSummary, SlugRecord,
};

const POSTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("posts");
const SLUGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("slugs");
const LANGUAGE_INDEX_TABLE: TableDefinition<&str, &str> = TableDefinition::new("language_index");
const LANGUAGES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("languages");
const METADATA_TABLE: TableDefinition<&str, &str> = TableDefinition::new("metadata");
const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");

const VARIANT_SEPARATOR: char = '\u{1f}';
const DEFAULT_LANGUAGE_KEY: &str = "default_language";
const IMPORTED_AT_KEY: &str = "imported_unix_seconds";
const BLOG_SETTINGS_KEY: &str = "blog_settings";

pub trait BlogRepository: Send + Sync {
    fn list_languages(&self) -> Result<Vec<LanguageRecord>>;
    fn default_language(&self) -> Result<String>;
    fn list_slugs(&self) -> Result<Vec<SlugRecord>>;
    fn get_slug(&self, slug: &str) -> Result<Option<SlugRecord>>;
    fn get_post(&self, slug: &str, language: &str) -> Result<Option<PostRecord>>;
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

    pub fn replace_snapshot(&self, snapshot: &BlogSnapshot) -> Result<()> {
        let write_transaction = self.database.begin_write()?;
        {
            write_transaction.delete_table(POSTS_TABLE)?;
            write_transaction.delete_table(SLUGS_TABLE)?;
            write_transaction.delete_table(LANGUAGE_INDEX_TABLE)?;
            write_transaction.delete_table(LANGUAGES_TABLE)?;
            write_transaction.delete_table(METADATA_TABLE)?;
            write_transaction.delete_table(SETTINGS_TABLE)?;
            let mut posts_table = write_transaction.open_table(POSTS_TABLE)?;
            let mut slugs_table = write_transaction.open_table(SLUGS_TABLE)?;
            let mut language_index_table = write_transaction.open_table(LANGUAGE_INDEX_TABLE)?;
            let mut languages_table = write_transaction.open_table(LANGUAGES_TABLE)?;
            let mut metadata_table = write_transaction.open_table(METADATA_TABLE)?;
            let mut settings_table = write_transaction.open_table(SETTINGS_TABLE)?;
            for post in &snapshot.posts {
                let key = variant_key(&post.slug, &post.language);
                let serialized_post = serde_json::to_string(post)?;
                posts_table.insert(key.as_str(), serialized_post.as_str())?;
            }
            for summary in &snapshot.summaries {
                let index_key = language_index_key(&summary.language, &summary.slug);
                let serialized_index_summary = serde_json::to_string(summary)?;
                language_index_table
                    .insert(index_key.as_str(), serialized_index_summary.as_str())?;
            }
            for slug in &snapshot.slugs {
                let serialized_slug = serde_json::to_string(slug)?;
                slugs_table.insert(slug.slug.as_str(), serialized_slug.as_str())?;
            }
            for language in &snapshot.languages {
                let serialized_language = serde_json::to_string(language)?;
                languages_table.insert(language.code.as_str(), serialized_language.as_str())?;
            }
            metadata_table.insert(DEFAULT_LANGUAGE_KEY, snapshot.default_language.as_str())?;
            let imported_at = snapshot.imported_unix_seconds.to_string();
            metadata_table.insert(IMPORTED_AT_KEY, imported_at.as_str())?;
            let serialized_settings = serde_json::to_string(&snapshot.settings)?;
            settings_table.insert(BLOG_SETTINGS_KEY, serialized_settings.as_str())?;
        }
        write_transaction.commit()?;
        Ok(())
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
