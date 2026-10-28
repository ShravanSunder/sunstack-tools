//! Pure relatedness projection and whole-unit context packing.
use crate::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{
    CandidateContext, CodeExcerpt, CodeRedactionRange, CodeTextPart, ContextUnit, PieceId,
    RelatedPiece, RelationshipUncertainty, SourceSide, SourceUnitKey, SourceUnitKind,
    content_digest,
};

#[path = "context_label_projection.rs"]
mod context_label_projection;
#[path = "context_page_rendering.rs"]
mod context_page_rendering;
#[path = "context_unit_truncation.rs"]
mod context_unit_truncation;
use context_label_projection::{contains, edge_id, edge_label, input_label, unit_label};
use context_page_rendering::{PieceNames, label, part, relation_anchor, revision, source_anchor};
use context_unit_truncation::{shrink_unit, signature_header};

pub const CONTEXT_PAGE_CHARACTER_LIMIT: usize = 20_000;

struct PagePiece<'context> {
    piece_id: PieceId,
    part: CodeTextPart,
    label: RelatednessLabel,
    anchor: String,
    body: String,
    unit: Option<&'context ContextUnit>,
    omitted_lines: Option<OmittedSourceLines>,
    never_found: Option<RelationshipUncertainty>,
    excluded: Option<ContextExclusionReason>,
    matched: Option<MatchAnchor<'context>>,
}
struct MatchAnchor<'context> {
    piece_id: PieceId,
    label: RelatednessLabel,
    excerpt: &'context CodeExcerpt,
}
impl MatchAnchor<'_> {
    fn coverage(
        &self,
        page: usize,
        omitted: Option<&OmittedSourceLines>,
        ranges: &[CodeRedactionRange],
    ) -> ContextPieceCoverage {
        let span = &self.excerpt.span;
        let last = span.end_line.saturating_sub(usize::from(
            span.end_column == 1 && span.end_line > span.start_line,
        ));
        let intersection = omitted.and_then(|omitted| {
            let first = span.start_line.max(omitted.first);
            let last = last.min(omitted.last);
            (first <= last).then_some(OmittedSourceLines { first, last })
        });
        let disposition =
            intersection.map_or(ContextPieceDisposition::Placed { page }, |omitted_lines| {
                ContextPieceDisposition::Truncated {
                    page,
                    omitted_lines,
                }
            });
        let redacted = ranges
            .iter()
            .filter(|range| range.part == CodeTextPart::Match)
            .filter(|range| {
                let prefix = self
                    .excerpt
                    .text
                    .as_bytes()
                    .get(..range.byte_start)
                    .unwrap_or_default();
                let first = span.start_line + prefix.iter().filter(|byte| **byte == b'\n').count();
                let ending = self
                    .excerpt
                    .text
                    .as_bytes()
                    .get(..range.byte_end)
                    .unwrap_or_default();
                let last = span.start_line + ending.iter().filter(|byte| **byte == b'\n').count();
                omitted.is_none_or(|omitted| last < omitted.first || first > omitted.last)
            })
            .count();
        ContextPieceCoverage {
            piece_id: self.piece_id.clone(),
            disposition,
            redacted,
        }
    }
}
impl PagePiece<'_> {
    fn ids(&self) -> impl Iterator<Item = &PieceId> {
        std::iter::once(&self.piece_id).chain(self.matched.iter().map(|matched| &matched.piece_id))
    }
    fn labels(&self) -> impl Iterator<Item = (&PieceId, &RelatednessLabel)> {
        std::iter::once((&self.piece_id, &self.label)).chain(
            self.matched
                .iter()
                .map(|matched| (&matched.piece_id, &matched.label)),
        )
    }
    fn coverage_entries(
        &self,
        page: usize,
        ranges: &[CodeRedactionRange],
    ) -> Vec<ContextPieceCoverage> {
        let mut entries = vec![self.coverage(page, ranges)];
        if let Some(matched) = &self.matched {
            entries.push(matched.coverage(page, self.omitted_lines.as_ref(), ranges));
        }
        entries
    }
    fn coverage(&self, page: usize, ranges: &[CodeRedactionRange]) -> ContextPieceCoverage {
        let disposition = if let Some(reason) = &self.excluded {
            ContextPieceDisposition::Excluded {
                reason: reason.clone(),
            }
        } else if let Some(reason) = self.never_found {
            ContextPieceDisposition::NeverFound { reason }
        } else if let Some(omitted_lines) = &self.omitted_lines {
            ContextPieceDisposition::Truncated {
                page,
                omitted_lines: omitted_lines.clone(),
            }
        } else {
            ContextPieceDisposition::Placed { page }
        };
        ContextPieceCoverage {
            piece_id: self.piece_id.clone(),
            disposition,
            redacted: self.redacted(ranges),
        }
    }
}

/// The callback measures the complete canonical payload after request materialization.
/// Returned page text is reused unchanged by every consumer.
pub fn project_context_pages(
    context: &CandidateContext,
    settings: &ContextSettings,
    redaction_ranges: &[CodeRedactionRange],
    mut rendered_cost: impl FnMut(&ContextPage) -> Result<usize, DiagnosticList>,
) -> Result<ContextProjection, DiagnosticList> {
    let mut pieces = context_pieces(context)?;
    let mut groups = vec![(0..pieces.len()).collect::<Vec<_>>()];
    loop {
        let mut pages = Vec::new();
        let mut changed = false;
        for position in 0..groups.len() {
            let mut page = render_page(context, &pieces, &groups, position, redaction_ranges)?;
            page.rendered_chars = rendered_cost(&page)?;
            if page.rendered_chars > CONTEXT_PAGE_CHARACTER_LIMIT {
                if groups[position].len() > 1 {
                    // Move an ordered suffix, preserving indivisible pieces and source units.
                    let midpoint = groups[position].len() / 2;
                    let suffix = groups[position].split_off(midpoint);
                    groups.insert(position + 1, suffix);
                } else {
                    let index = *groups[position].first().ok_or_else(invalid_context)?;
                    if let Err(diagnostics) = shrink_unit(&mut pieces[index], settings) {
                        if pieces[index].unit.is_none() {
                            return Err(diagnostics);
                        }
                        // A real unit exhausts shrinking only at its mandatory minimum.
                        // The current page was measured with that complete rendering/header.
                        return Ok(over_page_budget_projection(
                            &pieces,
                            &pieces[index].piece_id,
                            page.rendered_chars,
                        ));
                    }
                    // Truncation changes the complete packing problem. Start
                    // again so earlier splits cannot create artificial pages.
                    groups = vec![(0..pieces.len()).collect()];
                }
                changed = true;
                break;
            }
            pages.push(page);
        }
        if !changed {
            let labels = pieces
                .iter()
                .flat_map(|piece| piece.labels())
                .map(|(piece_id, label)| (piece_id.clone(), label.clone()))
                .collect();
            let mut coverage = Vec::new();
            for (position, group) in groups.iter().enumerate() {
                coverage.extend(group.iter().flat_map(|index| {
                    pieces[*index].coverage_entries(position + 1, redaction_ranges)
                }));
            }
            return Ok(ContextProjection {
                labels,
                pages,
                coverage,
            });
        }
    }
}

fn over_page_budget_projection(
    pieces: &[PagePiece<'_>],
    unit: &PieceId,
    required_chars: usize,
) -> ContextProjection {
    let reason = ContextExclusionReason::OverPageBudget {
        unit: unit.clone(),
        required_chars: u32::try_from(required_chars).unwrap_or(u32::MAX),
    };
    ContextProjection {
        labels: pieces
            .iter()
            .flat_map(|piece| piece.labels())
            .map(|(id, label)| (id.clone(), label.clone()))
            .collect(),
        pages: vec![],
        coverage: pieces
            .iter()
            .flat_map(|piece| piece.ids())
            .map(|id| ContextPieceCoverage {
                piece_id: id.clone(),
                disposition: ContextPieceDisposition::Excluded {
                    reason: reason.clone(),
                },
                redacted: 0,
            })
            .collect(),
    }
}

fn invalid_context() -> DiagnosticList {
    DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration)
}
fn over_budget() -> DiagnosticList {
    DiagnosticList::rule(RuleDiagnosticCode::RuleExceedsBudget)
}
fn json_text(value: &impl Serialize) -> Result<String, DiagnosticList> {
    serde_json::to_string(value).map_err(|_| invalid_context())
}
fn helper_id(value: String) -> Result<PieceId, DiagnosticList> {
    PieceId::new(value).map_err(|error| error.diagnostics().into())
}
fn excerpt_piece<'context>(
    context: &CandidateContext,
    piece_id: PieceId,
    part: CodeTextPart,
    excerpt: &CodeExcerpt,
) -> Result<PagePiece<'context>, DiagnosticList> {
    Ok(PagePiece {
        piece_id,
        part,
        label: input_label(context, &excerpt.span),
        anchor: source_anchor(&excerpt.span),
        body: excerpt.text.clone(),
        unit: None,
        omitted_lines: None,
        never_found: None,
        excluded: None,
        matched: None,
    })
}
fn context_pieces(context: &CandidateContext) -> Result<Vec<PagePiece<'_>>, DiagnosticList> {
    let mut pieces = Vec::new();
    for hunk in &context.changes {
        for (side, excerpt) in [("before", &hunk.before), ("after", &hunk.after)] {
            if let Some(excerpt) = excerpt {
                pieces.push(excerpt_piece(
                    context,
                    helper_id(format!("hunk:{}:{side}", hunk.hunk_id.as_str()))?,
                    CodeTextPart::Hunk {
                        hunk_id: hunk.hunk_id.clone(),
                        side: if side == "before" {
                            SourceSide::Before
                        } else {
                            SourceSide::After
                        },
                    },
                    excerpt,
                )?);
            }
        }
    }
    let match_unit = context
        .units
        .iter()
        .filter(|unit| contains(&unit.unit.span, &context.matched.span))
        .min_by_key(|unit| {
            (
                unit.unit.key.byte_end - unit.unit.key.byte_start,
                unit.piece_id.clone(),
            )
        })
        .map(|unit| &unit.unit.key);
    let match_id = helper_id(format!("match:{}", context.context_id.as_str()))?;
    if match_unit.is_none() {
        pieces.push(excerpt_piece(
            context,
            match_id.clone(),
            CodeTextPart::Match,
            &context.matched,
        )?);
    }
    let mut ordered_units = context.units.iter().collect::<Vec<_>>();
    ordered_units.sort_by_key(|unit| {
        let priority = if matches!(unit_label(context, unit), RelatednessLabel::Related { .. }) {
            0
        } else {
            1
        };
        let nearest = context
            .related
            .iter()
            .position(|piece| {
                matches!(piece,
            RelatedPiece::Resolved { unit: key, .. } if *key == unit.unit.key)
            })
            .unwrap_or(usize::MAX);
        (
            priority,
            nearest,
            unit.unit.key.clone(),
            unit.piece_id.clone(),
        )
    });
    let mut retained_units = BTreeMap::<SourceUnitKey, PieceId>::new();
    for unit in ordered_units {
        let excluded = retained_units.get(&unit.unit.key).map(|retained| {
            ContextExclusionReason::DuplicateUnit {
                retained: retained.clone(),
            }
        });
        retained_units
            .entry(unit.unit.key.clone())
            .or_insert_with(|| unit.piece_id.clone());
        pieces.push(PagePiece {
            piece_id: unit.piece_id.clone(),
            part: CodeTextPart::UnitText {
                piece_id: unit.piece_id.clone(),
            },
            label: unit_label(context, unit),
            anchor: source_anchor(&unit.unit.span),
            body: if excluded.is_some() {
                String::new()
            } else {
                format!("{}{}", signature_header(unit), unit.text)
            },
            unit: if excluded.is_some() { None } else { Some(unit) },
            matched: if excluded.is_none() && match_unit == Some(&unit.unit.key) {
                Some(MatchAnchor {
                    piece_id: match_id.clone(),
                    label: input_label(context, &context.matched.span),
                    excerpt: &context.matched,
                })
            } else {
                None
            },
            omitted_lines: None,
            never_found: None,
            excluded,
        });
    }
    let mut related = context.related.iter().collect::<Vec<_>>();
    related.sort_by_key(|piece| edge_id(piece).clone());
    for relation in related {
        // Native facts, call sites, hints and qualifications are all part of the counted payload.
        let never_found = match relation {
            RelatedPiece::Unresolved { reason, .. } => Some(*reason),
            RelatedPiece::Resolved { .. } => None,
        };
        pieces.push(PagePiece {
            piece_id: edge_id(relation).clone(),
            part: CodeTextPart::Hint {
                piece_id: edge_id(relation).clone(),
            },
            label: edge_label(context, relation),
            anchor: relation_anchor(context, relation)?,
            body: String::new(),
            unit: None,
            omitted_lines: None,
            never_found,
            excluded: None,
            matched: None,
        });
    }
    for (position, excerpt) in context.imports.iter().enumerate() {
        pieces.push(excerpt_piece(
            context,
            helper_id(format!(
                "import:{}:{}",
                context.context_id.as_str(),
                content_digest(json_text(&excerpt.span)?.as_bytes())
            ))?,
            CodeTextPart::Import { position },
            excerpt,
        )?);
    }
    let mut identities = BTreeSet::new();
    if pieces
        .iter()
        .flat_map(|piece| piece.ids())
        .any(|piece_id| !identities.insert(piece_id.clone()))
    {
        return Err(invalid_context());
    }
    Ok(pieces)
}
fn render_page(
    context: &CandidateContext,
    pieces: &[PagePiece<'_>],
    groups: &[Vec<usize>],
    position: usize,
    redaction_ranges: &[CodeRedactionRange],
) -> Result<ContextPage, DiagnosticList> {
    let holds = groups[position]
        .iter()
        .flat_map(|index| pieces[*index].ids())
        .cloned()
        .collect::<Vec<_>>();
    let names = PieceNames::new(pieces.iter().flat_map(|piece| piece.ids()));
    let other_pages = groups
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != position)
        .map(|(other, group)| {
            format!(
                "page {}: {}",
                other + 1,
                group
                    .iter()
                    .map(|index| {
                        let piece = &pieces[*index];
                        format!("{} {}", part(&piece.part), names.get(&piece.piece_id))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let revisions = context
        .units
        .iter()
        .map(|unit| revision(&unit.unit.key.revision))
        .chain(match &context.matched.span.source {
            sunstack_evidence::SourceRef::RepositoryFile {
                revision: source, ..
            } => Some(revision(source)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let revision = revisions.into_iter().collect::<Vec<_>>().join(", ");
    let mut text = format!(
        "repository {} revision {}\npage {} of {}\nother pages: {}\n",
        context.repository.as_str(),
        revision,
        position + 1,
        groups.len(),
        if other_pages.is_empty() {
            "none"
        } else {
            &other_pages
        }
    );
    for index in &groups[position] {
        let piece = &pieces[*index];
        text.push_str(&format!(
            "\npiece [{}] {}\nanchor: {}\nlabel: {}\n{}\n",
            names.get(&piece.piece_id),
            part(&piece.part),
            piece.anchor,
            label(context, &piece.label, &names)?,
            piece.body
        ));
        if let Some(matched) = &piece.matched {
            text.push_str(&format!(
                "match anchor [{}] into [{}]\nanchor: {}\nlabel: {}\n",
                names.get(&matched.piece_id),
                names.get(&piece.piece_id),
                source_anchor(&matched.excerpt.span),
                label(context, &matched.label, &names)?
            ));
            if let ContextPieceDisposition::Truncated { omitted_lines, .. } = matched
                .coverage(position + 1, piece.omitted_lines.as_ref(), redaction_ranges)
                .disposition
            {
                text.push_str(&format!(
                    "match source omitted at lines {}-{}\n",
                    omitted_lines.first, omitted_lines.last
                ));
            }
        }
    }
    Ok(ContextPage {
        number: position + 1,
        of: groups.len(),
        revision,
        holds,
        rendered_chars: 0,
        content_identity: content_digest(text.as_bytes()),
        text,
    })
}
