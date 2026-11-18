use std::path::{Path, PathBuf};

use crate::model::ValidationIssue;

const IMAGE_EXTENSIONS: [&str; 6] = [".jpg", ".jpeg", ".png", ".webp", ".gif", ".svg"];

#[derive(Debug)]
pub struct RenderedBody {
    pub html: String,
    pub referenced_media: Vec<String>,
    pub issues: Vec<ValidationIssue>,
}

pub fn render_body(lines: &[&str], language: &str, source_path: &Path) -> RenderedBody {
    let mut renderer = BodyRenderer::new(language, source_path);
    for (offset, line) in lines.iter().enumerate() {
        renderer.process_line(line, offset + 6);
    }
    renderer.finish()
}

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

fn escape_code_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        let character = text[index..].chars().next().unwrap_or_default();
        match character {
            '&' => {
                if is_existing_entity(&text[index..]) {
                    escaped.push('&');
                } else {
                    escaped.push_str("&amp;");
                }
            }
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(character),
        }
        index += character.len_utf8();
    }
    escaped
}

fn is_existing_entity(text: &str) -> bool {
    let Some(remainder) = text.strip_prefix('&') else {
        return false;
    };
    let Some(end_index) = remainder.find(';') else {
        return false;
    };
    if end_index == 0 || end_index > 10 {
        return false;
    }
    remainder[..end_index]
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '#')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalizationMode {
    Text,
    OnlyRussian,
    OnlyNonRussian,
}

fn mode_is_included(mode: LocalizationMode, language: &str) -> bool {
    match mode {
        LocalizationMode::Text => true,
        LocalizationMode::OnlyRussian => language == "ru",
        LocalizationMode::OnlyNonRussian => language != "ru",
    }
}

fn filter_localization(line: &str, language: &str) -> Option<String> {
    let mut output = String::new();
    let mut mode = LocalizationMode::Text;
    let mut remaining = line;
    let mut saw_tag = false;
    loop {
        let Some(tag_start) = remaining.find('<') else {
            if mode_is_included(mode, language) {
                output.push_str(remaining);
            }
            break;
        };
        let before_tag = &remaining[..tag_start];
        if mode_is_included(mode, language) {
            output.push_str(before_tag);
        }
        let tail = &remaining[tag_start..];
        let tag = if tail.starts_with("<RUONLY>") {
            Some((LocalizationMode::OnlyRussian, "<RUONLY>".len()))
        } else if tail.starts_with("</RUONLY>") {
            Some((LocalizationMode::Text, "</RUONLY>".len()))
        } else if tail.starts_with("<NOTRU>") {
            Some((LocalizationMode::OnlyNonRussian, "<NOTRU>".len()))
        } else if tail.starts_with("</NOTRU>") {
            Some((LocalizationMode::Text, "</NOTRU>".len()))
        } else {
            None
        };
        match tag {
            Some((next_mode, tag_length)) => {
                saw_tag = true;
                mode = next_mode;
                remaining = &tail[tag_length..];
            }
            None => {
                if mode_is_included(mode, language) {
                    output.push('<');
                }
                remaining = &tail[1..];
            }
        }
    }
    if saw_tag && output.trim().is_empty() {
        None
    } else {
        Some(output)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockState {
    Idle,
    Paragraph,
    UnorderedList,
    OrderedList,
}

#[derive(Debug, Clone, Copy)]
enum CodeState {
    Outside,
    Inside { expecting_language_line: bool },
}

enum MediaReference {
    LocalFile(String),
    RemoteImage(String),
    LinkedImage {
        page_url: String,
        local_file: String,
    },
}

struct BodyRenderer {
    language: String,
    source_path: PathBuf,
    output: String,
    referenced_media: Vec<String>,
    issues: Vec<ValidationIssue>,
    block_state: BlockState,
    code_state: CodeState,
}

impl BodyRenderer {
    fn new(language: &str, source_path: &Path) -> Self {
        Self {
            language: language.to_string(),
            source_path: source_path.to_path_buf(),
            output: String::new(),
            referenced_media: Vec::new(),
            issues: Vec::new(),
            block_state: BlockState::Idle,
            code_state: CodeState::Outside,
        }
    }

    fn process_line(&mut self, raw_line: &str, line_number: usize) {
        match self.code_state {
            CodeState::Inside { .. } => self.process_code_line(raw_line, line_number),
            CodeState::Outside => self.process_body_line(raw_line, line_number),
        }
    }

    fn process_body_line(&mut self, raw_line: &str, line_number: usize) {
        let Some(filtered_line) = filter_localization(raw_line, &self.language) else {
            return;
        };
        let line = filtered_line.trim_end();
        if line.trim().is_empty() {
            self.close_block();
            return;
        }
        if let Some(remainder) = line.trim_start().strip_prefix("[DO NOT PROCESS LINE]") {
            self.close_block();
            self.output.push_str(remainder);
            self.output.push('\n');
            return;
        }
        if self.try_start_code_block(line, line_number) {
            return;
        }
        if let Some(media_reference) = parse_media_reference(line) {
            self.close_block();
            self.write_media_reference(media_reference);
            return;
        }
        let trimmed_line = line.trim();
        if trimmed_line.starts_with('<') {
            self.close_block();
            self.output.push_str(trimmed_line);
            self.output.push('\n');
            self.track_raw_media(trimmed_line);
            return;
        }
        if trimmed_line.starts_with("http://") || trimmed_line.starts_with("https://") {
            self.close_block();
            let link = normalize_link(trimmed_line);
            self.output.push_str("<p class=\"post-link\"><a href=\"");
            self.output.push_str(&escape_html(&link));
            self.output
                .push_str("\" rel=\"noopener\" target=\"_blank\">");
            self.output.push_str(&escape_html(&link));
            self.output.push_str("</a></p>\n");
            return;
        }
        if let Some(item) = trimmed_line
            .strip_prefix("- ")
            .or_else(|| trimmed_line.strip_prefix("* "))
        {
            self.open_list(BlockState::UnorderedList);
            self.output.push_str("<li>");
            self.output.push_str(&convert_inline_code(item));
            self.output.push_str("</li>\n");
            return;
        }
        if let Some(item) = ordered_list_item(trimmed_line) {
            self.open_list(BlockState::OrderedList);
            self.output.push_str("<li>");
            self.output.push_str(&convert_inline_code(item));
            self.output.push_str("</li>\n");
            return;
        }
        if self.block_state != BlockState::Paragraph {
            self.close_block();
            self.block_state = BlockState::Paragraph;
            self.output.push_str("<p>");
        } else {
            self.output.push(' ');
        }
        self.output.push_str(&convert_inline_code(trimmed_line));
    }

    fn try_start_code_block(&mut self, line: &str, line_number: usize) -> bool {
        let lowercase_line = line.to_ascii_lowercase();
        if !lowercase_line.starts_with("<pre") || !lowercase_line.contains("<code") {
            return false;
        }
        self.close_block();
        if !lowercase_line.starts_with("<pre><code>") {
            self.warn(
                line_number,
                "pre tag with attributes normalized to <pre><code>",
            );
        }
        let Some(code_marker_index) = lowercase_line.find("<code>") else {
            return false;
        };
        let content_start = code_marker_index + "<code>".len();
        let remainder = &line[content_start..];
        let lowercase_remainder = remainder.to_ascii_lowercase();
        match find_code_close(&lowercase_remainder) {
            Some(close_index) => {
                self.output.push_str("<pre><code>");
                self.output
                    .push_str(&escape_code_text(remainder[..close_index].trim_end()));
                self.output.push_str("</code></pre>\n");
            }
            None => {
                self.output.push_str("<pre><code>");
                self.code_state = CodeState::Inside {
                    expecting_language_line: true,
                };
                let content = remainder.trim_end();
                if !content.is_empty() {
                    self.process_code_content_line(content, line_number);
                }
            }
        }
        true
    }

    fn process_code_line(&mut self, raw_line: &str, line_number: usize) {
        let lowercase_line = raw_line.to_ascii_lowercase();
        if let Some(close_index) = find_code_close(&lowercase_line) {
            if lowercase_line.contains("</pre></code>") {
                self.warn(
                    line_number,
                    "malformed closing tags normalized to </code></pre>",
                );
            }
            let content_before_close = &raw_line[..close_index];
            if !content_before_close.trim().is_empty() {
                self.process_code_content_line(content_before_close, line_number);
            }
            self.output.push_str("</code></pre>\n");
            self.code_state = CodeState::Outside;
            return;
        }
        self.process_code_content_line(raw_line, line_number);
    }

    fn process_code_content_line(&mut self, line: &str, line_number: usize) {
        let CodeState::Inside {
            expecting_language_line,
        } = self.code_state
        else {
            return;
        };
        if expecting_language_line && line.trim().to_ascii_lowercase().starts_with("language:") {
            self.warn(
                line_number,
                "code block language specification is not part of the blog format and was skipped",
            );
            self.code_state = CodeState::Inside {
                expecting_language_line: false,
            };
            return;
        }
        self.code_state = CodeState::Inside {
            expecting_language_line: false,
        };
        self.output.push_str(&escape_code_text(line));
        self.output.push('\n');
    }

    fn write_media_reference(&mut self, media_reference: MediaReference) {
        match media_reference {
            MediaReference::LocalFile(file) => {
                self.track_media(&file);
                self.output
                    .push_str("<figure class=\"post-image\"><img src=\"/media/");
                self.output.push_str(&urlencoding::encode(&file));
                self.output.push_str("\" alt=\"");
                self.output.push_str(&escape_html(&media_alt_text(&file)));
                self.output.push_str("\"/></figure>\n");
            }
            MediaReference::RemoteImage(url) => {
                self.output
                    .push_str("<figure class=\"post-image\"><img src=\"");
                self.output.push_str(&escape_html(&url));
                self.output.push_str("\" alt=\"\"/></figure>\n");
            }
            MediaReference::LinkedImage {
                page_url,
                local_file,
            } => {
                self.track_media(&local_file);
                self.output
                    .push_str("<figure class=\"post-image\"><a href=\"");
                self.output.push_str(&escape_html(&page_url));
                self.output
                    .push_str("\" rel=\"noopener\" target=\"_blank\"><img src=\"/media/");
                self.output.push_str(&urlencoding::encode(&local_file));
                self.output.push_str("\" alt=\"");
                self.output
                    .push_str(&escape_html(&media_alt_text(&local_file)));
                self.output.push_str("\"/></a></figure>\n");
            }
        }
    }

    fn track_media(&mut self, file: &str) {
        if !self.referenced_media.iter().any(|known| known == file) {
            self.referenced_media.push(file.to_string());
        }
    }

    fn track_raw_media(&mut self, line: &str) {
        for (prefix, quote) in [("src=\"/media/", '"'), ("src='/media/", '\'')] {
            if let Some(start_index) = line.find(prefix) {
                let remainder = &line[start_index + prefix.len()..];
                if let Some(end_index) = remainder.find(quote) {
                    self.track_media(&remainder[..end_index]);
                }
            }
        }
    }

    fn open_list(&mut self, list_state: BlockState) {
        if self.block_state != list_state {
            self.close_block();
            self.block_state = list_state;
            self.output.push_str(match list_state {
                BlockState::UnorderedList => "<ul>\n",
                BlockState::OrderedList => "<ol>\n",
                _ => "",
            });
        }
    }

    fn close_block(&mut self) {
        match self.block_state {
            BlockState::Idle => {}
            BlockState::Paragraph => self.output.push_str("</p>\n"),
            BlockState::UnorderedList => self.output.push_str("</ul>\n"),
            BlockState::OrderedList => self.output.push_str("</ol>\n"),
        }
        self.block_state = BlockState::Idle;
    }

    fn warn(&mut self, line_number: usize, message: &str) {
        self.issues.push(ValidationIssue::warning(
            self.source_path.clone(),
            Some(line_number),
            message,
        ));
    }

    fn finish(mut self) -> RenderedBody {
        if matches!(self.code_state, CodeState::Inside { .. }) {
            self.output.push_str("</code></pre>\n");
            self.code_state = CodeState::Outside;
            self.issues.push(ValidationIssue::warning(
                self.source_path.clone(),
                None,
                "code block was not closed, closing tags were appended",
            ));
        }
        self.close_block();
        RenderedBody {
            html: self.output,
            referenced_media: self.referenced_media,
            issues: self.issues,
        }
    }
}

fn find_code_close(lowercase_text: &str) -> Option<usize> {
    let code_close = lowercase_text.find("</code>");
    let pre_close = lowercase_text.find("</pre>");
    match (code_close, pre_close) {
        (Some(code_index), Some(pre_index)) => Some(code_index.min(pre_index)),
        (Some(code_index), None) => Some(code_index),
        (None, Some(pre_index)) => Some(pre_index),
        (None, None) => None,
    }
}

fn ordered_list_item(text: &str) -> Option<&str> {
    let digit_count = text
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return None;
    }
    text[digit_count..].strip_prefix(". ")
}

fn normalize_link(link: &str) -> String {
    match link.strip_prefix("http://") {
        Some(remainder) => format!("https://{remainder}"),
        None => link.to_string(),
    }
}

fn has_image_extension(file: &str) -> bool {
    let lowercase_file = file.to_ascii_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|extension| lowercase_file.ends_with(*extension))
}

fn parse_media_reference(line: &str) -> Option<MediaReference> {
    let trimmed_line = line.trim();
    if trimmed_line.starts_with('<') {
        return None;
    }
    if let Some((first_part, second_part)) = trimmed_line.split_once('|') {
        let page_url = first_part.trim();
        let file = second_part.trim();
        if (page_url.starts_with("http://") || page_url.starts_with("https://"))
            && has_image_extension(file)
        {
            return Some(MediaReference::LinkedImage {
                page_url: normalize_link(page_url),
                local_file: file.to_string(),
            });
        }
    }
    if has_image_extension(trimmed_line) && !trimmed_line.contains(char::is_whitespace) {
        if trimmed_line.starts_with("http://") || trimmed_line.starts_with("https://") {
            return Some(MediaReference::RemoteImage(normalize_link(trimmed_line)));
        }
        return Some(MediaReference::LocalFile(trimmed_line.to_string()));
    }
    None
}

fn media_alt_text(file: &str) -> String {
    let stem = Path::new(file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file);
    stem.replace(['-', '_'], " ")
}

fn convert_inline_code(text: &str) -> String {
    let mut segments = text.split('`');
    let mut output = String::new();
    if let Some(first_segment) = segments.next() {
        output.push_str(first_segment);
    }
    for (segment_index, segment) in segments.enumerate() {
        if segment_index % 2 == 0 {
            output.push_str("<code>");
            output.push_str(&escape_code_text(segment));
            output.push_str("</code>");
        } else {
            output.push_str(segment);
        }
    }
    output
}
