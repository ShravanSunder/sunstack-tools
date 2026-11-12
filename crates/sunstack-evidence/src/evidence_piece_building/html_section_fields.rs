use super::*;
use crate::document_evidence::{
    html_content::html_text_content,
    html_outline::{HtmlSection, html_sections},
};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct HtmlMatchedKind {
    outline: Arc<Vec<HtmlSection>>,
    selected: Vec<usize>,
    pub limited: bool,
}

pub(super) fn collect_html_units(
    path: &Path,
    text: &str,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    remaining: &mut usize,
) -> Result<BTreeMap<EvidenceKindName, HtmlMatchedKind>, String> {
    if !is_html_path(path)
        || !kinds
            .values()
            .any(|kind| matches!(kind.matcher, crate::KindMatcher::HtmlUnits(_)))
    {
        return Ok(BTreeMap::new());
    }
    let outline = Arc::new(html_sections(text));
    let mut matches = BTreeMap::new();
    for (name, definition) in kinds {
        let crate::KindMatcher::HtmlUnits(selection) = &definition.matcher else {
            continue;
        };
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
            HtmlMatchedKind {
                outline: outline.clone(),
                selected,
                limited,
            },
        );
    }
    Ok(matches)
}

impl MatchedFile {
    pub(super) fn outline_unavailable(&self, kind: &EvidenceKindName) -> bool {
        self.html_units
            .get(kind)
            .is_some_and(|matches| matches.limited && matches.selected.is_empty())
    }

    pub(super) fn kind_ranges(
        &self,
        kind: &EvidenceKindName,
        definition: &EvidenceKindDefinition,
    ) -> Vec<MatchRange> {
        match &definition.matcher {
            crate::KindMatcher::HtmlUnits(_) => self
                .html_units
                .get(kind)
                .map(|matches| {
                    matches
                        .selected
                        .iter()
                        .filter_map(|index| matches.outline.get(*index))
                        .map(|section| MatchRange {
                            start: section.section_range.start,
                            end: section.section_range.end,
                        })
                        .collect()
                })
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
            crate::KindMatcher::HtmlUnits(_) => self
                .html_units
                .get(kind)
                .is_some_and(|matches| matches.limited),
            crate::KindMatcher::Named(_) | crate::KindMatcher::Inline { .. } => self.index.limited,
        }
    }

    pub(super) fn section_field(
        &self,
        kind: &EvidenceKindName,
        range: &MatchRange,
        mode: crate::SectionText,
    ) -> FieldValue {
        let Some(matches) = self.html_units.get(kind) else {
            return FieldValue::Unavailable {
                reason: FieldUnavailable::Unreadable,
            };
        };
        let Some(section) = matches.outline.iter().find(|section| {
            section.section_range.start == range.start && section.section_range.end == range.end
        }) else {
            return FieldValue::Unavailable {
                reason: FieldUnavailable::Unreadable,
            };
        };
        let selected = match mode {
            crate::SectionText::Full => section.section_range.clone(),
            crate::SectionText::Own => section.own_range.clone(),
            crate::SectionText::EnclosingOwn => {
                let Some(parent) = section
                    .enclosing_index
                    .and_then(|index| matches.outline.get(index))
                else {
                    return FieldValue::Absent;
                };
                parent.own_range.clone()
            }
        };
        let range = MatchRange {
            start: selected.start,
            end: selected.end,
        };
        FieldValue::Text {
            value: self.text[range.start..range.end].into(),
            spans: vec![source_span(self, &range)],
        }
    }

    pub(super) fn outline_match_count(&self) -> usize {
        self.html_units
            .values()
            .map(|matches| matches.selected.len())
            .sum()
    }
}
