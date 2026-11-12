use super::{MarkdownDocument, heading_level};
use crate::document_evidence::{
    github_heading_slug::GithubHeadingSlugger,
    html_content::decode_entities,
    markdown_offset_projection::{NormalizedMarkdown, OffsetProjection},
};
use ast_grep_language::{LanguageExt, SupportLang};

impl MarkdownDocument {
    pub(crate) fn has_anchor(&self, fragment: &str) -> bool {
        let normalized = NormalizedMarkdown::new(self.tree.source());
        let projection = OffsetProjection::new(&normalized);
        let mut slugger = GithubHeadingSlugger::new();
        let has_heading = self
            .tree
            .root()
            .dfs()
            .filter(|node| {
                heading_level(node).is_some()
                    && !node.ancestors().any(|ancestor| {
                        matches!(
                            ancestor.kind().as_ref(),
                            "html_block"
                                | "minus_metadata"
                                | "plus_metadata"
                                | "fenced_code_block"
                                | "indented_code_block"
                        )
                    })
            })
            .any(|heading| slugger.slug(&projection.prose(heading.range(), false).0) == fragment);
        has_heading || has_explicit_anchor(&projection, fragment)
    }
}

fn has_explicit_anchor(projection: &OffsetProjection<'_>, fragment: &str) -> bool {
    let mut combined = String::new();
    let mut previous_end = None;
    for (html, range) in projection.html_fragments() {
        if previous_end.is_some_and(|end| end != range.start) {
            if html_fragment_has_anchor(&combined, fragment) {
                return true;
            }
            combined.clear();
        }
        combined.push_str(html);
        previous_end = Some(range.end);
    }
    html_fragment_has_anchor(&combined, fragment)
}

fn html_fragment_has_anchor(html: &str, fragment: &str) -> bool {
    let tree = SupportLang::Html.ast_grep(html);
    tree.root()
        .dfs()
        .filter(|node| matches!(node.kind().as_ref(), "start_tag" | "self_closing_tag"))
        .any(|tag| {
            let is_anchor = tag
                .children()
                .any(|child| child.kind() == "tag_name" && child.text().eq_ignore_ascii_case("a"));
            tag.children()
                .filter(|child| child.kind() == "attribute")
                .any(|attribute| {
                    let matches_name = attribute.children().any(|part| {
                        part.kind() == "attribute_name"
                            && (part.text().eq_ignore_ascii_case("id")
                                || (is_anchor && part.text().eq_ignore_ascii_case("name")))
                    });
                    matches_name
                        && attribute
                            .dfs()
                            .find(|part| part.kind() == "attribute_value")
                            .is_some_and(|value| decode_entities(&value.text()) == fragment)
                })
        })
}

#[cfg(test)]
#[path = "heading_anchors_test.rs"]
mod tests;
