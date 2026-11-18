use serde::{Deserialize, Serialize};

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
