use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};
use pulldown_cmark::{Event, Parser};

pub(crate) fn is_html_path(path: &std::path::Path) -> bool {
    use ast_grep_core::Language;
    SupportLang::from_path(path) == Some(SupportLang::Html)
}

type HtmlNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

pub(crate) fn html_text_content(raw: &str) -> String {
    let tree = SupportLang::Html.ast_grep(raw);
    let mut content = String::new();
    // Selection purpose never performs fallible raw-span recovery.
    let _ = append_text(&tree.root(), raw, &mut content, HtmlTextPurpose::Selection);
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HtmlAskingText {
    Complete(String),
    Incomplete {
        text: String,
        reason: HtmlProjectionReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HtmlProjectionReason {
    UnrecoverableMarkup,
}
impl HtmlProjectionReason {
    pub(crate) fn cut_marker(self) -> &'static str {
        match self {
            Self::UnrecoverableMarkup => "[cut: html_projection_unrecoverable_markup]",
        }
    }
}

/// Recover malformed source only for asking; selector text keeps its original projection.
pub(crate) fn html_asking_text(raw: &str) -> HtmlAskingText {
    let tree = SupportLang::Html.ast_grep(raw);
    let mut content = String::new();
    let result = append_text(&tree.root(), raw, &mut content, HtmlTextPurpose::Asking);
    let text = content.split_whitespace().collect::<Vec<_>>().join(" ");
    match result {
        Ok(()) => HtmlAskingText::Complete(text),
        Err(reason) => HtmlAskingText::Incomplete { text, reason },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlErrorSpan {
    PlainText,
    MarkupOnly,
    UnrecoverableMarkup,
}

fn classify_error_span(node: &HtmlNode<'_>, raw: &str) -> HtmlErrorSpan {
    if !node.text().contains('<') {
        HtmlErrorSpan::PlainText
    } else if is_markup_only_error(node, raw) {
        HtmlErrorSpan::MarkupOnly
    } else {
        HtmlErrorSpan::UnrecoverableMarkup
    }
}

fn is_markup_only_error(node: &HtmlNode<'_>, raw: &str) -> bool {
    if node.dfs().any(|part| part.is_missing()) {
        return false;
    }
    let mut tags = Vec::new();
    for part in node.dfs() {
        if matches!(
            part.kind().as_ref(),
            "start_tag" | "end_tag" | "self_closing_tag" | "erroneous_end_tag"
        ) {
            tags.push(part.range());
        }
        if part.is_error() {
            let children = part.children().collect::<Vec<_>>();
            for group in children.windows(3) {
                if let Some(range) = cst_closing_tag_range(group, raw) {
                    tags.push(range);
                }
            }
        }
    }
    tags.sort_by_key(|range| (range.start, range.end));
    let mut cursor = node.range().start;
    for tag in tags {
        if tag.end <= cursor {
            continue;
        }
        if tag.start > cursor && !raw[cursor..tag.start].chars().all(char::is_whitespace) {
            return false;
        }
        cursor = tag.end;
    }
    raw[cursor..node.range().end]
        .chars()
        .all(char::is_whitespace)
}

fn cst_closing_tag_range(parts: &[HtmlNode<'_>], raw: &str) -> Option<std::ops::Range<usize>> {
    let [opening, name, closing] = parts else {
        return None;
    };
    if opening.kind() != "</"
        || closing.kind() != ">"
        || !matches!(name.kind().as_ref(), "ERROR" | "tag_name")
        || name.children().len() != 0
        || name.text().is_empty()
        || !name
            .text()
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '-' | ':'))
        || !raw[opening.range().end..name.range().start]
            .chars()
            .all(char::is_whitespace)
        || !raw[name.range().end..closing.range().start]
            .chars()
            .all(char::is_whitespace)
    {
        return None;
    }
    // tree-sitter-html 0.23.2 scan_tag_name accepts alphanumeric, '-' and ':'.
    Some(opening.range().start..closing.range().end)
}

pub(crate) fn html_has_id(raw: &str, id: &str) -> bool {
    let tree = SupportLang::Html.ast_grep(raw);
    tree.root().dfs().any(|element| {
        if !matches!(
            element.kind().as_ref(),
            "element" | "script_element" | "style_element"
        ) {
            return false;
        }
        element
            .children()
            .find(|child| matches!(child.kind().as_ref(), "start_tag" | "self_closing_tag"))
            .and_then(|tag| {
                tag.children().find(|child| {
                    child.kind() == "attribute"
                        && child.children().any(|part| {
                            part.kind() == "attribute_name"
                                && part.text().eq_ignore_ascii_case("id")
                        })
                })
            })
            .is_some_and(|attribute| {
                let value = attribute
                    .dfs()
                    .find(|part| part.kind() == "attribute_value")
                    .map_or_else(String::new, |part| decode_entities(&part.text()));
                value == id
            })
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HtmlTextPurpose {
    Selection,
    Asking,
}

fn append_text(
    node: &HtmlNode<'_>,
    raw: &str,
    content: &mut String,
    purpose: HtmlTextPurpose,
) -> Result<(), HtmlProjectionReason> {
    let mut incomplete = None;
    match node.kind().as_ref() {
        "text" | "entity" => content.push_str(&decode_entities(&node.text())),
        "raw_text" => content.push_str(&node.text()),
        "ERROR" if purpose == HtmlTextPurpose::Asking => match classify_error_span(node, raw) {
            HtmlErrorSpan::PlainText => content.push_str(&decode_entities(&node.text())),
            HtmlErrorSpan::MarkupOnly => content.push(' '),
            HtmlErrorSpan::UnrecoverableMarkup => {
                content.push(' ');
                return Err(HtmlProjectionReason::UnrecoverableMarkup);
            }
        },
        "ERROR" if node.children().len() == 0 => {
            let text = node.text();
            if text.starts_with('&') && !text.contains(['<', '>']) {
                content.push_str(&text);
            }
        }
        "start_tag" | "self_closing_tag" => {
            if purpose == HtmlTextPurpose::Asking && node.dfs().any(|part| part.is_missing()) {
                content.push(' ');
                return Err(HtmlProjectionReason::UnrecoverableMarkup);
            }
            content.push(' ');
            append_tag_attributes(node, content);
            content.push(' ');
        }
        "end_tag" => content.push(' '),
        "document" | "element" | "script_element" | "style_element" | "ERROR" => {
            let is_element = matches!(
                node.kind().as_ref(),
                "element" | "script_element" | "style_element"
            );
            if is_element {
                content.push(' ');
            }
            let mut cursor = node.range().start;
            for child in node.children() {
                let gap = &raw[cursor..child.range().start];
                if gap.chars().all(char::is_whitespace) {
                    content.push_str(gap);
                }
                if let Err(reason) = append_text(&child, raw, content, purpose) {
                    incomplete = Some(reason);
                }
                cursor = child.range().end;
            }
            let gap = &raw[cursor..node.range().end];
            if gap.chars().all(char::is_whitespace) {
                content.push_str(gap);
            }
            if is_element {
                content.push(' ');
            }
        }
        _ => {}
    }
    match incomplete {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

fn append_tag_attributes(tag: &HtmlNode<'_>, content: &mut String) {
    let is_mermaid = tag
        .children()
        .any(|child| child.kind() == "tag_name" && child.text().eq_ignore_ascii_case("ex-mermaid"));
    for attribute in tag.children().filter(|child| child.kind() == "attribute") {
        let Some(name) = attribute
            .children()
            .find(|child| child.kind() == "attribute_name")
        else {
            continue;
        };
        let name = name.text();
        let label = if name.eq_ignore_ascii_case("alt") {
            "alt"
        } else if name.eq_ignore_ascii_case("aria-label") {
            "aria-label"
        } else if is_mermaid && name.eq_ignore_ascii_case("label") {
            "label"
        } else {
            continue;
        };
        if let Some(value) = attribute
            .dfs()
            .find(|part| part.kind() == "attribute_value")
        {
            content.push(' ');
            content.push('[');
            content.push_str(label);
            content.push_str(": ");
            content.push_str(&decode_entities(&value.text()));
            content.push(']');
            content.push(' ');
        }
    }
}

// Parse each complete entity in isolation so Markdown syntax in HTML text stays literal.
pub(crate) fn decode_entities(input: &str) -> String {
    let mut decoded = String::new();
    let mut remainder = input;
    while let Some(start) = remainder.find('&') {
        decoded.push_str(&remainder[..start]);
        remainder = &remainder[start..];
        let Some(end) = remainder.find(';') else {
            break;
        };
        let candidate = &remainder[..=end];
        if candidate.len() <= 33
            && candidate[1..end]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'#')
        {
            for event in Parser::new(candidate) {
                if let Event::Text(text) = event {
                    decoded.push_str(&text);
                }
            }
            remainder = &remainder[end + 1..];
        } else {
            decoded.push('&');
            remainder = &remainder[1..];
        }
    }
    decoded.push_str(remainder);
    decoded
}

#[cfg(test)]
#[path = "html_content_test.rs"]
mod tests;
