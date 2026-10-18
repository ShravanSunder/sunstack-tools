use super::*;
use crate::{
    EvidenceCode, EvidenceResult, MarkdownByteRange, MarkdownOccurrence, MarkdownUnitFilter,
    MarkdownUnitKind, MarkdownUnitSelection, SelectedMarkdownUnit, SourceRef, source_span,
};

impl MarkdownUnitSelection {
    pub(crate) fn validate(&self) -> EvidenceResult<()> {
        let (units, filter) = self.parts();
        if units.is_empty() {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        if let Some(filter) = filter {
            match filter {
                MarkdownUnitFilter::StartsWith { starts_with } if starts_with.is_empty() => {
                    return Err(EvidenceCode::InvalidSelector.into());
                }
                MarkdownUnitFilter::Regex { pattern } => {
                    regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?;
                    let hir = regex_syntax::Parser::new()
                        .parse(pattern)
                        .map_err(|_| EvidenceCode::InvalidSelector)?;
                    if hir.properties().minimum_len() == Some(0) {
                        return Err(EvidenceCode::InvalidSelector.into());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn parts(&self) -> (&[MarkdownUnitKind], Option<&MarkdownUnitFilter>) {
        match self {
            Self::Unit { units, filter } => (units, filter.as_ref()),
            Self::CallSite { units, filter } => (units, Some(filter)),
        }
    }
}

struct Candidate {
    kind: MarkdownUnitKind,
    range: Range<usize>,
    visible: crate::markdown_mapped_text::MappedMarkdownText,
}

impl MarkdownDocument {
    pub(crate) fn select_units(
        &self,
        source: &SourceRef,
        selection: &MarkdownUnitSelection,
        max_matches: usize,
    ) -> EvidenceResult<(Vec<SelectedMarkdownUnit>, bool)> {
        selection.validate()?;
        let (kinds, filter) = selection.parts();
        let text = self.tree.source();
        let normalized = crate::markdown_offset_projection::NormalizedMarkdown::new(text);
        let projection = crate::markdown_offset_projection::OffsetProjection::new(&normalized);
        let candidates = self
            .tree
            .root()
            .dfs()
            .filter_map(|node| {
                let kind = match node.kind().as_ref() {
                    "atx_heading" | "setext_heading" => MarkdownUnitKind::Section,
                    "paragraph" => MarkdownUnitKind::Paragraph,
                    "list_item" => MarkdownUnitKind::ListItem,
                    _ => return None,
                };
                if !kinds.contains(&kind) {
                    return None;
                }
                let range = if kind == MarkdownUnitKind::Section {
                    self.section_range(&node)
                } else {
                    node.range()
                };
                let (_, visible) = self.project_with_offsets(range.clone(), &projection);
                Some(Candidate {
                    kind,
                    range,
                    visible,
                })
            })
            .collect::<Vec<_>>();
        let regex = match filter {
            Some(MarkdownUnitFilter::Regex { pattern }) => {
                Some(regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?)
            }
            _ => None,
        };
        let mut selected = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            let mut visible_ranges: Box<dyn Iterator<Item = Range<usize>> + '_> = match filter {
                None => Box::new(std::iter::empty()),
                Some(MarkdownUnitFilter::StartsWith { starts_with }) => Box::new(
                    candidate
                        .visible
                        .text
                        .starts_with(starts_with)
                        .then_some(0..starts_with.len())
                        .into_iter(),
                ),
                Some(MarkdownUnitFilter::Regex { .. }) => Box::new(
                    regex
                        .as_ref()
                        .into_iter()
                        .flat_map(|matcher| matcher.find_iter(&candidate.visible.text))
                        .map(|found| found.range()),
                ),
            };
            let span = source_span(source, text, candidate.range.clone());
            let make_unit = |occurrence| SelectedMarkdownUnit {
                kind: candidate.kind,
                span: span.clone(),
                text: text[candidate.range.clone()].into(),
                visible_text: candidate.visible.text.clone(),
                occurrence,
            };
            if matches!(selection, MarkdownUnitSelection::Unit { .. }) {
                if filter.is_none() || visible_ranges.next().is_some() {
                    selected.push(make_unit(MarkdownOccurrence::Unit { span: span.clone() }));
                }
            } else {
                for visible_range in visible_ranges {
                    let Some(raw) = candidate.visible.raw_range(visible_range.clone()) else {
                        return Err(EvidenceCode::InvalidSelector.into());
                    };
                    let owner = candidates
                        .iter()
                        .enumerate()
                        .filter(|(_, owner)| contains_range(&owner.range, &raw))
                        .filter(|(_, owner)| {
                            !(owner.kind == MarkdownUnitKind::Paragraph
                                && candidates.iter().any(|item| {
                                    item.kind == MarkdownUnitKind::ListItem
                                        && contains_range(&item.range, &owner.range)
                                }))
                        })
                        .min_by_key(|(position, owner)| (owner.range.len(), *position))
                        .map(|(position, _)| position);
                    if owner == Some(index) {
                        selected.push(make_unit(MarkdownOccurrence::CallSite {
                            unit_span: span.clone(),
                            match_span: source_span(source, text, raw),
                            visible_range: MarkdownByteRange {
                                start: visible_range.start,
                                end: visible_range.end,
                            },
                        }));
                    }
                    if selected.len() > max_matches {
                        break;
                    }
                }
            }
            if selected.len() > max_matches {
                break;
            }
        }
        let limited = selected.len() > max_matches;
        selected.truncate(max_matches);
        Ok((selected, limited))
    }
}
