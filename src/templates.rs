use std::fmt::Write as _;

use crate::model::{LanguageRecord, PostRecord, PostSummary};
use crate::render::escape_html;

pub const STYLESHEET: &str = r#"
:root {
  color-scheme: light dark;
  --background: #f7f5f0;
  --surface: #ffffff;
  --text: #20201d;
  --muted: #6b6a63;
  --border: #d9d5cc;
  --accent: #9a3412;
  --accent-soft: #fde8dc;
  --code-background: #1f2328;
  --code-text: #f0f0ea;
}
@media (prefers-color-scheme: dark) {
  :root {
    --background: #16171a;
    --surface: #1f2126;
    --text: #ecebe6;
    --muted: #a5a49c;
    --border: #33363d;
    --accent: #fb923c;
    --accent-soft: #33241a;
    --code-background: #0d1014;
    --code-text: #e6e6df;
  }
}
* { box-sizing: border-box; }
html { font-size: 17px; }
body {
  margin: 0;
  background: var(--background);
  color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  line-height: 1.65;
}
a { color: var(--accent); }
.site-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  max-width: 960px;
  margin: 0 auto;
  padding: 1.25rem 1.25rem 0.75rem;
}
.site-title {
  font-weight: 700;
  font-size: 1.2rem;
  text-decoration: none;
  color: var(--text);
}
.language-switcher { display: flex; flex-wrap: wrap; gap: 0.35rem; }
.language-switcher a {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.15rem 0.6rem;
  font-size: 0.8rem;
  text-decoration: none;
  color: var(--muted);
  background: var(--surface);
}
.language-switcher a.current {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}
.language-switcher a.unavailable { opacity: 0.55; }
main {
  max-width: 820px;
  margin: 0 auto;
  padding: 1rem 1.25rem 3rem;
}
h1 { line-height: 1.25; font-size: 2rem; margin: 0.5rem 0 1rem; }
h2 { line-height: 1.3; margin-top: 2rem; font-size: 1.45rem; }
.post-list { list-style: none; padding: 0; margin: 0; display: grid; gap: 0.9rem; }
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 1rem 1.15rem;
}
.card-title { font-size: 1.15rem; font-weight: 650; text-decoration: none; color: var(--text); }
.card-title:hover { color: var(--accent); }
.card-meta, .post-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
  margin-top: 0.5rem;
  font-size: 0.82rem;
  color: var(--muted);
}
.badge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.05rem 0.5rem;
  background: var(--background);
}
.badge.category { background: var(--accent-soft); border-color: transparent; color: var(--accent); }
.notice {
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  border-radius: 8px;
  background: var(--accent-soft);
  padding: 0.7rem 0.9rem;
  margin: 1rem 0;
  font-size: 0.92rem;
}
.breadcrumbs { font-size: 0.9rem; margin-bottom: 0.5rem; }
.post-body { overflow-wrap: anywhere; }
.post-body p { margin: 0.8rem 0; }
.post-body figure { margin: 1.4rem 0; }
.post-body img, .post-body iframe, .post-body video {
  max-width: 100%;
  height: auto;
  border-radius: 10px;
}
.post-body pre {
  background: var(--code-background);
  color: var(--code-text);
  border-radius: 10px;
  padding: 0.9rem 1rem;
  overflow-x: auto;
  font-size: 0.88rem;
  line-height: 1.5;
}
.post-body code {
  background: var(--accent-soft);
  border-radius: 4px;
  padding: 0.05rem 0.3rem;
  font-size: 0.9em;
}
.post-body pre code { background: transparent; padding: 0; font-size: inherit; }
.post-body .post-link { margin: 0.5rem 0; }
footer {
  max-width: 820px;
  margin: 0 auto;
  padding: 1rem 1.25rem 2.5rem;
  color: var(--muted);
  font-size: 0.85rem;
}
@media (max-width: 560px) {
  html { font-size: 16px; }
  h1 { font-size: 1.6rem; }
}
"#;

pub struct HomeItem {
    pub summary: PostSummary,
    pub languages: Vec<String>,
    pub is_fallback: bool,
}

pub struct HomePage {
    pub site_title: String,
    pub selected_language: String,
    pub default_language: String,
    pub languages: Vec<LanguageRecord>,
    pub items: Vec<HomeItem>,
    pub category: Option<String>,
    pub fallback_to_default: bool,
}

pub struct PostPage {
    pub site_title: String,
    pub selected_language: String,
    pub languages: Vec<LanguageRecord>,
    pub post: PostRecord,
    pub available_languages: Vec<String>,
}

pub struct MessagePage {
    pub site_title: String,
    pub selected_language: String,
    pub languages: Vec<LanguageRecord>,
    pub title: String,
    pub message: String,
}

pub fn render_home(page: &HomePage) -> String {
    let mut content = String::new();
    let heading = match &page.category {
        Some(category) => format!("Posts in {}", escape_html(category)),
        None => "Posts".to_string(),
    };
    let _ = write!(
        content,
        "<h1>{} <span class=\"badge\">{}</span></h1>",
        heading,
        page.items.len()
    );
    if page.fallback_to_default {
        let _ = write!(
            content,
            "<div class=\"notice\">No posts in {} yet, showing {}.</div>",
            escape_html(&language_label(&page.selected_language)),
            escape_html(&language_label(&page.default_language))
        );
    }
    if page.items.is_empty() {
        content.push_str("<p>No posts found.</p>");
    } else {
        content.push_str("<ul class=\"post-list\">");
        for item in &page.items {
            let summary = &item.summary;
            let _ = write!(
                content,
                "<li class=\"card\"><a class=\"card-title\" href=\"/posts/{}?lang={}\">{}</a><div class=\"card-meta\">",
                urlencoding::encode(&summary.slug),
                urlencoding::encode(&page.selected_language),
                escape_html(&summary.title)
            );
            for category in &summary.categories {
                let _ = write!(
                    content,
                    "<a class=\"badge category\" href=\"/category/{}?lang={}\">{}</a>",
                    urlencoding::encode(category),
                    urlencoding::encode(&page.selected_language),
                    escape_html(category)
                );
            }
            for language in &item.languages {
                let _ = write!(
                    content,
                    "<span class=\"badge\">{}</span>",
                    escape_html(&language_label(language))
                );
            }
            if item.is_fallback {
                content.push_str("<span class=\"badge\">fallback</span>");
            }
            content.push_str("</div></li>");
        }
        content.push_str("</ul>");
    }
    let next_path = match &page.category {
        Some(category) => format!("/category/{}", urlencoding::encode(category)),
        None => "/".to_string(),
    };
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.languages,
        &next_path,
        None,
        &content,
    )
}

pub fn render_post(page: &PostPage) -> String {
    let mut content = String::new();
    let _ = write!(
        content,
        "<nav class=\"breadcrumbs\"><a href=\"/?lang={}\">Posts</a></nav>",
        urlencoding::encode(&page.selected_language)
    );
    let _ = write!(
        content,
        "<article class=\"post\"><h1>{}</h1><div class=\"post-meta\">",
        escape_html(&page.post.title)
    );
    for category in &page.post.categories {
        let _ = write!(
            content,
            "<a class=\"badge category\" href=\"/category/{}?lang={}\">{}</a>",
            urlencoding::encode(category),
            urlencoding::encode(&page.selected_language),
            escape_html(category)
        );
    }
    let _ = write!(
        content,
        "<span class=\"badge\">{}</span></div>",
        escape_html(&language_label(&page.post.language))
    );
    if page.post.language != page.selected_language {
        let _ = write!(
            content,
            "<div class=\"notice\">Translation into {} is not available, showing {}. Pick another language above or add a translated .txt file with slug \"{}\" and language \"{}\".</div>",
            escape_html(&language_label(&page.selected_language)),
            escape_html(&language_label(&page.post.language)),
            escape_html(&page.post.slug),
            escape_html(&page.selected_language)
        );
    }
    let _ = write!(
        content,
        "<div class=\"post-body\">{}</div></article>",
        page.post.body_html
    );
    let next_path = format!("/posts/{}", urlencoding::encode(&page.post.slug));
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.languages,
        &next_path,
        Some(&page.available_languages),
        &content,
    )
}

pub fn render_message(page: &MessagePage) -> String {
    let content = format!(
        "<h1>{}</h1><p>{}</p><p><a href=\"/\">Back to posts</a></p>",
        escape_html(&page.title),
        escape_html(&page.message)
    );
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.languages,
        "/",
        None,
        &content,
    )
}

fn render_layout(
    site_title: &str,
    selected_language: &str,
    languages: &[LanguageRecord],
    next_path: &str,
    available_languages: Option<&[String]>,
    content: &str,
) -> String {
    let mut html = String::new();
    let _ = write!(
        html,
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"/><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/><title>{}</title><link rel=\"stylesheet\" href=\"/assets/style.css\"/></head><body><header class=\"site-header\"><a class=\"site-title\" href=\"/\">{}</a><nav class=\"language-switcher\" aria-label=\"Language\">",
        escape_html(selected_language),
        escape_html(site_title),
        escape_html(site_title)
    );
    for language in languages {
        let is_current = language.code.as_str() == selected_language;
        let is_available = available_languages
            .map(|available| available.contains(&language.code))
            .unwrap_or(true);
        let mut classes = Vec::new();
        if is_current {
            classes.push("current");
        }
        if !is_available {
            classes.push("unavailable");
        }
        let class_attribute = if classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", classes.join(" "))
        };
        let current_attribute = if is_current {
            " aria-current=\"true\""
        } else {
            ""
        };
        let _ = write!(
            html,
            "<a href=\"/language/{}?next={}\"{}{} hreflang=\"{}\" title=\"{}\">{}</a>",
            urlencoding::encode(&language.code),
            urlencoding::encode(next_path),
            class_attribute,
            current_attribute,
            escape_html(&language.code),
            escape_html(&language_label(&language.code)),
            escape_html(&language_label(&language.code))
        );
    }
    let _ = write!(
        html,
        "</nav></header><main>{content}</main><footer><p>Powered by <a href=\"https://github.com/zefir1990/RustyBlogger\" rel=\"noopener\" target=\"_blank\">RustyBlogger</a> and <a href=\"https://github.com/cberner/redb\" rel=\"noopener\" target=\"_blank\">redb</a></p></footer></body></html>"
    );
    html
}

pub fn language_label(language: &str) -> String {
    match language {
        "ru" => "Русский".to_string(),
        "en" => "English".to_string(),
        "zh" => "中文".to_string(),
        "de" => "Deutsch".to_string(),
        "ja" => "日本語".to_string(),
        "fr" => "Français".to_string(),
        "es" => "Español".to_string(),
        "pt" => "Português".to_string(),
        "hi" => "हिन्दी".to_string(),
        other => other.to_ascii_uppercase(),
    }
}
