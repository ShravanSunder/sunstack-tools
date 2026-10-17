use ast_grep_core::{
    Language, Node, Pattern, PatternError,
    matcher::PatternBuilder,
    tree_sitter::{LanguageExt, StrDoc, TSLanguage},
};
use std::collections::BTreeSet;

// ast-grep exposes Markdown's block grammar; its inline grammar uses the same adapter API.
#[derive(Clone)]
struct MarkdownInline;
impl Language for MarkdownInline {
    fn kind_to_id(&self, kind: &str) -> u16 {
        self.get_ts_language().id_for_node_kind(kind, true)
    }
    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.get_ts_language()
            .field_id_for_name(field)
            .map(|id| id.get())
    }
    fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
        builder.build(|source| StrDoc::try_new(source, self.clone()))
    }
}
impl LanguageExt for MarkdownInline {
    fn get_ts_language(&self) -> TSLanguage {
        tree_sitter_md::INLINE_LANGUAGE.into()
    }
}

pub(crate) fn inside_code_span(text: &str, offset: usize) -> bool {
    MarkdownInline
        .ast_grep(text)
        .root()
        .dfs()
        .any(|node| node.kind() == "code_span" && node.range().contains(&offset))
}

pub(crate) fn reference_label(text: &str) -> String {
    text.trim_start_matches('[')
        .trim_end_matches(']')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn project_inline(text: &str, references: &BTreeSet<String>) -> (String, String) {
    let tree = MarkdownInline.ast_grep(text);
    (
        render_inline(tree.root(), true, references),
        render_inline(tree.root(), false, references),
    )
}

fn render_inline(
    node: Node<'_, StrDoc<MarkdownInline>>,
    keep_code: bool,
    references: &BTreeSet<String>,
) -> String {
    let text = node.text();
    match node.kind().as_ref() {
        "html_tag"
        | "emphasis_delimiter"
        | "code_span_delimiter"
        | "link_destination"
        | "link_title" => String::new(),
        "code_span" => {
            if !keep_code {
                return String::new();
            }
            let content = text.trim_matches('`').replace(['\n', '\r'], " ");
            if content.starts_with(' ') && content.ends_with(' ') && !content.trim().is_empty() {
                content[1..content.len() - 1].to_owned()
            } else {
                content
            }
        }
        "image" => node
            .children()
            .find(|child| child.kind() == "image_description")
            .map_or_else(String::new, |child| {
                render_inline(child, keep_code, references)
            }),
        "inline_link" => node
            .children()
            .find(|child| child.kind() == "link_text")
            .map_or_else(String::new, |child| {
                render_inline(child, keep_code, references)
            }),
        "full_reference_link" | "collapsed_reference_link" | "shortcut_link" => {
            let label = node
                .children()
                .find(|child| child.kind() == "link_label")
                .or_else(|| node.children().find(|child| child.kind() == "link_text"));
            if label
                .as_ref()
                .is_some_and(|label| references.contains(&reference_label(&label.text())))
            {
                node.children()
                    .find(|child| matches!(child.kind().as_ref(), "link_text" | "link_label"))
                    .map_or_else(String::new, |child| {
                        render_inline(child, keep_code, references)
                    })
            } else {
                text.into_owned()
            }
        }
        "uri_autolink" | "email_autolink" => text
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_owned(),
        "entity_reference" | "numeric_character_reference" => {
            html_escape::decode_html_entities(&text).into_owned()
        }
        "backslash_escape" => text.strip_prefix('\\').unwrap_or(&text).to_owned(),
        _ => {
            let mut output = String::new();
            let mut offset = 0;
            for child in node.children() {
                let range = child.range();
                let start = range.start - node.range().start;
                let end = range.end - node.range().start;
                output.push_str(&text[offset..start]);
                output.push_str(&render_inline(child, keep_code, references));
                offset = end;
            }
            output.push_str(&text[offset..]);
            output
        }
    }
}
