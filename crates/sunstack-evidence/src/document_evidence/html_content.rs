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
    let _ = append_text(
        &tree.root(),
        raw,
        &mut content,
        HtmlTextPurpose::Selection,
        false,
        &mut 0,
    );
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
    UnterminatedMarkup,
}
impl HtmlProjectionReason {
    pub(crate) fn cut_marker(self) -> &'static str {
        match self {
            Self::UnterminatedMarkup => "[cut: html_projection_unterminated_markup]",
        }
    }
}

/// Recover malformed source only for asking; selector text keeps its original projection.
pub(crate) fn html_asking_text(raw: &str) -> HtmlAskingText {
    let tree = SupportLang::Html.ast_grep(raw);
    let mut content = String::new();
    let result = append_text(
        &tree.root(),
        raw,
        &mut content,
        HtmlTextPurpose::Asking,
        false,
        &mut 0,
    );
    let text = content.split_whitespace().collect::<Vec<_>>().join(" ");
    match result {
        Ok(()) => HtmlAskingText::Complete(text),
        Err(reason) => HtmlAskingText::Incomplete { text, reason },
    }
}

// ERROR leaves can contain both markup and prose with no useful child-node boundaries.
// Strip complete tag-like tokens from that source span; never guess past an unfinished token.
fn append_recovered_text(
    node: &HtmlNode<'_>,
    document: &str,
    content: &mut String,
    consumed_until: &mut usize,
) -> Result<(), HtmlProjectionReason> {
    let range = node.range();
    let mut raw = &document[range.start.max(*consumed_until)..range.end];
    let tags = node
        .dfs()
        .filter(|part| matches!(part.kind().as_ref(), "start_tag" | "self_closing_tag"))
        .map(|tag| (tag.range().start, tag))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut excluded_tags: Vec<String> = Vec::new();
    while !raw.is_empty() {
        let start = raw.find('<').unwrap_or(raw.len());
        if excluded_tags.is_empty() {
            content.push_str(&decode_entities(&raw[..start]));
        }
        raw = &raw[start..];
        if raw.is_empty() {
            break;
        }
        if raw.starts_with("<![CDATA[") {
            // A CDATA block can cross ERROR, entity and element node boundaries.
            let start = range.end - raw.len() + "<![CDATA[".len();
            let cdata = &document[start..];
            let end = cdata
                .find("]]>")
                .ok_or(HtmlProjectionReason::UnterminatedMarkup)?;
            if excluded_tags.is_empty() {
                content.push(' ');
                content.push_str(&cdata[..end]);
                content.push(' ');
            }
            let end = start + end + "]]>".len();
            *consumed_until = (*consumed_until).max(end);
            if end >= range.end {
                return Ok(());
            }
            raw = &document[end..range.end];
            continue;
        }
        if let Some(comment) = raw.strip_prefix("<!--") {
            let Some(end) = comment.find("-->") else {
                return if excluded_tags.is_empty() {
                    Err(HtmlProjectionReason::UnterminatedMarkup)
                } else {
                    Ok(())
                };
            };
            raw = &comment[end + 3..];
            content.push(' ');
            continue;
        }
        let Some(end) = recovered_tag_end(raw) else {
            return if excluded_tags.is_empty() {
                Err(HtmlProjectionReason::UnterminatedMarkup)
            } else {
                Ok(())
            };
        };
        let tag_start = range.end - raw.len();
        let token = raw[1..end].trim();
        let closing = token.starts_with('/');
        let name = token.trim_start_matches('/').trim_start();
        let name_end = name
            .find(|character: char| {
                !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_' | ':')
            })
            .unwrap_or(name.len());
        let name = &name[..name_end];
        if recovery_excluded_tag(name) {
            if closing {
                if let Some(index) = excluded_tags
                    .iter()
                    .rposition(|open| open.eq_ignore_ascii_case(name))
                {
                    excluded_tags.truncate(index);
                }
            } else {
                // HTML ignores the self-closing slash on these non-void elements.
                excluded_tags.push(name.to_ascii_lowercase());
            }
        }
        if excluded_tags.is_empty() {
            content.push(' ');
            if let Some(tag) = tags.get(&tag_start) {
                append_tag_attributes(tag, content);
            }
            content.push(' ');
        }
        raw = &raw[end + 1..];
    }
    Ok(())
}

fn recovered_tag_end(raw: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, character) in raw.char_indices().skip(1) {
        match (quote, character) {
            (Some(expected), actual) if expected == actual => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '>') => return Some(offset),
            (None, '<') => return None,
            (None, _) => {}
        }
    }
    None
}

fn recovery_excluded_tag(name: &str) -> bool {
    ["script", "style", "template", "ex-markdown"]
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
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
    excluded_context: bool,
    consumed_until: &mut usize,
) -> Result<(), HtmlProjectionReason> {
    if purpose == HtmlTextPurpose::Asking && node.range().start < *consumed_until {
        if node.range().end > *consumed_until && !excluded_context {
            append_recovered_text(node, raw, content, consumed_until)?;
        }
        return Ok(());
    }
    match node.kind().as_ref() {
        "text" | "entity" => content.push_str(&decode_entities(&node.text())),
        "raw_text" => content.push_str(&node.text()),
        "ERROR"
            if node.children().len() == 0
                && node.text().starts_with('&')
                && !node.text().contains(['<', '>']) =>
        {
            // These existing ampersand leaves were already visible, not lost prose.
            content.push_str(&node.text());
        }
        "ERROR" if purpose == HtmlTextPurpose::Asking => {
            if !excluded_context {
                append_recovered_text(node, raw, content, consumed_until)?;
            }
        }
        "ERROR" if node.children().len() == 0 => {}
        "start_tag" | "self_closing_tag" => {
            if purpose == HtmlTextPurpose::Asking && node.dfs().any(|part| part.is_missing()) {
                if !excluded_context {
                    append_recovered_text(node, raw, content, consumed_until)?;
                }
            } else {
                content.push(' ');
                append_tag_attributes(node, content);
                content.push(' ');
            }
        }
        "end_tag" => content.push(' '),
        "document" | "element" | "script_element" | "style_element" | "ERROR" => {
            let is_element = matches!(
                node.kind().as_ref(),
                "element" | "script_element" | "style_element"
            );
            let excluded_context = purpose == HtmlTextPurpose::Asking
                && (excluded_context
                    || node.children().any(|child| {
                        child.kind() == "start_tag"
                            && child.children().any(|part| {
                                part.kind() == "tag_name" && recovery_excluded_tag(&part.text())
                            })
                    }));
            if is_element {
                content.push(' ');
            }
            let mut cursor = node.range().start;
            for child in node.children() {
                let gap = &raw[cursor..child.range().start];
                if gap.chars().all(char::is_whitespace) {
                    content.push_str(gap);
                }
                append_text(
                    &child,
                    raw,
                    content,
                    purpose,
                    excluded_context,
                    consumed_until,
                )?;
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
    Ok(())
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
