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
    visible: crate::document_evidence::markdown_mapped_text::MappedMarkdownText,
}

impl Candidate {
    fn selected(
        &self,
        source: &SourceRef,
        text: &str,
        occurrence: MarkdownOccurrence,
    ) -> SelectedMarkdownUnit {
        SelectedMarkdownUnit {
            kind: self.kind,
            span: source_span(source, text, self.range.clone()),
            text: text[self.range.clone()].into(),
            visible_text: self.visible.text.clone(),
            occurrence,
        }
    }
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
        let normalized =
            crate::document_evidence::markdown_offset_projection::NormalizedMarkdown::new(text);
        let projection =
            crate::document_evidence::markdown_offset_projection::OffsetProjection::new(
                &normalized,
            );
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
        let mut emitted = std::collections::BTreeSet::new();
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
            if matches!(selection, MarkdownUnitSelection::Unit { .. }) {
                if filter.is_none() || visible_ranges.next().is_some() {
                    selected.push(candidate.selected(
                        source,
                        text,
                        MarkdownOccurrence::Unit { span },
                    ));
                }
            } else {
                for visible_range in visible_ranges {
                    let Some(raw) = candidate.visible.raw_range(visible_range.clone()) else {
                        return Err(EvidenceCode::InvalidSelector.into());
                    };
                    let owners = candidates
                        .iter()
                        .enumerate()
                        .filter(|(_, owner)| contains_range(&owner.range, &raw))
                        .filter_map(|(position, owner)| {
                            let range = if position == index {
                                Some(visible_range.clone())
                            } else {
                                owner
                                    .visible
                                    .corresponding_range(&candidate.visible, visible_range.clone())
                            }?;
                            Some((position, owner, range))
                        })
                        .collect::<Vec<_>>();
                    let owner = owners
                        .iter()
                        .filter(|(_, owner, _)| {
                            !(owner.kind == MarkdownUnitKind::Paragraph
                                && owners.iter().any(|(_, item, _)| {
                                    item.kind == MarkdownUnitKind::ListItem
                                        && contains_range(&item.range, &owner.range)
                                }))
                        })
                        .min_by_key(|(position, owner, _)| (owner.range.len(), *position));
                    if let Some((position, owner, owner_visible)) = owner
                        && emitted.insert((*position, owner_visible.start, owner_visible.end))
                    {
                        selected.push(owner.selected(
                            source,
                            text,
                            MarkdownOccurrence::CallSite {
                                unit_span: source_span(source, text, owner.range.clone()),
                                match_span: source_span(source, text, raw),
                                visible_range: MarkdownByteRange {
                                    start: owner_visible.start,
                                    end: owner_visible.end,
                                },
                            },
                        ));
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
        if matches!(selection, MarkdownUnitSelection::CallSite { .. }) {
            selected.sort_by_key(|unit| match &unit.occurrence {
                MarkdownOccurrence::CallSite {
                    match_span,
                    visible_range,
                    ..
                } => (
                    unit.span.byte_start,
                    match_span.byte_start,
                    visible_range.start,
                ),
                _ => (unit.span.byte_start, 0, 0),
            });
        }
        Ok((selected, limited))
    }
}
