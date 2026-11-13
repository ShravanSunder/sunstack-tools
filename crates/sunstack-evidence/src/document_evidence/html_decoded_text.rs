use super::html_content::decode_entities;
use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};
use std::ops::Range;

type HtmlNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

pub(crate) struct DecodedHtmlText {
    pub value: String,
    pub range: Range<usize>,
}

/// Mechanical text preserves characters and whitespace; asking projection is a separate view.
pub(crate) fn decoded_html_text(raw: &str) -> Result<String, crate::FieldUnavailable> {
    let tree = SupportLang::Html.ast_grep(raw);
    if tree
        .root()
        .dfs()
        .any(|node| node.is_error() || node.is_missing())
    {
        return Err(crate::FieldUnavailable::Unreadable);
    }
    let mut value = String::new();
    append_decoded_children(&tree.root(), raw, &mut value)?;
    Ok(value)
}

fn append_decoded_children(
    node: &HtmlNode<'_>,
    raw: &str,
    value: &mut String,
) -> Result<(), crate::FieldUnavailable> {
    match node.kind().as_ref() {
        "text" | "entity" => value.push_str(&decode_entities(&node.text())),
        "raw_text" => value.push_str(&node.text()),
        "document" | "element" | "script_element" | "style_element" => {
            let mut cursor = node.range().start;
            for child in node.children() {
                let gap = &raw[cursor..child.range().start];
                if !gap.chars().all(char::is_whitespace) {
                    return Err(crate::FieldUnavailable::Unreadable);
                }
                value.push_str(gap);
                append_decoded_children(&child, raw, value)?;
                cursor = child.range().end;
            }
            let gap = &raw[cursor..node.range().end];
            if !gap.chars().all(char::is_whitespace) {
                return Err(crate::FieldUnavailable::Unreadable);
            }
            value.push_str(gap);
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn decoded_html_attribute(
    raw: &str,
    name: &crate::HtmlAttributeName,
) -> Result<Option<DecodedHtmlText>, crate::FieldUnavailable> {
    let tree = SupportLang::Html.ast_grep(raw);
    if tree
        .root()
        .dfs()
        .any(|node| node.is_error() || node.is_missing())
    {
        return Err(crate::FieldUnavailable::Unreadable);
    }
    let element = tree
        .root()
        .children()
        .find(|node| {
            matches!(
                node.kind().as_ref(),
                "element" | "script_element" | "style_element"
            ) && node.range() == (0..raw.len())
        })
        .ok_or(crate::FieldUnavailable::Unreadable)?;
    let tag = element
        .children()
        .find(|node| matches!(node.kind().as_ref(), "start_tag" | "self_closing_tag"))
        .ok_or(crate::FieldUnavailable::Unreadable)?;
    let attribute = tag.children().find(|node| {
        node.kind() == "attribute"
            && node.children().any(|part| {
                part.kind() == "attribute_name" && part.text().eq_ignore_ascii_case(name.as_str())
            })
    });
    Ok(attribute.map(|attribute| {
        attribute
            .dfs()
            .find(|node| node.kind() == "attribute_value")
            .map_or_else(
                || DecodedHtmlText {
                    value: String::new(),
                    range: attribute.range(),
                },
                |value| DecodedHtmlText {
                    value: decode_entities(&value.text()),
                    range: value.range(),
                },
            )
    }))
}

#[cfg(test)]
#[path = "html_decoded_text_test.rs"]
mod tests;
