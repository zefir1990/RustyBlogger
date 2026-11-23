pub fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

const BLOCK_ELEMENT_NAMES: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "details",
    "dialog",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "iframe",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
    "video",
];

pub fn render_post_body(body_html: &str) -> String {
    let mut rendered_body = String::with_capacity(body_html.len());
    let mut pending_text = String::new();
    let mut open_block_depth = 0usize;
    let mut position = 0usize;
    while position < body_html.len() {
        let current_character = body_html[position..]
            .chars()
            .next()
            .expect("position points to a valid character boundary");
        if current_character == '<' {
            if let Some(tag_end) = find_tag_end(body_html, position) {
                let tag = &body_html[position..tag_end];
                let tag_name = parse_tag_name(tag);
                let closes_element = tag.starts_with("</");
                let self_closing =
                    tag.trim_end().ends_with("/>") || tag_name.as_deref() == Some("hr");
                let is_block = tag_name
                    .as_deref()
                    .map(|name| BLOCK_ELEMENT_NAMES.contains(&name))
                    .unwrap_or(false);
                if open_block_depth == 0 {
                    if is_block {
                        flush_pending_text(&mut pending_text, &mut rendered_body);
                        rendered_body.push_str(tag);
                        if !closes_element && !self_closing {
                            open_block_depth += 1;
                        }
                    } else {
                        pending_text.push_str(tag);
                    }
                } else {
                    rendered_body.push_str(tag);
                    if is_block {
                        if closes_element {
                            open_block_depth -= 1;
                        } else if !self_closing {
                            open_block_depth += 1;
                        }
                    }
                }
                position = tag_end;
                continue;
            }
        }
        if open_block_depth > 0 {
            rendered_body.push(current_character);
        } else {
            pending_text.push(current_character);
        }
        position += current_character.len_utf8();
    }
    flush_pending_text(&mut pending_text, &mut rendered_body);
    rendered_body
}

fn find_tag_end(html: &str, tag_start: usize) -> Option<usize> {
    let mut quote_character: Option<char> = None;
    for (offset, character) in html[tag_start..].char_indices() {
        match quote_character {
            Some(quote) => {
                if character == quote {
                    quote_character = None;
                }
            }
            None => match character {
                '"' | '\'' => quote_character = Some(character),
                '>' => return Some(tag_start + offset + character.len_utf8()),
                _ => {}
            },
        }
    }
    None
}

fn parse_tag_name(tag: &str) -> Option<String> {
    let trimmed_tag = tag
        .trim_start_matches('<')
        .trim_start_matches('/')
        .trim_start();
    let name_end = trimmed_tag
        .find(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == ':')
        })
        .unwrap_or(trimmed_tag.len());
    if name_end == 0 {
        return None;
    }
    Some(trimmed_tag[..name_end].to_ascii_lowercase())
}

fn flush_pending_text(pending_text: &mut String, rendered_body: &mut String) {
    let normalized_text = pending_text.replace("\r\n", "\n").replace('\r', "\n");
    pending_text.clear();
    let mut paragraph_lines: Vec<&str> = Vec::new();
    for line in normalized_text.split('\n') {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            render_paragraph(&mut paragraph_lines, rendered_body);
        } else {
            paragraph_lines.push(trimmed_line);
        }
    }
    render_paragraph(&mut paragraph_lines, rendered_body);
}

fn render_paragraph(paragraph_lines: &mut Vec<&str>, rendered_body: &mut String) {
    if paragraph_lines.is_empty() {
        return;
    }
    rendered_body.push_str("<p>");
    for (line_index, line) in paragraph_lines.iter().enumerate() {
        if line_index > 0 {
            rendered_body.push_str("<br />");
        }
        rendered_body.push_str(line);
    }
    rendered_body.push_str("</p>");
    paragraph_lines.clear();
}

#[cfg(test)]
mod tests {
    use super::render_post_body;

    #[test]
    fn wraps_plain_lines_into_paragraphs() {
        let body = "Первый абзац.\nВторая строка.\n\nВторой абзац.";
        assert_eq!(
            render_post_body(body),
            "<p>Первый абзац.<br />Вторая строка.</p><p>Второй абзац.</p>"
        );
    }

    #[test]
    fn preserves_existing_block_elements() {
        let body = "<p>Готовый абзац.</p>\n<h2>Заголовок</h2>\nТекст после заголовка.";
        assert_eq!(
            render_post_body(body),
            "<p>Готовый абзац.</p><h2>Заголовок</h2><p>Текст после заголовка.</p>"
        );
    }

    #[test]
    fn preserves_newlines_inside_code_blocks() {
        let body = "<div class=\"hcb_wrap\"><pre><code>cargo build \\\n  --release\n</code></pre></div>\nПосле кода.";
        assert_eq!(
            render_post_body(body),
            "<div class=\"hcb_wrap\"><pre><code>cargo build \\\n  --release\n</code></pre></div><p>После кода.</p>"
        );
    }

    #[test]
    fn keeps_void_block_elements_from_opening_block() {
        let body = "<hr>\nТекст после линии.";
        assert_eq!(render_post_body(body), "<hr><p>Текст после линии.</p>");
    }

    #[test]
    fn keeps_inline_tags_within_paragraph() {
        let body = "<img src=\"/media/zefir1990.jpg\"/>\nТекст\nсо ссылкой <a href=\"https://example.com\">example</a>.";
        assert_eq!(
            render_post_body(body),
            "<p><img src=\"/media/zefir1990.jpg\"/><br />Текст<br />со ссылкой <a href=\"https://example.com\">example</a>.</p>"
        );
    }
}
