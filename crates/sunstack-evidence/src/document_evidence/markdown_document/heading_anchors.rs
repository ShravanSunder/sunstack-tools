use super::{MarkdownDocument, heading_level};
use crate::document_evidence::{
    heading_anchor_schemes::HeadingAnchorScheme,
    html_content::decode_entities,
    markdown_offset_projection::{NormalizedMarkdown, OffsetProjection},
};
use crate::{TargetAnchor, TargetAnchorUnsupportedReason};
use ast_grep_language::{LanguageExt, SupportLang};

impl MarkdownDocument {
    pub(crate) fn resolve_anchor(
        &self,
        fragment: String,
        scheme: Option<HeadingAnchorScheme>,
    ) -> TargetAnchor {
        if self.anchor_range(&fragment, scheme).is_some() {
            TargetAnchor::Found { fragment }
        } else if scheme.is_none() {
            TargetAnchor::Unsupported {
                fragment,
                reason: TargetAnchorUnsupportedReason::HeadingSchemeNotConfigured,
            }
        } else {
            TargetAnchor::Missing { fragment }
        }
    }

    pub(crate) fn anchor_range(
        &self,
        fragment: &str,
        scheme: Option<HeadingAnchorScheme>,
    ) -> Option<std::ops::Range<usize>> {
        let normalized = NormalizedMarkdown::new(self.tree.source());
        let projection = OffsetProjection::new(&normalized);
        if let Some(range) = explicit_anchor_range(&projection, fragment) {
            return Some(normalized.original_range(range));
        }
        let mut slugger = scheme?.slugger();
        self.tree
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
            .find(|heading| slugger.slug(&projection.heading_text(heading.range())) == fragment)
            .map(|heading| heading.range())
    }
}

fn explicit_anchor_range(
    projection: &OffsetProjection<'_>,
    fragment: &str,
) -> Option<std::ops::Range<usize>> {
    let mut combined = String::new();
    let mut previous_end = None;
    let mut combined_start = 0;
    for (html, range) in projection.html_fragments() {
        if previous_end.is_some_and(|end| end != range.start) {
            if let Some(range) = html_fragment_anchor_range(&combined, fragment) {
                return Some(combined_start + range.start..combined_start + range.end);
            }
            combined.clear();
        }
        if combined.is_empty() {
            combined_start = range.start;
        }
        combined.push_str(html);
        previous_end = Some(range.end);
    }
    html_fragment_anchor_range(&combined, fragment)
        .map(|range| combined_start + range.start..combined_start + range.end)
}

fn html_fragment_anchor_range(html: &str, fragment: &str) -> Option<std::ops::Range<usize>> {
    let tree = SupportLang::Html.ast_grep(html);
    tree.root()
        .dfs()
        .filter(|node| matches!(node.kind().as_ref(), "start_tag" | "self_closing_tag"))
        .find(|tag| {
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
        .map(|tag| tag.range())
}

#[cfg(test)]
#[path = "heading_anchors_test.rs"]
mod tests;
