const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const KNOWN_TAGS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "article",
    "aside",
    "audio",
    "b",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
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
    "head",
    "header",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "mark",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

pub fn normalize_html_body(input: &str) -> String {
    let repaired_attributes = input
        .replace("\"\" rel=", "\" rel=")
        .replace("\"\" target=", "\" target=");
    let mut output = String::with_capacity(repaired_attributes.len());
    let mut open_tags: Vec<String> = Vec::new();
    let mut position = 0;
    while let Some(relative_start) = repaired_attributes[position..].find('<') {
        let start = position + relative_start;
        output.push_str(&repaired_attributes[position..start]);
        if !looks_like_tag_start(&repaired_attributes, start) {
            output.push('<');
            position = start + 1;
            continue;
        }
        let Some(end) = find_tag_end(&repaired_attributes, start) else {
            output.push_str(&repaired_attributes[start..]);
            position = repaired_attributes.len();
            break;
        };
        let token = &repaired_attributes[start..=end];
        if token.starts_with("<!--") {
            output.push_str(token);
            position = end + 1;
            continue;
        }
        let Some(parsed_tag) = parse_tag(token) else {
            output.push_str(token);
            position = end + 1;
            continue;
        };
        let normalized_name = parsed_tag.name.to_lowercase();
        let canonical_name = canonical_tag_name(&normalized_name).unwrap_or(&normalized_name);
        if !KNOWN_TAGS.contains(&canonical_name) {
            output.push_str(&escape_tag(token));
            position = end + 1;
            continue;
        }
        if parsed_tag.closing {
            close_tag(&mut output, &mut open_tags, canonical_name);
        } else {
            close_optional_tags(&mut output, &mut open_tags, canonical_name);
            output.push_str(&rebuild_tag(token, &parsed_tag, canonical_name));
            if !VOID_TAGS.contains(&canonical_name) && !parsed_tag.self_closing {
                open_tags.push(canonical_name.to_string());
            }
        }
        position = end + 1;
    }
    output.push_str(&repaired_attributes[position..]);
    while let Some(tag) = open_tags.pop() {
        output.push_str("</");
        output.push_str(&tag);
        output.push('>');
    }
    output
}

struct ParsedTag<'a> {
    closing: bool,
    name: &'a str,
    name_end: usize,
    self_closing: bool,
}

fn parse_tag(token: &str) -> Option<ParsedTag<'_>> {
    let inner = token.get(1..token.len().checked_sub(1)?)?;
    let mut index = 0;
    skip_whitespace(inner, &mut index);
    let closing = inner.get(index..)?.starts_with('/');
    if closing {
        index += 1;
        skip_whitespace(inner, &mut index);
    }
    let name_start = index;
    while let Some(character) = inner.get(index..)?.chars().next() {
        if character.is_whitespace() || character == '/' {
            break;
        }
        index += character.len_utf8();
    }
    if name_start == index {
        return None;
    }
    Some(ParsedTag {
        closing,
        name: &inner[name_start..index],
        name_end: index,
        self_closing: inner.trim_end().ends_with('/'),
    })
}

fn rebuild_tag(original_token: &str, parsed_tag: &ParsedTag<'_>, canonical_name: &str) -> String {
    let prefix = if parsed_tag.closing { "</" } else { "<" };
    let suffix_start = 1 + parsed_tag.name_end;
    let suffix = original_token.get(suffix_start..).unwrap_or(">");
    let mut tag = String::with_capacity(prefix.len() + canonical_name.len() + suffix.len());
    tag.push_str(prefix);
    tag.push_str(canonical_name);
    tag.push_str(suffix);
    tag
}

fn close_tag(output: &mut String, open_tags: &mut Vec<String>, canonical_name: &str) {
    let Some(stack_index) = open_tags
        .iter()
        .rposition(|open_tag| open_tag == canonical_name)
    else {
        return;
    };
    while open_tags.len() > stack_index {
        if let Some(open_tag) = open_tags.pop() {
            output.push_str("</");
            output.push_str(&open_tag);
            output.push('>');
        }
    }
}

fn close_optional_tags(output: &mut String, open_tags: &mut Vec<String>, canonical_name: &str) {
    if canonical_name == "li" {
        if open_tags.last().map(String::as_str) == Some("p") {
            close_last_tag(output, open_tags);
        }
        if open_tags.last().map(String::as_str) == Some("li") {
            close_last_tag(output, open_tags);
        }
    }
    if canonical_name == "p" && open_tags.last().map(String::as_str) == Some("p") {
        close_last_tag(output, open_tags);
    }
    if is_heading(canonical_name) && open_tags.last().is_some_and(|tag| is_heading(tag)) {
        close_last_tag(output, open_tags);
    }
    if is_block_tag(canonical_name) && open_tags.last().map(String::as_str) == Some("p") {
        close_last_tag(output, open_tags);
    }
}

fn close_last_tag(output: &mut String, open_tags: &mut Vec<String>) {
    if let Some(open_tag) = open_tags.pop() {
        output.push_str("</");
        output.push_str(&open_tag);
        output.push('>');
    }
}

fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}

fn canonical_tag_name(name: &str) -> Option<&'static str> {
    match name {
        "blockzitat" | "citation" | "ब्लॉककोट" | "块引用" | "ブロック引用" => {
            Some("blockquote")
        }
        "heure" | "घंटा" | "小时" => Some("hr"),
        "कोड" => Some("code"),
        "オル" => Some("ol"),
        _ => None,
    }
}

fn skip_whitespace(value: &str, index: &mut usize) {
    while let Some(character) = value.get(*index..).and_then(|rest| rest.chars().next()) {
        if !character.is_whitespace() {
            break;
        }
        *index += character.len_utf8();
    }
}

fn looks_like_tag_start(input: &str, start: usize) -> bool {
    input
        .get(start + 1..)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|character| {
            character == '/' || character == '!' || character == '?' || character.is_alphabetic()
        })
}

fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    input
        .get(start..)?
        .find('>')
        .map(|relative_end| start + relative_end)
}

fn escape_tag(token: &str) -> String {
    token.replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::normalize_html_body;

    #[test]
    fn normalizes_localized_tags() {
        let html =
            "<p><Blockzitat>Text</p></blockquote><p><heure /></p><p><オル></p><li>Item</li></ol>";
        assert_eq!(
            normalize_html_body(html),
            "<p></p><blockquote>Text</blockquote><p></p><hr /><p></p><ol><li>Item</li></ol>"
        );
    }

    #[test]
    fn closes_crossed_list_tags() {
        let html = "<ol><li><p>Text</li><p> folder</p></ol>";
        assert_eq!(
            normalize_html_body(html),
            "<ol><li><p>Text</p></li><p> folder</p></ol>"
        );
    }

    #[test]
    fn escapes_placeholder_tags() {
        assert_eq!(
            normalize_html_body("<p><code>$HOME/<name></code></p>"),
            "<p><code>$HOME/&lt;name&gt;</code></p>"
        );
    }

    #[test]
    fn keeps_comparison_text() {
        assert_eq!(
            normalize_html_body("<p>a < b and c > d</p>"),
            "<p>a < b and c > d</p>"
        );
    }
}
