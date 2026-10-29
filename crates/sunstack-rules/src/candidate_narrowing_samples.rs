use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use sunstack_evidence::{
    ContextPart, FrontmatterFieldLocation, MarkdownOccurrence, OccurrenceId, PrivacyProjection,
    SourceOccurrence, SourcePosition, SourceRef, SourceRevision, SourceSide, SourceSpan,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum CandidateNarrowingSourceOrder {
    Record {
        source_ordinal: usize,
        positions: Vec<(u64, u64)>,
        matched_at: (u64, u64),
        field: String,
        span_start: usize,
        span_end: usize,
        identity_tie_breaker: String,
    },
    Repository {
        path: PathBuf,
        byte_start: usize,
        byte_end: usize,
        side: Option<SourceSide>,
        part: Option<ContextPart>,
        enclosing_span: Option<(usize, usize)>,
        identity_tie_breaker: String,
    },
}

pub(crate) struct CandidateNarrowingSampleView {
    pub(crate) input_position: usize,
    source_order: CandidateNarrowingSourceOrder,
    pub(crate) excerpt: String,
    pub(crate) privacy: PrivacyProjection,
}

impl CandidateNarrowingSampleView {
    pub(crate) fn new(
        input_position: usize,
        occurrence: &SourceOccurrence,
        record_ordinal: Option<usize>,
        occurrence_identity: &OccurrenceId,
        excerpt: String,
        privacy: PrivacyProjection,
    ) -> Option<Self> {
        let source_order =
            candidate_source_order(occurrence, record_ordinal, occurrence_identity.as_str())?;
        Some(Self {
            input_position,
            source_order,
            excerpt,
            privacy,
        })
    }
}

pub(crate) struct NarrowingDropCandidate {
    rule: RuleRef,
    occurrence: SourceOccurrence,
    view: CandidateNarrowingSampleView,
}

pub(crate) struct SelectedNarrowingSample {
    pub(crate) sample: DroppedSample,
    pub(crate) privacy: PrivacyProjection,
}

pub(crate) fn clear_narrowing_drop(
    entry: &CandidateEntry,
    view: CandidateNarrowingSampleView,
) -> Option<NarrowingDropCandidate> {
    if !matches!(
        &entry.outcome,
        CandidateOutcome::Dropped {
            at: QuestionRef::Narrowing {},
            ..
        }
    ) {
        return None;
    }
    Some(NarrowingDropCandidate {
        rule: entry.rule.clone(),
        occurrence: entry.occurrence.clone(),
        view,
    })
}

pub(crate) fn select_narrowing_samples(
    candidates: Vec<NarrowingDropCandidate>,
    samples_per_pass: usize,
) -> Vec<SelectedNarrowingSample> {
    let mut buckets = BTreeMap::<(RuleRef, usize), Vec<NarrowingDropCandidate>>::new();
    for candidate in candidates {
        buckets
            .entry((candidate.rule.clone(), candidate.view.input_position))
            .or_default()
            .push(candidate);
    }

    let mut selected = Vec::new();
    for ((_rule, _input_position), mut bucket) in buckets {
        bucket.sort_by(|left, right| left.view.source_order.cmp(&right.view.source_order));
        let selected_positions =
            select_evenly_spaced_sample_indices(bucket.len(), samples_per_pass)
                .into_iter()
                .collect::<BTreeSet<_>>();
        for (position, candidate) in bucket.into_iter().enumerate() {
            if !selected_positions.contains(&position) {
                continue;
            }
            selected.push(SelectedNarrowingSample {
                sample: DroppedSample {
                    rule: candidate.rule,
                    pass: SamplePass::Narrowing,
                    occurrence: candidate.occurrence,
                    excerpt: candidate.view.excerpt,
                },
                privacy: candidate.view.privacy,
            });
        }
    }
    selected
}

fn select_evenly_spaced_sample_indices(item_count: usize, limit: usize) -> Vec<usize> {
    let sample_count = item_count.min(limit);
    (0..sample_count)
        .map(|sample_position| sample_position * item_count / sample_count)
        .collect()
}

fn candidate_source_order(
    occurrence: &SourceOccurrence,
    record_ordinal: Option<usize>,
    identity_tie_breaker: &str,
) -> Option<CandidateNarrowingSourceOrder> {
    match occurrence {
        SourceOccurrence::Record {
            positions,
            matched_at,
            field,
            span,
            ..
        } => Some(CandidateNarrowingSourceOrder::Record {
            source_ordinal: record_ordinal?,
            positions: positions.iter().map(source_position_order).collect(),
            matched_at: source_position_order(matched_at),
            field: field.as_str().to_owned(),
            span_start: span.byte_start,
            span_end: span.byte_end,
            identity_tie_breaker: identity_tie_breaker.to_owned(),
        }),
        SourceOccurrence::DocumentUnit { occurrence, .. } => {
            let (span, enclosing_span) = match occurrence {
                MarkdownOccurrence::Unit { span } => (span, None),
                MarkdownOccurrence::CallSite {
                    unit_span,
                    match_span,
                    ..
                } => (
                    unit_span,
                    Some((match_span.byte_start, match_span.byte_end)),
                ),
            };
            repository_source_order(span, None, enclosing_span, identity_tie_breaker)
        }
        SourceOccurrence::DocumentField { source, state, .. } => {
            let (byte_start, byte_end) = match state.as_ref() {
                FrontmatterFieldLocation::Present { span }
                | FrontmatterFieldLocation::Absent {
                    frontmatter_span: span,
                } => (span.byte_start, span.byte_end),
                FrontmatterFieldLocation::BlockUnavailable { .. } => (0, 0),
            };
            repository_source_order_parts(
                source,
                byte_start,
                byte_end,
                None,
                None,
                identity_tie_breaker,
            )
        }
        SourceOccurrence::ContextMatch {
            part, unit, span, ..
        } => repository_source_order(
            span,
            Some(*part),
            unit.as_ref().map(|unit| (unit.byte_start, unit.byte_end)),
            identity_tie_breaker,
        ),
        SourceOccurrence::CodeMatch { unit, span, .. } => repository_source_order(
            span,
            None,
            unit.as_ref().map(|unit| (unit.byte_start, unit.byte_end)),
            identity_tie_breaker,
        ),
    }
}

fn source_position_order(position: &SourcePosition) -> (u64, u64) {
    (position.byte_offset, position.line)
}

fn repository_source_order(
    span: &SourceSpan,
    part: Option<ContextPart>,
    detail_span: Option<(usize, usize)>,
    identity_tie_breaker: &str,
) -> Option<CandidateNarrowingSourceOrder> {
    repository_source_order_parts(
        &span.source,
        span.byte_start,
        span.byte_end,
        part,
        detail_span,
        identity_tie_breaker,
    )
}

fn repository_source_order_parts(
    source: &SourceRef,
    byte_start: usize,
    byte_end: usize,
    part: Option<ContextPart>,
    detail_span: Option<(usize, usize)>,
    identity_tie_breaker: &str,
) -> Option<CandidateNarrowingSourceOrder> {
    let path = match source {
        SourceRef::RepositoryFile { path, .. }
        | SourceRef::Document { path, .. }
        | SourceRef::Transcript { path, .. } => path.clone(),
        SourceRef::Payload { .. } => return None,
    };
    Some(CandidateNarrowingSourceOrder::Repository {
        path,
        byte_start,
        byte_end,
        side: source_side(source),
        part,
        enclosing_span: detail_span,
        identity_tie_breaker: identity_tie_breaker.to_owned(),
    })
}

fn source_side(source: &SourceRef) -> Option<SourceSide> {
    let SourceRef::RepositoryFile { revision, .. } = source else {
        return None;
    };
    Some(match revision {
        SourceRevision::Commit { side, .. }
        | SourceRevision::Index { side, .. }
        | SourceRevision::Worktree { side, .. } => *side,
    })
}

#[cfg(test)]
#[path = "candidate_narrowing_sample_tests.rs"]
mod candidate_narrowing_sample_tests;
