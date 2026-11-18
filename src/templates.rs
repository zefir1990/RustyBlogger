use std::fmt::Write as _;

use crate::date::format_unix_date;
use crate::model::{LanguageRecord, PostRecord};
use crate::render::escape_html;

pub const STYLESHEET: &str = r#"
:root {
  color-scheme: dark;
  --background: #090a0f;
  --surface: rgba(255, 255, 255, 0.045);
  --surface-strong: rgba(255, 255, 255, 0.08);
  --text: #e8eef7;
  --muted: #96a3b8;
  --border: rgba(255, 255, 255, 0.09);
  --accent: #7dd3fc;
  --accent-secondary: #a78bfa;
  --accent-tertiary: #f472b6;
  --accent-soft: rgba(125, 211, 252, 0.12);
  --code-background: #0d1117;
  --code-text: #e6edf3;
  --shadow: 0 24px 70px rgba(0, 0, 0, 0.45);
}
* { box-sizing: border-box; }
html { font-size: 17px; scroll-behavior: smooth; max-width: 100%; overflow-x: clip; }
body { max-width: 100%; }
body {
  min-height: 100vh;
  margin: 0;
  background:
    radial-gradient(1100px 700px at 12% -12%, rgba(125, 211, 252, 0.17), transparent 62%),
    radial-gradient(900px 650px at 88% -5%, rgba(167, 139, 250, 0.18), transparent 58%),
    radial-gradient(1000px 900px at 50% 118%, rgba(244, 114, 182, 0.12), transparent 62%),
    var(--background);
  background-attachment: fixed;
  color: var(--text);
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  line-height: 1.65;
  -webkit-font-smoothing: antialiased;
}
body::before {
  content: "";
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background-image:
    linear-gradient(rgba(255, 255, 255, 0.022) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.022) 1px, transparent 1px);
  background-size: 64px 64px;
  mask-image: radial-gradient(circle at 50% 0%, #000 0%, transparent 72%);
}
::selection { background: rgba(167, 139, 250, 0.35); color: #fff; }
a { color: var(--accent); text-decoration-thickness: from-font; text-underline-offset: 0.18em; }
a:hover { color: var(--accent-secondary); }
.site-header {
  position: sticky;
  top: 0;
  z-index: 30;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  width: 100%;
  padding: 1rem clamp(1rem, 3.5vw, 3rem);
  border-bottom: 1px solid var(--border);
  background: rgba(9, 10, 15, 0.68);
  backdrop-filter: blur(22px) saturate(140%);
}
.site-title {
  font-size: clamp(1.05rem, 1.4vw, 1.35rem);
  font-weight: 750;
  letter-spacing: -0.03em;
  text-decoration: none;
  color: transparent;
  background: linear-gradient(90deg, #7dd3fc, #a78bfa 55%, #f472b6);
  -webkit-background-clip: text;
  background-clip: text;
}
.site-title:hover { filter: brightness(1.15); color: transparent; }
.language-switcher { display: flex; flex-wrap: wrap; gap: 0.3rem; }
.language-switcher a {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.22rem 0.65rem;
  font-size: 0.76rem;
  font-weight: 550;
  text-decoration: none;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.03);
  transition: color 160ms ease, border-color 160ms ease, background 160ms ease;
}
.language-switcher a:hover {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.07);
}
.language-switcher a.current {
  border-color: rgba(125, 211, 252, 0.45);
  background: linear-gradient(135deg, rgba(125, 211, 252, 0.18), rgba(167, 139, 250, 0.18));
  color: #dceeff;
  box-shadow: 0 0 20px rgba(125, 211, 252, 0.12);
}
.language-switcher a.unavailable { opacity: 0.4; }
main {
  width: 100%;
  max-width: none;
  padding: clamp(1.25rem, 3vw, 3rem) clamp(0.75rem, 1.5vw, 1.5rem) 2rem;
}
h1 {
  line-height: 1.08;
  font-size: clamp(2rem, 4vw, 3.2rem);
  letter-spacing: -0.04em;
  margin: 0.5rem 0 1.2rem;
}
h2 {
  line-height: 1.2;
  margin-top: 2.2rem;
  font-size: clamp(1.35rem, 2vw, 1.85rem);
  letter-spacing: -0.02em;
}
.post-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: clamp(0.85rem, 1.5vw, 1.35rem);
}
.card {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.025));
  border: 1px solid var(--border);
  border-radius: clamp(16px, 1.4vw, 24px);
  padding: clamp(1.1rem, 1.8vw, 1.6rem);
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
}
.post-card {
  width: 100%;
  min-width: 0;
  padding: clamp(1.3rem, 3.2vw, 3rem);
  transition: border-color 200ms ease, transform 200ms ease, box-shadow 200ms ease;
}
.post-card:hover {
  border-color: rgba(167, 139, 250, 0.35);
  box-shadow: var(--shadow);
  transform: translateY(-3px);
}
.card-title {
  font-size: 1.15rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  text-decoration: none;
  color: var(--text);
  overflow-wrap: anywhere;
}
.card-title:hover { color: var(--accent); }
.post-card .card-title {
  display: block;
  font-size: clamp(1.55rem, 3vw, 2.75rem);
  line-height: 1.06;
  letter-spacing: -0.045em;
  background: linear-gradient(92deg, #f8fafc 0%, #c7d2fe 52%, #f0abfc 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.post-card .card-title:hover { filter: brightness(1.12); }
.card-meta, .post-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
  margin-top: 0.85rem;
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}
.badge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.12rem 0.55rem;
  background: rgba(255, 255, 255, 0.04);
}
.badge.category {
  background: var(--accent-soft);
  border-color: rgba(125, 211, 252, 0.25);
  color: #bae6fd;
  text-transform: none;
  letter-spacing: 0.01em;
  text-decoration: none;
}
.badge.category:hover { background: rgba(125, 211, 252, 0.2); color: #e0f2fe; }
time { font-variant-numeric: tabular-nums; }
.notice {
  border: 1px solid rgba(244, 114, 182, 0.2);
  border-left: 3px solid var(--accent-tertiary);
  border-radius: 12px;
  background: rgba(244, 114, 182, 0.08);
  padding: 0.8rem 1rem;
  margin: 1rem 0;
  font-size: 0.9rem;
  color: #fbcfe8;
}
.breadcrumbs {
  font-size: 0.78rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin: 0 0 0.8rem;
  color: var(--muted);
}
.breadcrumbs a { color: var(--accent); text-decoration: none; }
.post {
  width: 100%;
  min-width: 0;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.025));
  border: 1px solid var(--border);
  border-radius: clamp(16px, 1.2vw, 24px);
  padding: clamp(1.3rem, 3.2vw, 3.2rem);
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
}
.post h1 {
  margin-top: 0;
  overflow-wrap: anywhere;
  background: linear-gradient(92deg, #f8fafc, #c7d2fe 55%, #f0abfc);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.post-body {
  min-width: 0;
  max-width: 100%;
  overflow-wrap: anywhere;
  color: #cbd5e1;
  font-size: clamp(1rem, 0.25vw + 0.96rem, 1.12rem);
}
.post-body p { margin: 0.9rem 0; }
.post-body figure { margin: 1.4rem 0; }
.post-body img {
  display: block;
  width: auto;
  max-width: 100%;
  height: auto;
  border-radius: 14px;
  border: 1px solid var(--border);
}
.post-body iframe, .post-body video {
  max-width: 100%;
  height: auto;
  border-radius: 14px;
  border: 1px solid var(--border);
}
.post-body h2, .post-body h3, .post-body h4 { color: var(--text); }
.post-body blockquote {
  margin: 1.4rem 0;
  padding: 0.8rem 1.1rem;
  border-left: 3px solid var(--accent-secondary);
  border-radius: 0 12px 12px 0;
  background: rgba(167, 139, 250, 0.08);
  color: #ddd6fe;
}
.post-body pre {
  background: var(--code-background);
  color: var(--code-text);
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 14px;
  padding: 1rem 1.1rem;
  max-width: 100%;
  overflow-x: auto;
  font-size: 0.88rem;
  line-height: 1.55;
}
.post-body code {
  background: rgba(125, 211, 252, 0.1);
  border-radius: 5px;
  padding: 0.05rem 0.32rem;
  font-size: 0.9em;
  color: #bae6fd;
}
.post-body pre code { background: transparent; padding: 0; font-size: inherit; color: inherit; }
.post-body .post-link { margin: 0.5rem 0; }
.post-card .post-body { margin-top: 1.1rem; }
.permalink {
  display: inline-block;
  margin-top: 0.2rem;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  text-decoration: none;
  color: var(--muted);
}
.permalink:hover { color: var(--accent); }
.pagination {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  margin-top: 2rem;
}
.pagination a, .pagination span {
  min-width: 2.4rem;
  padding: 0.4rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  text-align: center;
  text-decoration: none;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.035);
  font-size: 0.82rem;
  font-weight: 600;
  transition: color 160ms ease, border-color 160ms ease, background 160ms ease;
}
.pagination a:hover {
  border-color: rgba(125, 211, 252, 0.4);
  color: var(--accent);
  background: rgba(125, 211, 252, 0.1);
}
.pagination .current {
  border-color: rgba(167, 139, 250, 0.5);
  background: linear-gradient(135deg, rgba(125, 211, 252, 0.18), rgba(167, 139, 250, 0.18));
  color: #e9d5ff;
}
.pagination .disabled { opacity: 0.3; }
.pagination-summary {
  margin: 0.9rem 0 0;
  text-align: center;
  color: var(--muted);
  font-size: 0.72rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
footer {
  width: 100%;
  max-width: none;
  margin: 0;
  padding: 1.5rem clamp(1rem, 3.5vw, 3rem) 2.5rem;
  border-top: 1px solid var(--border);
  color: var(--muted);
  font-size: 0.78rem;
}
footer p { margin: 0; }
footer a { color: var(--muted); text-decoration: none; }
footer a:hover { color: var(--accent); }
.gdpr-warning {
  position: fixed;
  left: 50%;
  bottom: 1rem;
  transform: translateX(-50%);
  z-index: 40;
  width: min(960px, calc(100% - 2rem));
  margin: 0;
  padding: 0.9rem 1rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  border: 1px solid rgba(125, 211, 252, 0.2);
  border-left: 3px solid var(--accent);
  border-radius: 14px;
  background: rgba(15, 18, 27, 0.88);
  backdrop-filter: blur(18px);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  font-size: 0.84rem;
  color: var(--text);
}
.gdpr-warning p { margin: 0; flex: 1 1 16rem; }
.gdpr-warning button {
  border: 0;
  border-radius: 999px;
  padding: 0.45rem 0.95rem;
  background: linear-gradient(135deg, #7dd3fc, #a78bfa);
  color: #081018;
  font: inherit;
  font-size: 0.8rem;
  font-weight: 700;
  cursor: pointer;
}
.gdpr-warning button:hover { filter: brightness(1.1); }
.gdpr-warning[hidden] { display: none; }
@media (max-width: 640px) {
  html { font-size: 16px; }
  .site-header { padding: 0.85rem 1rem; }
  .language-switcher a { padding: 0.18rem 0.5rem; font-size: 0.7rem; }
  main { padding-left: 0.6rem; padding-right: 0.6rem; }
  .post-card, .post { padding: 1.15rem; border-radius: 18px; }
  .post-card .card-title { font-size: clamp(1.4rem, 7vw, 2rem); }
  .gdpr-warning { width: calc(100% - 1.2rem); }
}
"#;

pub struct HomeItem {
    pub post: PostRecord,
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
    pub posts_per_page: usize,
    pub current_page: usize,
    pub total_pages: usize,
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
    if let Some(category) = &page.category {
        let _ = write!(content, "<h1>Posts in {}</h1>", escape_html(category));
    }
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
            let post = &item.post;
            let post_href = format!(
                "/posts/{}?lang={}",
                urlencoding::encode(&post.slug),
                urlencoding::encode(&page.selected_language)
            );
            let _ = write!(
                content,
                "<li class=\"card post-card\"><a class=\"card-title\" href=\"{}\">{}</a><div class=\"card-meta\">",
                escape_html(&post_href),
                escape_html(&post.title)
            );
            for category in &post.categories {
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
            let post_date = if post.published_date.is_empty() {
                format_unix_date(post.published_unix_seconds)
            } else {
                post.published_date.clone()
            };
            let _ = write!(content, "<time datetime=\"{post_date}\">{post_date}</time>");
            if item.is_fallback {
                content.push_str("<span class=\"badge\">fallback</span>");
            }
            let _ = write!(
                content,
                "</div><div class=\"post-body\">{}</div><p><a class=\"permalink\" href=\"{}\">Permalink</a></p></li>",
                post.body_html,
                escape_html(&post_href)
            );
        }
        content.push_str("</ul>");
        render_pagination(&mut content, page);
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

fn render_pagination(content: &mut String, page: &HomePage) {
    let _ = write!(
        content,
        "<p class=\"pagination-summary\">Page {} of {} · {} posts per page</p>",
        page.current_page, page.total_pages, page.posts_per_page
    );
    if page.total_pages <= 1 {
        return;
    }
    content.push_str("<nav class=\"pagination\" aria-label=\"Pagination\">");
    if page.current_page > 1 {
        let _ = write!(
            content,
            "<a href=\"{}\" rel=\"prev\">Previous</a>",
            escape_html(&pagination_href(page, page.current_page - 1))
        );
    } else {
        content.push_str("<span class=\"disabled\">Previous</span>");
    }
    for page_number in 1..=page.total_pages {
        if page_number == page.current_page {
            let _ = write!(content, "<span class=\"current\">{page_number}</span>");
        } else {
            let _ = write!(
                content,
                "<a href=\"{}\">{page_number}</a>",
                escape_html(&pagination_href(page, page_number))
            );
        }
    }
    if page.current_page < page.total_pages {
        let _ = write!(
            content,
            "<a href=\"{}\" rel=\"next\">Next</a>",
            escape_html(&pagination_href(page, page.current_page + 1))
        );
    } else {
        content.push_str("<span class=\"disabled\">Next</span>");
    }
    content.push_str("</nav>");
}

fn pagination_href(page: &HomePage, page_number: usize) -> String {
    let base_path = match &page.category {
        Some(category) => format!("/category/{}", urlencoding::encode(category)),
        None => "/".to_string(),
    };
    format!(
        "{}?lang={}&page={}",
        base_path,
        urlencoding::encode(&page.selected_language),
        page_number
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
    let post_date = if page.post.published_date.is_empty() {
        format_unix_date(page.post.published_unix_seconds)
    } else {
        page.post.published_date.clone()
    };
    let _ = write!(
        content,
        "<span class=\"badge\">{}</span><time datetime=\"{post_date}\">{post_date}</time></div>",
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
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"/><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/><base target=\"_blank\"/><title>{}</title><link rel=\"stylesheet\" href=\"/assets/style.css\"/></head><body><header class=\"site-header\"><a class=\"site-title\" href=\"/\">{}</a><nav class=\"language-switcher\" aria-label=\"Language\">",
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
        "</nav></header><main>{content}</main><footer><p>Powered by <a href=\"https://github.com/zefir1990/RustyBlogger\" rel=\"noopener\" target=\"_blank\">RustyBlogger</a> and <a href=\"https://github.com/cberner/redb\" rel=\"noopener\" target=\"_blank\">redb</a></p><p>Blog by <a href=\"https://zefir1990.space/posts/zefir1990\">zefir1990.space/posts/zefir1990</a></p></footer>"
    );
    let warning = gdpr_warning_text(selected_language);
    let _ = write!(
        html,
        "<aside class=\"gdpr-warning\" id=\"gdpr-warning\" role=\"note\"><p>{}</p><button type=\"button\" id=\"gdpr-dismiss\">{}</button></aside><script>(function(){{var key=\"rustyblogger-gdpr-dismissed\";var warning=document.getElementById(\"gdpr-warning\");if(!warning){{return;}}var dismissed=false;try{{dismissed=window.localStorage.getItem(key)===\"1\";}}catch(error){{dismissed=false;}}if(dismissed){{warning.hidden=true;return;}}var button=document.getElementById(\"gdpr-dismiss\");button.addEventListener(\"click\",function(){{warning.hidden=true;try{{window.localStorage.setItem(key,\"1\");}}catch(error){{}}}});}})();</script></body></html>",
        escape_html(warning.message),
        escape_html(warning.dismiss_label)
    );
    html
}

struct GdprWarningText {
    message: &'static str,
    dismiss_label: &'static str,
}

fn gdpr_warning_text(language: &str) -> GdprWarningText {
    match language {
        "ru" => GdprWarningText {
            message: "Сайт использует функциональный cookie для запоминания языка и локальное хранилище, чтобы скрыть это уведомление. Данные для отслеживания не сохраняются.",
            dismiss_label: "Понятно",
        },
        "zh" => GdprWarningText {
            message: "本站使用功能性 Cookie 记住您的语言选择，并使用本地存储隐藏此提示。不会存储跟踪数据。",
            dismiss_label: "知道了",
        },
        "de" => GdprWarningText {
            message: "Diese Website verwendet ein funktionales Cookie, um Ihre Sprachauswahl zu speichern, und lokalen Speicher, um diesen Hinweis auszublenden. Es werden keine Tracking-Daten gespeichert.",
            dismiss_label: "Verstanden",
        },
        "ja" => GdprWarningText {
            message: "このサイトでは、言語の選択を記憶するために機能性 Cookie を、この通知を非表示にするためにローカルストレージを使用します。追跡データは保存されません。",
            dismiss_label: "了解",
        },
        "fr" => GdprWarningText {
            message: "Ce site utilise un cookie fonctionnel pour mémoriser votre choix de langue et le stockage local pour masquer cet avis. Aucune donnée de suivi n’est conservée.",
            dismiss_label: "Compris",
        },
        "es" => GdprWarningText {
            message: "Este sitio utiliza una cookie funcional para recordar su elección de idioma y el almacenamiento local para ocultar este aviso. No se almacenan datos de seguimiento.",
            dismiss_label: "Entendido",
        },
        "pt" => GdprWarningText {
            message: "Este site utiliza um cookie funcional para memorizar a sua escolha de idioma e o armazenamento local para ocultar este aviso. Não são armazenados dados de rastreamento.",
            dismiss_label: "Entendi",
        },
        "hi" => GdprWarningText {
            message: "यह साइट आपकी भाषा का चयन याद रखने के लिए एक कार्यात्मक कुकी और इस सूचना को छिपाने के लिए स्थानीय स्टोरेज का उपयोग करती है। कोई ट्रैकिंग डेटा संग्रहीत नहीं किया जाता है।",
            dismiss_label: "समझ गया",
        },
        _ => GdprWarningText {
            message: "This site uses a functional cookie to remember your language choice and local storage to hide this notice. No tracking data is stored.",
            dismiss_label: "Got it",
        },
    }
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
