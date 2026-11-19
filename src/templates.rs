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
.site-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 0.6rem;
}
.category-nav { display: flex; flex-wrap: wrap; gap: 0.4rem; }
.category-button {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.34rem 0.85rem;
  font-size: 0.78rem;
  font-weight: 550;
  text-decoration: none;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.03);
  transition: color 160ms ease, border-color 160ms ease, background 160ms ease;
}
.category-button:hover {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.07);
}
.category-button.current {
  border-color: rgba(125, 211, 252, 0.45);
  background: linear-gradient(135deg, rgba(125, 211, 252, 0.18), rgba(167, 139, 250, 0.18));
  color: #dceeff;
  box-shadow: 0 0 20px rgba(125, 211, 252, 0.12);
}
.language-switcher { display: flex; align-items: center; }
.language-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.5rem;
  height: 2.5rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  font-size: 1.15rem;
  line-height: 1;
  text-decoration: none;
  background: rgba(255, 255, 255, 0.03);
  transition: border-color 160ms ease, background 160ms ease, transform 160ms ease;
}
.language-toggle:hover {
  border-color: rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.07);
  transform: translateY(-1px);
}
.language-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.language-options {
  list-style: none;
  padding: 0;
  margin: 1.5rem 0 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
  gap: 0.75rem;
}
.language-options a {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.9rem 1.1rem;
  border: 1px solid var(--border);
  border-radius: 16px;
  text-decoration: none;
  color: var(--text);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.025));
  transition: border-color 160ms ease, background 160ms ease;
}
.language-options a:hover {
  border-color: rgba(167, 139, 250, 0.35);
  background: rgba(255, 255, 255, 0.08);
}
.language-options a.current {
  border-color: rgba(125, 211, 252, 0.45);
  background: linear-gradient(135deg, rgba(125, 211, 252, 0.18), rgba(167, 139, 250, 0.18));
}
.language-options .language-flag { font-size: 1.4rem; line-height: 1; }
.language-options .language-name { font-weight: 600; }
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
.post-body .code-block { position: relative; margin: 1rem 0; }
.post-body .code-block pre { margin: 0; padding-top: 3rem; }
.code-block-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.5rem 0.7rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 14px 14px 0 0;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.015));
}
.code-block-label {
  color: var(--muted);
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.copy-code {
  flex: 0 0 auto;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 999px;
  padding: 0.28rem 0.7rem;
  background: rgba(15, 18, 27, 0.82);
  color: var(--muted);
  font: inherit;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.03em;
  cursor: pointer;
  backdrop-filter: blur(10px);
  transition: color 160ms ease, border-color 160ms ease, background 160ms ease;
}
.copy-code:hover, .copy-code:focus-visible {
  color: var(--text);
  border-color: rgba(125, 211, 252, 0.45);
  background: rgba(125, 211, 252, 0.14);
  outline: none;
}
.copy-code.copied {
  color: #86efac;
  border-color: rgba(134, 239, 172, 0.45);
  background: rgba(134, 239, 172, 0.12);
}
.copy-code.failed {
  color: #fca5a5;
  border-color: rgba(252, 165, 165, 0.45);
  background: rgba(252, 165, 165, 0.12);
}
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
  .language-toggle { width: 2.2rem; height: 2.2rem; font-size: 1rem; }
  .category-button { padding: 0.28rem 0.65rem; font-size: 0.72rem; }
  main { padding-left: 0.6rem; padding-right: 0.6rem; }
  .post-card, .post { padding: 1.15rem; border-radius: 18px; }
  .post-card .card-title { font-size: clamp(1.4rem, 7vw, 2rem); }
  .gdpr-warning { width: calc(100% - 1.2rem); }
}
"#;

const COPY_CODE_SCRIPT_TEMPLATE: &str = r#"
(function () {
  var copyLabel = "__COPY_LABEL__";
  var copiedLabel = "__COPIED_LABEL__";
  var failedLabel = "__FAILED_LABEL__";
  var ariaLabel = "__ARIA_LABEL__";
  var exampleLabel = "__EXAMPLE_LABEL__";
  function mark(button, label, state) {
    button.textContent = label;
    button.classList.remove("copied", "failed");
    button.classList.add(state);
    window.clearTimeout(button.resetTimer);
    button.resetTimer = window.setTimeout(function () {
      button.textContent = copyLabel;
      button.classList.remove("copied", "failed");
    }, 1800);
  }
  function copyWithFallback(text, button) {
    var textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    var copied = false;
    try {
      copied = document.execCommand("copy");
    } catch (error) {
      copied = false;
    }
    document.body.removeChild(textarea);
    mark(button, copied ? copiedLabel : failedLabel, copied ? "copied" : "failed");
  }
  function copyText(text, button) {
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(function () {
        mark(button, copiedLabel, "copied");
      }, function () {
        copyWithFallback(text, button);
      });
    } else {
      copyWithFallback(text, button);
    }
  }
  Array.prototype.forEach.call(document.querySelectorAll(".post-body pre"), function (pre) {
    var code = pre.querySelector("code");
    if (!code || (pre.parentElement && pre.parentElement.classList.contains("code-block"))) {
      return;
    }
    var wrapper = document.createElement("div");
    wrapper.className = "code-block";
    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);
    var header = document.createElement("div");
    header.className = "code-block-header";
    var label = document.createElement("span");
    label.className = "code-block-label";
    label.textContent = exampleLabel;
    var button = document.createElement("button");
    button.type = "button";
    button.className = "copy-code";
    button.textContent = copyLabel;
    button.setAttribute("aria-label", ariaLabel);
    button.title = ariaLabel;
    button.addEventListener("click", function () {
      copyText(code.textContent, button);
    });
    header.appendChild(label);
    header.appendChild(button);
    wrapper.appendChild(header);
  });
})();
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
    pub categories: Vec<String>,
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
    pub categories: Vec<String>,
    pub post: PostRecord,
}

pub struct MessagePage {
    pub site_title: String,
    pub selected_language: String,
    pub categories: Vec<String>,
    pub title: String,
    pub message: String,
}

pub struct LanguagesPage {
    pub site_title: String,
    pub selected_language: String,
    pub categories: Vec<String>,
    pub languages: Vec<LanguageRecord>,
    pub next_path: String,
}

pub fn render_home(page: &HomePage) -> String {
    let mut content = String::new();
    if page.fallback_to_default {
        let notice = fallback_notice_text(
            &page.selected_language,
            &page.selected_language,
            &page.default_language,
        );
        let _ = write!(
            content,
            "<div class=\"notice\">{}</div>",
            escape_html(&notice)
        );
    }
    if page.items.is_empty() {
        let _ = write!(
            content,
            "<p>{}</p>",
            escape_html(no_posts_found_label(&page.selected_language))
        );
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
                "<li class=\"card post-card\"><a class=\"card-title\" href=\"{}\" target=\"_self\">{}</a><div class=\"card-meta\">",
                escape_html(&post_href),
                escape_html(&post.title)
            );
            for category in &post.categories {
                let localized_category = category_label(category, &page.selected_language);
                let _ = write!(
                    content,
                    "<a class=\"badge category\" href=\"/category/{}?lang={}\" target=\"_self\">{}</a>",
                    urlencoding::encode(category),
                    urlencoding::encode(&page.selected_language),
                    escape_html(&localized_category)
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
                "</div><div class=\"post-body\">{}</div><p><a class=\"permalink\" href=\"{}\" target=\"_self\">Permalink</a></p></li>",
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
    let active_categories: Vec<String> = page.category.iter().cloned().collect();
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.categories,
        &active_categories,
        &next_path,
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
            "<a href=\"{}\" rel=\"prev\" target=\"_self\">Previous</a>",
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
                "<a href=\"{}\" target=\"_self\">{page_number}</a>",
                escape_html(&pagination_href(page, page_number))
            );
        }
    }
    if page.current_page < page.total_pages {
        let _ = write!(
            content,
            "<a href=\"{}\" rel=\"next\" target=\"_self\">Next</a>",
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
        "<article class=\"post\"><h1>{}</h1><div class=\"post-meta\">",
        escape_html(&page.post.title)
    );
    for category in &page.post.categories {
        let localized_category = category_label(category, &page.selected_language);
        let _ = write!(
            content,
            "<a class=\"badge category\" href=\"/category/{}?lang={}\">{}</a>",
            urlencoding::encode(category),
            urlencoding::encode(&page.selected_language),
            escape_html(&localized_category)
        );
    }
    let post_date = if page.post.published_date.is_empty() {
        format_unix_date(page.post.published_unix_seconds)
    } else {
        page.post.published_date.clone()
    };
    let _ = write!(
        content,
        "<time datetime=\"{post_date}\">{post_date}</time></div>"
    );
    if page.post.language != page.selected_language {
        let notice = translation_notice_text(
            &page.selected_language,
            &page.selected_language,
            &page.post.language,
            &page.post.slug,
        );
        let _ = write!(
            content,
            "<div class=\"notice\">{}</div>",
            escape_html(&notice)
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
        &page.categories,
        &page.post.categories,
        &next_path,
        &content,
    )
}

pub fn render_message(page: &MessagePage) -> String {
    let content = format!(
        "<h1>{}</h1><p>{}</p><p><a href=\"/\" target=\"_self\">{}</a></p>",
        escape_html(&page.title),
        escape_html(&page.message),
        escape_html(back_to_posts_label(&page.selected_language))
    );
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.categories,
        &[],
        "/",
        &content,
    )
}

pub fn render_languages(page: &LanguagesPage) -> String {
    let mut content = String::new();
    let _ = write!(
        content,
        "<h1>{}</h1><ul class=\"language-options\">",
        escape_html(choose_language_label(&page.selected_language))
    );
    for language in &page.languages {
        let is_current = language.code.as_str() == page.selected_language;
        let class_attribute = if is_current { " class=\"current\"" } else { "" };
        let current_attribute = if is_current {
            " aria-current=\"true\""
        } else {
            ""
        };
        let href = format!(
            "/language/{}?next={}",
            urlencoding::encode(&language.code),
            urlencoding::encode(&page.next_path)
        );
        let _ = write!(
            content,
            "<li><a href=\"{}\"{}{} hreflang=\"{}\" target=\"_self\"><span class=\"language-flag\" aria-hidden=\"true\">{}</span><span class=\"language-name\">{}</span></a></li>",
            escape_html(&href),
            class_attribute,
            current_attribute,
            escape_html(&language.code),
            language_flag(&language.code),
            escape_html(&language_label(&language.code))
        );
    }
    content.push_str("</ul>");
    render_layout(
        &page.site_title,
        &page.selected_language,
        &page.categories,
        &[],
        &page.next_path,
        &content,
    )
}

fn render_layout(
    site_title: &str,
    selected_language: &str,
    categories: &[String],
    active_categories: &[String],
    next_path: &str,
    content: &str,
) -> String {
    let mut category_buttons = String::new();
    for category in categories {
        let is_current = active_categories.iter().any(|active| active == category);
        let class_name = if is_current {
            "category-button current"
        } else {
            "category-button"
        };
        let current_attribute = if is_current {
            " aria-current=\"true\""
        } else {
            ""
        };
        let _ = write!(
            category_buttons,
            "<a class=\"{}\" href=\"/category/{}?lang={}\"{} target=\"_self\">{}</a>",
            class_name,
            urlencoding::encode(category),
            urlencoding::encode(selected_language),
            current_attribute,
            escape_html(&category_label(category, selected_language))
        );
    }
    let mut html = String::new();
    let _ = write!(
        html,
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"/><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/><base target=\"_blank\"/><title>{}</title><link rel=\"stylesheet\" href=\"/assets/style.css\"/></head><body><header class=\"site-header\"><a class=\"site-title\" href=\"/\" target=\"_self\">{}</a><div class=\"site-actions\"><nav class=\"category-nav\" aria-label=\"{}\">{category_buttons}</nav><nav class=\"language-switcher\" aria-label=\"{}\"><a class=\"language-toggle\" href=\"/languages?next={}\" hreflang=\"{}\" title=\"{}\" aria-label=\"{}\" target=\"_self\">{}</a></nav></div></header>",
        escape_html(selected_language),
        escape_html(site_title),
        escape_html(site_title),
        escape_html(categories_label(selected_language)),
        escape_html(choose_language_label(selected_language)),
        urlencoding::encode(next_path),
        escape_html(selected_language),
        escape_html(choose_language_label(selected_language)),
        escape_html(choose_language_label(selected_language)),
        language_flag(selected_language)
    );
    let powered_by = footer_powered_by_label(selected_language)
        .replace(
            "{rustyblogger}",
            "<a href=\"https://github.com/zefir1990/RustyBlogger\" rel=\"noopener\" target=\"_blank\">RustyBlogger</a>",
        )
        .replace(
            "{redb}",
            "<a href=\"https://github.com/cberner/redb\" rel=\"noopener\" target=\"_blank\">redb</a>",
        );
    let author_by = footer_author_label(selected_language).replace(
        "{blog}",
        "<a href=\"https://zefir1990.space/posts/zefir1990\">zefir1990.space/posts/zefir1990</a>",
    );
    let _ = write!(
        html,
        "</header><main>{content}</main><footer><p>{powered_by}</p><p>{author_by}</p></footer>"
    );
    let warning = gdpr_warning_text(selected_language);
    let copy_code_labels = copy_code_labels(selected_language);
    let copy_code_script = COPY_CODE_SCRIPT_TEMPLATE
        .replace("__COPY_LABEL__", copy_code_labels.copy)
        .replace("__COPIED_LABEL__", copy_code_labels.copied)
        .replace("__FAILED_LABEL__", copy_code_labels.failed)
        .replace("__ARIA_LABEL__", copy_code_labels.aria)
        .replace("__EXAMPLE_LABEL__", copy_code_labels.example);
    let _ = write!(
        html,
        "<aside class=\"gdpr-warning\" id=\"gdpr-warning\" role=\"note\"><p>{}</p><button type=\"button\" id=\"gdpr-dismiss\">{}</button></aside><script>(function(){{var key=\"rustyblogger-gdpr-dismissed\";var warning=document.getElementById(\"gdpr-warning\");if(!warning){{return;}}var dismissed=false;try{{dismissed=window.localStorage.getItem(key)===\"1\";}}catch(error){{dismissed=false;}}if(dismissed){{warning.hidden=true;return;}}var button=document.getElementById(\"gdpr-dismiss\");button.addEventListener(\"click\",function(){{warning.hidden=true;try{{window.localStorage.setItem(key,\"1\");}}catch(error){{}}}});}})();</script><script>{copy_code_script}</script></body></html>",
        escape_html(warning.message),
        escape_html(warning.dismiss_label)
    );
    html
}

struct CopyCodeLabels {
    copy: &'static str,
    copied: &'static str,
    failed: &'static str,
    aria: &'static str,
    example: &'static str,
}

fn copy_code_labels(language: &str) -> CopyCodeLabels {
    match language {
        "ru" => CopyCodeLabels {
            copy: "Копировать",
            copied: "Скопировано",
            failed: "Не удалось",
            aria: "Копировать код",
            example: "Пример кода",
        },
        "zh" => CopyCodeLabels {
            copy: "复制",
            copied: "已复制",
            failed: "复制失败",
            aria: "复制代码",
            example: "代码示例",
        },
        "de" => CopyCodeLabels {
            copy: "Kopieren",
            copied: "Kopiert",
            failed: "Kopieren fehlgeschlagen",
            aria: "Code kopieren",
            example: "Codebeispiel",
        },
        "ja" => CopyCodeLabels {
            copy: "コピー",
            copied: "コピーしました",
            failed: "コピー失敗",
            aria: "コードをコピー",
            example: "コード例",
        },
        "fr" => CopyCodeLabels {
            copy: "Copier",
            copied: "Copié",
            failed: "Échec de la copie",
            aria: "Copier le code",
            example: "Exemple de code",
        },
        "es" => CopyCodeLabels {
            copy: "Copiar",
            copied: "Copiado",
            failed: "Error al copiar",
            aria: "Copiar código",
            example: "Ejemplo de código",
        },
        "pt" => CopyCodeLabels {
            copy: "Copiar",
            copied: "Copiado",
            failed: "Falha ao copiar",
            aria: "Copiar código",
            example: "Exemplo de código",
        },
        "hi" => CopyCodeLabels {
            copy: "कॉपी करें",
            copied: "कॉपी हो गया",
            failed: "कॉपी विफल",
            aria: "कोड कॉपी करें",
            example: "कोड उदाहरण",
        },
        _ => CopyCodeLabels {
            copy: "Copy",
            copied: "Copied",
            failed: "Copy failed",
            aria: "Copy code",
            example: "Code example",
        },
    }
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

pub fn post_not_found_text(language: &str, slug: &str) -> (String, String) {
    let title = match language {
        "ru" => "Пост не найден",
        "zh" => "文章未找到",
        "de" => "Beitrag nicht gefunden",
        "ja" => "投稿が見つかりません",
        "fr" => "Article introuvable",
        "es" => "Publicación no encontrada",
        "pt" => "Publicação não encontrada",
        "hi" => "पोस्ट नहीं मिली",
        _ => "Post not found",
    };
    (title.to_string(), post_not_found_message(language, slug))
}

pub fn post_not_found_message(language: &str, slug: &str) -> String {
    match language {
        "ru" => format!("Пост со слагом «{slug}» не найден."),
        "zh" => format!("未找到别名为“{slug}”的文章。"),
        "de" => format!("Kein Beitrag mit dem Slug „{slug}“ gefunden."),
        "ja" => format!("スラッグ「{slug}」の投稿は見つかりません。"),
        "fr" => format!("Aucun article avec le slug « {slug} » n’a été trouvé."),
        "es" => format!("No se encontró ninguna publicación con el slug «{slug}»."),
        "pt" => format!("Nenhuma publicação encontrada com o slug «{slug}»."),
        "hi" => format!("स्लग \"{slug}\" वाली कोई पोस्ट नहीं मिली।"),
        _ => format!("No post with slug \"{slug}\"."),
    }
}

pub fn language_not_found_message(language: &str, missing_language: &str) -> String {
    match language {
        "ru" => format!("Язык «{missing_language}» не найден."),
        "zh" => format!("未找到语言“{missing_language}”。"),
        "de" => format!("Sprache „{missing_language}“ nicht gefunden."),
        "ja" => format!("言語「{missing_language}」が見つかりません。"),
        "fr" => format!("Langue « {missing_language} » introuvable."),
        "es" => format!("Idioma «{missing_language}» no encontrado."),
        "pt" => format!("Idioma «{missing_language}» não encontrado."),
        "hi" => format!("भाषा \"{missing_language}\" नहीं मिली।"),
        _ => format!("language \"{missing_language}\" not found"),
    }
}

pub fn page_not_found_text(language: &str) -> (&'static str, &'static str) {
    match language {
        "ru" => ("Страница не найдена", "Запрошенная страница не существует."),
        "zh" => ("页面未找到", "请求的页面不存在。"),
        "de" => (
            "Seite nicht gefunden",
            "Die angeforderte Seite existiert nicht.",
        ),
        "ja" => (
            "ページが見つかりません",
            "リクエストされたページは存在しません。",
        ),
        "fr" => ("Page introuvable", "La page demandée n’existe pas."),
        "es" => ("Página no encontrada", "La página solicitada no existe."),
        "pt" => ("Página não encontrada", "A página solicitada não existe."),
        "hi" => ("पृष्ठ नहीं मिला", "अनुरोधित पृष्ठ मौजूद नहीं है।"),
        _ => ("Page not found", "The requested page does not exist."),
    }
}

pub fn method_not_allowed_text(language: &str) -> (&'static str, &'static str) {
    match language {
        "ru" => ("Метод не разрешён", "Этот метод запроса не поддерживается."),
        "zh" => ("方法不允许", "不支持此请求方法。"),
        "de" => (
            "Methode nicht erlaubt",
            "Diese Anfragemethode wird nicht unterstützt.",
        ),
        "ja" => (
            "メソッドが許可されていません",
            "このリクエストメソッドはサポートされていません。",
        ),
        "fr" => (
            "Méthode non autorisée",
            "Cette méthode de requête n’est pas prise en charge.",
        ),
        "es" => (
            "Método no permitido",
            "Este método de solicitud no es compatible.",
        ),
        "pt" => (
            "Método não permitido",
            "Este método de solicitação não é suportado.",
        ),
        "hi" => ("विधि की अनुमति नहीं है", "यह अनुरोध विधि समर्थित नहीं है।"),
        _ => (
            "Method not allowed",
            "This request method is not supported.",
        ),
    }
}

fn footer_powered_by_label(language: &str) -> &'static str {
    match language {
        "ru" => "Работает на {rustyblogger} и {redb}",
        "zh" => "由 {rustyblogger} 和 {redb} 提供支持",
        "de" => "Bereitgestellt von {rustyblogger} und {redb}",
        "ja" => "{rustyblogger} と {redb} で動作",
        "fr" => "Propulsé par {rustyblogger} et {redb}",
        "es" => "Con tecnología de {rustyblogger} y {redb}",
        "pt" => "Desenvolvido com {rustyblogger} e {redb}",
        "hi" => "{rustyblogger} और {redb} द्वारा संचालित",
        _ => "Powered by {rustyblogger} and {redb}",
    }
}

fn footer_author_label(language: &str) -> &'static str {
    match language {
        "ru" => "Автор: {blog}",
        "zh" => "作者：{blog}",
        "de" => "Autor: {blog}",
        "ja" => "著者: {blog}",
        "fr" => "Auteur : {blog}",
        "es" => "Autor: {blog}",
        "pt" => "Autor: {blog}",
        "hi" => "लेखक: {blog}",
        _ => "Author: {blog}",
    }
}

fn back_to_posts_label(language: &str) -> &'static str {
    match language {
        "ru" => "Назад к постам",
        "zh" => "返回文章",
        "de" => "Zurück zu den Beiträgen",
        "ja" => "投稿一覧に戻る",
        "fr" => "Retour aux articles",
        "es" => "Volver a las publicaciones",
        "pt" => "Voltar às publicações",
        "hi" => "पोस्ट पर वापस जाएँ",
        _ => "Back to posts",
    }
}

fn no_posts_found_label(language: &str) -> &'static str {
    match language {
        "ru" => "Посты не найдены.",
        "zh" => "未找到文章。",
        "de" => "Keine Beiträge gefunden.",
        "ja" => "投稿が見つかりません。",
        "fr" => "Aucun article trouvé.",
        "es" => "No se encontraron publicaciones.",
        "pt" => "Nenhuma publicação encontrada.",
        "hi" => "कोई पोस्ट नहीं मिली।",
        _ => "No posts found.",
    }
}

fn fallback_notice_text(language: &str, selected_language: &str, default_language: &str) -> String {
    let selected_label = language_label(selected_language);
    let default_label = language_label(default_language);
    match language {
        "ru" => format!(
            "Постов на языке {selected_label} пока нет, показаны посты на языке {default_label}."
        ),
        "zh" => format!("暂时没有{selected_label}文章，改为显示{default_label}。"),
        "de" => format!(
            "Noch keine Beiträge auf {selected_label} verfügbar, es werden {default_label} angezeigt."
        ),
        "ja" => format!(
            "{selected_label}の投稿はまだありません。{default_label}を表示しています。"
        ),
        "fr" => format!(
            "Aucun article en {selected_label} pour le moment, affichage en {default_label}."
        ),
        "es" => format!(
            "Aún no hay publicaciones en {selected_label}, se muestran en {default_label}."
        ),
        "pt" => format!(
            "Ainda não há publicações em {selected_label}, exibindo em {default_label}."
        ),
        "hi" => format!(
            "{selected_label} में अभी कोई पोस्ट नहीं है, {default_label} में दिखाया जा रहा है।"
        ),
        _ => format!("No posts in {selected_label} yet, showing {default_label}."),
    }
}

fn translation_notice_text(
    language: &str,
    selected_language: &str,
    post_language: &str,
    slug: &str,
) -> String {
    let selected_label = language_label(selected_language);
    let post_label = language_label(post_language);
    match language {
        "ru" => format!(
            "Перевод на {selected_label} недоступен, показан {post_label}. Выберите другой язык выше или добавьте переведённый файл .txt со слагом \"{slug}\" и языком \"{selected_language}\"."
        ),
        "zh" => format!(
            "暂无法提供{selected_label}翻译，当前显示{post_label}。请在上方选择其他语言，或添加别名为“{slug}”、语言为“{selected_language}”的 .txt 翻译文件。"
        ),
        "de" => format!(
            "Eine Übersetzung ins {selected_label} ist nicht verfügbar, angezeigt wird {post_label}. Wähle oben eine andere Sprache oder füge eine übersetzte .txt-Datei mit dem Slug „{slug}“ und der Sprache „{selected_language}“ hinzu."
        ),
        "ja" => format!(
            "{selected_label}への翻訳はありません。{post_label}を表示しています。上から別の言語を選ぶか、スラッグ「{slug}」、言語「{selected_language}」の翻訳済み .txt ファイルを追加してください。"
        ),
        "fr" => format!(
            "La traduction en {selected_label} n’est pas disponible, affichage en {post_label}. Choisissez une autre langue ci-dessus ou ajoutez un fichier .txt traduit avec le slug « {slug} » et la langue « {selected_language} »."
        ),
        "es" => format!(
            "La traducción a {selected_label} no está disponible; se muestra en {post_label}. Elige otro idioma arriba o añade un archivo .txt traducido con el slug «{slug}» y el idioma «{selected_language}»."
        ),
        "pt" => format!(
            "A tradução para {selected_label} não está disponível; exibindo em {post_label}. Escolha outro idioma acima ou adicione um arquivo .txt traduzido com o slug «{slug}» e o idioma «{selected_language}»."
        ),
        "hi" => format!(
            "{selected_label} में अनुवाद उपलब्ध नहीं है, {post_label} में दिखाया जा रहा है। ऊपर से दूसरी भाषा चुनें या स्लग \"{slug}\" और भाषा \"{selected_language}\" वाली अनुवादित .txt फ़ाइल जोड़ें।"
        ),
        _ => format!(
            "Translation into {selected_label} is not available, showing {post_label}. Pick another language above or add a translated .txt file with slug \"{slug}\" and language \"{selected_language}\"."
        ),
    }
}

struct CategoryLabels {
    about: &'static str,
    games: &'static str,
    notes: &'static str,
    software: &'static str,
}

fn category_labels(language: &str) -> CategoryLabels {
    match language {
        "ru" => CategoryLabels {
            about: "О нас",
            games: "Игры",
            notes: "Заметки",
            software: "Софт",
        },
        "zh" => CategoryLabels {
            about: "关于",
            games: "游戏",
            notes: "笔记",
            software: "软件",
        },
        "de" => CategoryLabels {
            about: "Über",
            games: "Spiele",
            notes: "Notizen",
            software: "Software",
        },
        "ja" => CategoryLabels {
            about: "概要",
            games: "ゲーム",
            notes: "メモ",
            software: "ソフトウェア",
        },
        "fr" => CategoryLabels {
            about: "À propos",
            games: "Jeux",
            notes: "Notes",
            software: "Logiciel",
        },
        "es" => CategoryLabels {
            about: "Acerca de",
            games: "Juegos",
            notes: "Notas",
            software: "Software",
        },
        "pt" => CategoryLabels {
            about: "Sobre",
            games: "Jogos",
            notes: "Notas",
            software: "Software",
        },
        "hi" => CategoryLabels {
            about: "परिचय",
            games: "खेल",
            notes: "नोट्स",
            software: "सॉफ़्टवेयर",
        },
        _ => CategoryLabels {
            about: "About",
            games: "Games",
            notes: "Notes",
            software: "Software",
        },
    }
}

fn category_label(category: &str, language: &str) -> String {
    let normalized_category = category.trim().to_ascii_lowercase();
    let labels = category_labels(language);
    let localized_category = match normalized_category.as_str() {
        "about" => labels.about,
        "games" => labels.games,
        "notes" => labels.notes,
        "software" => labels.software,
        _ => return capitalize_first(category),
    };
    localized_category.to_string()
}

fn capitalize_first(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first_character) => {
            first_character.to_uppercase().collect::<String>() + characters.as_str()
        }
        None => String::new(),
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

fn choose_language_label(language: &str) -> &'static str {
    match language {
        "ru" => "Выбор языка",
        "zh" => "选择语言",
        "de" => "Sprache wählen",
        "ja" => "言語を選択",
        "fr" => "Choisir la langue",
        "es" => "Elegir idioma",
        "pt" => "Escolher idioma",
        "hi" => "भाषा चुनें",
        _ => "Choose language",
    }
}

fn categories_label(language: &str) -> &'static str {
    match language {
        "ru" => "Категории",
        "zh" => "分类",
        "de" => "Kategorien",
        "ja" => "カテゴリ",
        "fr" => "Catégories",
        "es" => "Categorías",
        "pt" => "Categorias",
        "hi" => "श्रेणियाँ",
        _ => "Categories",
    }
}

fn language_flag(language: &str) -> &'static str {
    match language {
        "ru" => "🇷🇺",
        "en" => "🇬🇧",
        "zh" => "🇨🇳",
        "de" => "🇩🇪",
        "ja" => "🇯🇵",
        "fr" => "🇫🇷",
        "es" => "🇪🇸",
        "pt" => "🇵🇹",
        "hi" => "🇮🇳",
        "uk" => "🇺🇦",
        "it" => "🇮🇹",
        "ko" => "🇰🇷",
        "ar" => "🇸🇦",
        "tr" => "🇹🇷",
        "pl" => "🇵🇱",
        "nl" => "🇳🇱",
        _ => "🏳️",
    }
}
