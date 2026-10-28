//! Relatedness labels derived from captured input spans and native graph facts.
use crate::{RelatedBasis, RelatednessLabel, RelationshipFactRef, UncertaintyReason};
use sunstack_evidence::{
    CandidateContext, ContextUnit, PieceId, RelatedPiece, RelationshipUncertainty, SourceSpan,
    SyntaxQuality,
};

pub(super) fn input_label(context: &CandidateContext, span: &SourceSpan) -> RelatednessLabel {
    let containing = context
        .units
        .iter()
        .filter(|unit| contains(&unit.unit.span, span))
        .min_by_key(|unit| unit.unit.key.byte_end - unit.unit.key.byte_start);
    let basis = containing.map_or_else(
        || RelatedBasis::InputSpan { span: span.clone() },
        |unit| RelatedBasis::InputUnit {
            unit: unit.unit.key.clone(),
        },
    );
    RelatednessLabel::Related { basis }
}
pub(super) fn contains(parent: &SourceSpan, child: &SourceSpan) -> bool {
    parent.source == child.source
        && parent.byte_start <= child.byte_start
        && child.byte_end <= parent.byte_end
}
fn overlaps(left: &SourceSpan, right: &SourceSpan) -> bool {
    left.source == right.source
        && left.byte_start < right.byte_end
        && right.byte_start < left.byte_end
}
pub(super) fn edge_id(piece: &RelatedPiece) -> &PieceId {
    match piece {
        RelatedPiece::Resolved { piece_id, .. } | RelatedPiece::Unresolved { piece_id, .. } => {
            piece_id
        }
    }
}
fn uncertain(reason: RelationshipUncertainty) -> RelatednessLabel {
    RelatednessLabel::Uncertain {
        reasons: vec![UncertaintyReason::Graph { reason }],
    }
}
pub(super) fn edge_label(context: &CandidateContext, piece: &RelatedPiece) -> RelatednessLabel {
    match piece {
        RelatedPiece::Unresolved { reason, .. } => uncertain(*reason),
        RelatedPiece::Resolved {
            piece_id,
            unit,
            facts,
            uncertainties,
            contradictions,
            ..
        } => {
            let mut reasons = uncertainties
                .iter()
                .filter(|reason| **reason != RelationshipUncertainty::SyntaxOnly)
                .map(|reason| UncertaintyReason::Graph { reason: *reason })
                .collect::<Vec<_>>();
            reasons.extend(
                contradictions
                    .iter()
                    .map(|reason| UncertaintyReason::Contradicted { reason: *reason }),
            );
            if context.units.iter().any(|candidate| {
                candidate.unit.key == *unit
                    && candidate.unit.syntax_quality == SyntaxQuality::RecoveredWithErrors
            }) && !uncertainties.contains(&RelationshipUncertainty::RecoveredSyntax)
            {
                reasons.push(UncertaintyReason::Graph {
                    reason: RelationshipUncertainty::RecoveredSyntax,
                });
            }
            if facts.is_empty() {
                reasons.push(UncertaintyReason::Graph {
                    reason: RelationshipUncertainty::MissingSource,
                });
            }
            if reasons.is_empty() {
                RelatednessLabel::ProbablyRelated {
                    fact: RelationshipFactRef {
                        piece_id: piece_id.clone(),
                        position: 0,
                    },
                }
            } else {
                RelatednessLabel::Uncertain { reasons }
            }
        }
    }
}
pub(super) fn unit_label(context: &CandidateContext, unit: &ContextUnit) -> RelatednessLabel {
    if contains(&unit.unit.span, &context.matched.span)
        || context
            .changes
            .iter()
            .flat_map(|hunk| [hunk.before.as_ref(), hunk.after.as_ref()])
            .flatten()
            .any(|change| overlaps(&unit.unit.span, &change.span))
    {
        return RelatednessLabel::Related {
            basis: RelatedBasis::InputUnit {
                unit: unit.unit.key.clone(),
            },
        };
    }
    let edges = context
        .related
        .iter()
        .filter(|piece| {
            matches!(piece,
        RelatedPiece::Resolved { unit: key, .. } if *key == unit.unit.key)
        })
        .collect::<Vec<_>>();
    let mut probable = None;
    let mut reasons = Vec::new();
    for edge in edges {
        match edge_label(context, edge) {
            RelatednessLabel::ProbablyRelated { fact } => {
                probable.get_or_insert(fact);
            }
            RelatednessLabel::Uncertain {
                reasons: edge_reasons,
            } => reasons.extend(edge_reasons),
            RelatednessLabel::Related { .. } | RelatednessLabel::Unrelated { .. } => {}
        }
    }
    if !reasons.is_empty() {
        RelatednessLabel::Uncertain { reasons }
    } else if let Some(fact) = probable {
        RelatednessLabel::ProbablyRelated { fact }
    } else {
        uncertain(RelationshipUncertainty::MissingSource)
    }
}
