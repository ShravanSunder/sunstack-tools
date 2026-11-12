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
    append_text(&tree.root(), raw, &mut content);
    content.split_whitespace().collect::<Vec<_>>().join(" ")
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

fn append_text(node: &HtmlNode<'_>, raw: &str, content: &mut String) {
    match node.kind().as_ref() {
        "text" | "entity" => content.push_str(&decode_entities(&node.text())),
        "raw_text" => content.push_str(&node.text()),
        "ERROR" if node.children().len() == 0 => {
            let text = node.text();
            if text.starts_with('&') && !text.contains(['<', '>']) {
                content.push_str(&text);
            }
        }
        "start_tag" | "self_closing_tag" => append_tag_attributes(node, content),
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
                append_text(&child, raw, content);
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
