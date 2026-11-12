use super::*;
use crate::document_evidence::{
    html_content::html_text_content,
    html_outline::{HtmlSection, html_sections},
    markdown_document::{MarkdownDocument, is_markdown_path, section_outline::MarkdownSection},
};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct HtmlMatchedKind {
    outline: Arc<Vec<HtmlSection>>,
    selected: Vec<usize>,
    limited: bool,
}
#[derive(Clone)]
pub(super) struct MarkdownMatchedKind {
    outline: Arc<Vec<MarkdownSection>>,
    selected: Vec<MatchRange>,
    limited: bool,
}
#[derive(Clone)]
pub(super) enum NativeMatchedKind {
    Html(HtmlMatchedKind),
    Markdown(MarkdownMatchedKind),
}
impl NativeMatchedKind {
    pub(super) fn is_limited(&self) -> bool {
        match self {
            Self::Html(value) => value.limited,
            Self::Markdown(value) => value.limited,
        }
    }
    fn match_count(&self) -> usize {
        match self {
            Self::Html(value) => value.selected.len(),
            Self::Markdown(value) => value.selected.len(),
        }
    }
    fn ranges(&self) -> Vec<MatchRange> {
        match self {
            Self::Html(value) => value
                .selected
                .iter()
                .filter_map(|index| value.outline.get(*index))
                .map(|section| match_range(&section.section_range))
                .collect(),
            Self::Markdown(value) => value.selected.clone(),
        }
    }
    fn section_ranges(
        &self,
        range: &MatchRange,
        mode: crate::SectionText,
    ) -> Result<Option<Vec<MatchRange>>, FieldUnavailable> {
        match self {
            Self::Html(value) => {
                let section = value
                    .outline
                    .iter()
                    .find(|section| same_range(&section.section_range, range))
                    .ok_or(FieldUnavailable::Unreadable)?;
                let selected = match mode {
                    crate::SectionText::Full => &section.section_range,
                    crate::SectionText::Own => &section.own_range,
                    crate::SectionText::EnclosingOwn => {
                        let Some(index) = section.enclosing_index else {
                            return Ok(None);
                        };
                        &value
                            .outline
                            .get(index)
                            .ok_or(FieldUnavailable::Unreadable)?
                            .own_range
                    }
                };
                Ok(Some(vec![match_range(selected)]))
            }
            Self::Markdown(value) => {
                let section = value
                    .outline
                    .iter()
                    .find(|section| same_range(&section.section_range, range))
                    .ok_or(FieldUnavailable::Unreadable)?;
                let ranges = match mode {
                    crate::SectionText::Full => vec![match_range(&section.section_range)],
                    crate::SectionText::Own => section.own_ranges.iter().map(match_range).collect(),
                    crate::SectionText::EnclosingOwn => {
                        let Some(index) = section.enclosing_index else {
                            return Ok(None);
                        };
                        value
                            .outline
                            .get(index)
                            .ok_or(FieldUnavailable::Unreadable)?
                            .own_ranges
                            .iter()
                            .map(match_range)
                            .collect()
                    }
                };
                Ok(Some(ranges))
            }
        }
    }
}
fn match_range(range: &std::ops::Range<usize>) -> MatchRange {
    MatchRange {
        start: range.start,
        end: range.end,
    }
}
fn same_range(section: &std::ops::Range<usize>, range: &MatchRange) -> bool {
    section.start == range.start && section.end == range.end
}

pub(super) fn collect_native_units(
    path: &Path,
    text: &str,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    source: &SourceRef,
    remaining: &mut usize,
) -> Result<BTreeMap<EvidenceKindName, NativeMatchedKind>, String> {
    let html = if is_html_path(path)
        && kinds
            .values()
            .any(|kind| matches!(kind.matcher, crate::KindMatcher::HtmlUnits(_)))
    {
        Some(Arc::new(
            html_sections(text).map_err(|error| error.to_string())?,
        ))
    } else {
        None
    };
    let markdown = (is_markdown_path(path)
        && kinds
            .values()
            .any(|kind| matches!(kind.matcher, crate::KindMatcher::MarkdownUnits(_))))
    .then(|| MarkdownDocument::parse(text));
    let markdown_outline = markdown
        .as_ref()
        .map(|document| Arc::new(document.section_outline()));
    let mut matches = BTreeMap::new();
    for (name, definition) in kinds {
        match &definition.matcher {
            crate::KindMatcher::HtmlUnits(selection) => {
                let Some(outline) = &html else { continue };
                selection
                    .validate()
                    .map_err(|error| format!("html_units {}: {error}", name.as_str()))?;
                let mut selected = Vec::new();
                let mut limited = false;
                for (index, section) in outline.iter().enumerate() {
                    let visible = html_text_content(&text[section.section_range.clone()]);
                    if !selection
                        .matches_text(&visible)
                        .map_err(|error| format!("html_units {}: {error}", name.as_str()))?
                    {
                        continue;
                    }
                    if *remaining == 0 {
                        limited = true;
                        break;
                    }
                    selected.push(index);
                    *remaining -= 1;
                }
                matches.insert(
                    name.clone(),
                    NativeMatchedKind::Html(HtmlMatchedKind {
                        outline: outline.clone(),
                        selected,
                        limited,
                    }),
                );
            }
            crate::KindMatcher::MarkdownUnits(selection) => {
                let (Some(document), Some(outline)) = (&markdown, &markdown_outline) else {
                    continue;
                };
                if !matches!(selection, crate::MarkdownUnitSelection::Unit { .. }) {
                    return Err(format!(
                        "markdown_units {}: unit mode required; call_site is unsupported in rulebook kinds",
                        name.as_str()
                    ));
                }
                let (units, limited) = document
                    .select_units(source, selection, *remaining)
                    .map_err(|error| format!("markdown_units {}: {error}", name.as_str()))?;
                *remaining = remaining.saturating_sub(units.len());
                let selected = units
                    .into_iter()
                    .map(|unit| MatchRange {
                        start: unit.span.byte_start,
                        end: unit.span.byte_end,
                    })
                    .collect();
                matches.insert(
                    name.clone(),
                    NativeMatchedKind::Markdown(MarkdownMatchedKind {
                        outline: outline.clone(),
                        selected,
                        limited,
                    }),
                );
            }
            crate::KindMatcher::Named(_) | crate::KindMatcher::Inline { .. } => {}
        }
    }
    Ok(matches)
}
impl MatchedFile {
    pub(super) fn native_unavailable(&self, kind: &EvidenceKindName) -> bool {
        self.native_units
            .get(kind)
            .is_some_and(|value| value.is_limited() && value.match_count() == 0)
    }
    pub(super) fn kind_ranges(
        &self,
        kind: &EvidenceKindName,
        definition: &EvidenceKindDefinition,
    ) -> Vec<MatchRange> {
        match &definition.matcher {
            crate::KindMatcher::HtmlUnits(_) | crate::KindMatcher::MarkdownUnits(_) => self
                .native_units
                .get(kind)
                .map(NativeMatchedKind::ranges)
                .unwrap_or_default(),
            crate::KindMatcher::Named(_) | crate::KindMatcher::Inline { .. } => definition
                .matcher
                .ast_rule_id(kind)
                .and_then(|rule| self.index.by_rule.get(&rule))
                .cloned()
                .unwrap_or_default(),
        }
    }
    pub(super) fn kind_limited(
        &self,
        kind: &EvidenceKindName,
        definition: &EvidenceKindDefinition,
    ) -> bool {
        match &definition.matcher {
            crate::KindMatcher::HtmlUnits(_) | crate::KindMatcher::MarkdownUnits(_) => self
                .native_units
                .get(kind)
                .is_some_and(NativeMatchedKind::is_limited),
            crate::KindMatcher::Named(_) | crate::KindMatcher::Inline { .. } => self.index.limited,
        }
    }
    pub(super) fn section_field(
        &self,
        kind: &EvidenceKindName,
        range: &MatchRange,
        mode: crate::SectionText,
    ) -> FieldValue {
        let Some(value) = self.native_units.get(kind) else {
            return FieldValue::Unavailable {
                reason: FieldUnavailable::Unreadable,
            };
        };
        match value.section_ranges(range, mode) {
            Ok(Some(ranges)) => field_composition::text_for_ranges(self, &ranges),
            Ok(None) => FieldValue::Absent,
            Err(reason) => FieldValue::Unavailable { reason },
        }
    }
    pub(super) fn native_match_count(&self) -> usize {
        self.native_units
            .values()
            .map(NativeMatchedKind::match_count)
            .sum()
    }
}
#[cfg(test)]
#[path = "native_unit_fields_test.rs"]
mod tests;
